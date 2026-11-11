//! Glass: the one plane that reads the pixels painted beneath it.
//!
//! Every other surface in the kit is opaque — `Tier::*` picks a fill off the
//! grey ladder and that is the whole story. A glass pane is different in kind:
//! it samples the frame *under* it, blurs it, tints it, and puts a lit rim on
//! the result. That needs the renderer to hand the pane what was already drawn,
//! which is what `engine::glass::backdrop` (the vendor-side pass split) exists
//! for. This module is the only caller of that channel.
//!
//! Two consequences the rest of the app must respect:
//!   * A glass pane only reads **layers below its own**. Inside one layer,
//!     whatever was painted first is not in the sample. So glass belongs to a
//!     floating overlay (`Layered::overlay`), never to a strip that sits in the
//!     base layer with content beside it — the same rule Apple draws the
//!     material by.
//!   * The fill is a *legibility* control, not decoration. `glass_tint`'s alpha
//!     is what keeps ink readable when the sample behind it turns into a bright
//!     hatch; the tint is the floor, the blur is the flourish.

use crate::ui::style::tokens;
use bytemuck::{Pod, Zeroable};
use iced::widget::shader::{self, Backdrop, Viewport};
use iced::widget::{Shader, Stack};
use iced::{Color, Element, Length, Rectangle, Theme, wgpu};
use rustc_hash::FxHashMap;
use std::sync::Mutex;

/// The 1px rim, in logical pixels, before the viewport scale is applied.
const RIM_LOGICAL: f32 = 1.0;

/// Per-pane uniforms, mirrored field-for-field by `shaders/glass_pane.wgsl`.
///
/// Field order and padding are load-bearing: the WGSL struct expects
/// `vec2`/`vec4` alignment, and `Pod` will not tolerate Rust-side padding the
/// shader does not declare.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
struct Params {
    plate_origin: [f32; 2],
    plate_size: [f32; 2],
    radius: f32,
    rim: f32,
    frame_size: [f32; 2],
    saturate: f32,
    pad: [f32; 3],
    tint: [f32; 4],
    rim_color: [f32; 4],
    hi_top: [f32; 4],
    hi_bottom: [f32; 4],
}

/// Identity of one pane's uniform slot: where it sits and how sharp its corner
/// is. Prepared and drawn in the same frame, in the same order, so the key never
/// has to outlive it — `trim` drops the whole map at frame end. The corner is
/// part of the key because two panes of different `raised` can share a rectangle.
type SlotKey = (u32, u32, u32, u32, u32);

fn slot_key(bounds: &Rectangle, radius: f32) -> SlotKey {
    (
        bounds.x.to_bits(),
        bounds.y.to_bits(),
        bounds.width.to_bits(),
        bounds.height.to_bits(),
        radius.to_bits(),
    )
}

fn rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a]
}

/// The shared GPU state for every glass pane on screen.
///
/// The device is held because a pane's parameter buffer is minted at draw time,
/// when the rectangle and colours are finally known. The queue is not: the same
/// draw is handed the queue it must write through, so holding a second clone
/// would only risk stamping an old one.
#[derive(Debug)]
pub struct GlassPipeline {
    device: wgpu::Device,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    slots: Mutex<FxHashMap<SlotKey, wgpu::Buffer>>,
}

impl GlassPipeline {
    fn slot(&self, key: SlotKey) -> Option<wgpu::Buffer> {
        self.slots.lock().ok()?.get(&key).cloned()
    }
}

impl shader::Pipeline for GlassPipeline {
    fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("draftflow.glass.layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("draftflow.glass.pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("draftflow.glass.shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "../../shaders/glass_pane.wgsl"
            ))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("draftflow.glass.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // The pane covers what it samples, so it composites with
                    // plain source alpha; only the anti-aliased rim needs the
                    // destination.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("draftflow.glass.sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self {
            device: device.clone(),
            pipeline,
            layout,
            sampler,
            slots: Mutex::new(FxHashMap::default()),
        }
    }

    /// Glass panes are keyed by bounds, and bounds move every time a menu is
    /// re-opened. Dropping the map at frame end keeps the cache to the panes
    /// actually on screen instead of every rectangle ever laid out.
    fn trim(&mut self) {
        if let Ok(mut slots) = self.slots.lock() {
            slots.clear();
        }
    }
}

/// One pane to be painted.
#[derive(Clone, Copy, Debug)]
pub struct GlassPane {
    bounds: Rectangle,
    tint: Color,
    rim: Color,
    hi_top: Color,
    hi_bottom: Color,
    radius: f32,
    /// Entrance fade, applied to the material's own alphas so a pane coming in
    /// does not flash a fully-lit plate over a half-drawn menu.
    fade: f32,
}

impl GlassPane {
    fn params(&self, scale: f32, frame_size: [f32; 2]) -> Params {
        let fade = self.fade.clamp(0.0, 1.0);
        let ramp = |color: Color| {
            let mut out = rgba(color);
            out[3] *= fade;
            out
        };
        Params {
            plate_origin: [self.bounds.x * scale, self.bounds.y * scale],
            plate_size: [self.bounds.width * scale, self.bounds.height * scale],
            radius: self.radius * scale,
            rim: RIM_LOGICAL * scale,
            frame_size,
            saturate: tokens::GLASS_SATURATE,
            pad: [0.0; 3],
            tint: ramp(self.tint),
            rim_color: ramp(self.rim),
            hi_top: ramp(self.hi_top),
            hi_bottom: ramp(self.hi_bottom),
        }
    }
}

impl shader::Primitive for GlassPane {
    type Pipeline = GlassPipeline;

    fn reads_backdrop(&self) -> bool {
        // The whole point of this primitive; it also opens the frame split.
        true
    }

    fn prepare(
        &self,
        pipeline: &mut GlassPipeline,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        let scale = viewport.scale_factor() as f32;
        let frame_size = [
            viewport.physical_width().max(1) as f32,
            viewport.physical_height().max(1) as f32,
        ];
        let params = self.params(scale, frame_size);
        let key = slot_key(&self.bounds, self.radius);
        let buffer = {
            let Ok(mut slots) = pipeline.slots.lock() else {
                return;
            };
            slots
                .entry(key)
                .or_insert_with(|| {
                    pipeline.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("draftflow.glass.params"),
                        size: std::mem::size_of::<Params>() as u64,
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    })
                })
                .clone()
        };
        queue.write_buffer(&buffer, 0, bytemuck::bytes_of(&params));
    }

    fn draw_with_backdrop(
        &self,
        pipeline: &GlassPipeline,
        pass: &mut wgpu::RenderPass<'_>,
        backdrop: &Backdrop<'_>,
    ) -> bool {
        let Some(buffer) = pipeline.slot(slot_key(&self.bounds, self.radius)) else {
            return false;
        };

        let bind_group = pipeline.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("draftflow.glass.bind"),
            layout: &pipeline.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(backdrop.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });

        // The pass already carries this pane's viewport and scissor, so the
        // draw is one triangle that covers the clip rect and the shader decides
        // which of those pixels belong to the rounded plate.
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
        true
    }
}

/// The program side of a glass pane: the four resolved colours and the corner.
/// It publishes no message — a pane is paint, and everything a screen needs to
/// decide about it is already in the `Theme`.
#[derive(Clone, Copy, Debug)]
struct GlassPlate {
    tint: Color,
    rim: Color,
    hi_top: Color,
    hi_bottom: Color,
    radius: f32,
    fade: f32,
}

impl<Message> shader::Program<Message> for GlassPlate {
    type State = ();
    type Primitive = GlassPane;

    fn draw(
        &self,
        _state: &(),
        _cursor: iced::mouse::Cursor,
        bounds: Rectangle,
    ) -> GlassPane {
        GlassPane {
            bounds,
            tint: self.tint,
            rim: self.rim,
            hi_top: self.hi_top,
            hi_bottom: self.hi_bottom,
            radius: self.radius,
            fade: self.fade,
        }
    }
}

/// The L1 glass primitive: `content` resting on a pane that reads the frame
/// under it.
///
/// `raised` picks the 20 px corner for glass that pops from other glass, the
/// 16 px corner for glass floating over the document. `track` is the
/// entrance-fade key, so a pane that is fading in fades its material with its
/// content instead of painting a lit plate over a half-drawn menu.
pub fn glass_pane<'a, Message>(
    content: Element<'a, Message>,
    theme: &Theme,
    raised: bool,
    track: &'static str,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let radius = if raised {
        tokens::RADIUS_GLASS_RAISED
    } else {
        tokens::RADIUS_GLASS
    };
    let (hi_top, hi_bottom) = tokens::glass_hi(theme);
    let plate = GlassPlate {
        tint: tokens::glass_tint(theme),
        rim: tokens::glass_edge(theme),
        hi_top,
        hi_bottom,
        radius,
        fade: crate::ui::motion::value_or(track, 1.0),
    };

    let glass = Shader::new(plate)
        .width(Length::Fill)
        .height(Length::Fill);

    // `push_under` is what keeps the two in the order the channel needs. The
    // first child of a `Stack` is painted inside the overlay's own layer and
    // every later child gets a fresh layer above it, so the pane is the only
    // thing in the layer the split is taken at, and the menu's ink lands over
    // it. A plain `push` would put the glass on top of its own content.
    Stack::new().push_under(glass).push(content).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `Params` struct and the WGSL `Glass` struct are one layout written
    /// twice; a field added on either side shifts every colour after it, so the
    /// mirror is asserted instead of trusted.
    #[test]
    fn params_mirror_the_shader() {
        let shader = include_str!("../../shaders/glass_pane.wgsl");
        let body = shader
            .split("struct Glass {")
            .nth(1)
            .and_then(|rest| rest.split("};").next())
            .expect("`struct Glass` in the glass shader");

        let mut slots = 0usize;
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            slots += match line {
                _ if line.contains("vec4<f32>") => 4,
                _ if line.contains("vec3<f32>") => 3,
                _ if line.contains("vec2<f32>") => 2,
                _ if line.contains(": f32") || line.contains(": u32") => 1,
                other => panic!("unmapped WGSL declaration: {other}"),
            };
        }

        assert_eq!(
            slots,
            std::mem::size_of::<Params>() / 4,
            "the glass uniform block and its shader struct drifted apart"
        );
    }

    /// The frame has to be cut at the layer of the first reader, not at the
    /// reader itself: inside one layer, what was painted earlier never reaches
    /// the sample.
    #[test]
    fn the_channel_splits_at_the_first_reader_layer() {
        let lib = include_str!("../../../vendor/iced/wgpu/src/lib.rs");

        assert!(
            lib.contains("fn backdrop_split"),
            "the backdrop split is gone from the renderer"
        );
        assert!(
            lib.contains("instance.reads_backdrop()"),
            "the renderer no longer asks a primitive for its backdrop"
        );
        assert!(
            lib.contains("0..split"),
            "the layers beneath the first reader are no longer offscreen"
        );
    }

    /// A frame with no opaque ground to read back — a splash, a screenshot —
    /// must keep the single-pass path rather than blur a half-composed image.
    #[test]
    fn a_transparent_frame_keeps_the_single_pass() {
        let lib = include_str!("../../../vendor/iced/wgpu/src/lib.rs");
        let split = lib
            .split("let split = if")
            .nth(1)
            .and_then(|rest| rest.split(';').next())
            .expect("the split decision");

        assert!(
            split.contains("clear_color.is_some()"),
            "the channel would open on a frame it cannot read back"
        );
    }

    /// Merging moves a layer's batches *down*. For a pane that samples what lies
    /// beneath, that silently changes what it samples, so the reader's layer has
    /// to be left alone by `Stack::merge`.
    #[test]
    fn a_reader_layer_is_never_merged() {
        let graphics = include_str!("../../../vendor/iced/graphics/src/layer.rs");
        let wgpu = include_str!("../../../vendor/iced/wgpu/src/layer.rs");

        assert!(
            graphics.contains("fn is_isolated(&self) -> bool"),
            "the layer trait lost its isolation hook"
        );
        assert!(
            graphics.contains("if target.is_isolated()")
                && graphics.contains("candidate.is_isolated()"),
            "`Stack::merge` no longer refuses to move an isolated layer"
        );
        assert!(
            wgpu.contains("reads_backdrop"),
            "the wgpu layer no longer calls a reader's layer isolated"
        );
    }

    /// The shortcut menu is an opaque sheet, not a pane.
    ///
    /// It was glass when the rollout landed, on the argument that a menu is the
    /// one surface the drawing shows through. Measured over the model space on
    /// the sheet's dark theme, the pane's 72% tint plus the sheet's `line` edge
    /// (which is darker than the `Float` fill) left the menu with no visible
    /// plate at all — the 2026-11-02 review read the result as a transparent
    /// menu. A shortcut menu is an object the size of a field, so it is written
    /// on `popup_plate`: the `Float` rung with the field edge the layer palette
    /// and the properties panel already wear.
    #[test]
    fn the_context_menu_is_an_opaque_sheet() {
        let overlay = include_str!("../../app/view/overlay.rs");
        let start = overlay
            .find("pub(super) fn viewport_context_menu_overlay")
            .expect("the viewport context menu overlay");
        let end = overlay[start..]
            .find("\nfn context_menu_row(")
            .map(|offset| start + offset)
            .expect("the end of the overlay function");
        let body = &overlay[start..end];

        assert!(
            body.contains("kit::popup_plate("),
            "the context menu no longer draws the kit's menu paper"
        );
        assert!(
            !body.contains("kit::glass_pane("),
            "the context menu went back to reading the drawing through its own rows"
        );
        assert!(
            !body.contains("Tier::Float"),
            "the context menu should take `popup_plate`, not re-spell the rung"
        );
    }

    /// The fill is the legibility control, not a decoration: whatever the
    /// sample behind it turns into, the tint has to keep the plate on one side
    /// of the ink. The specular stays a hairline so it never reads as a frame.
    #[test]
    fn the_fill_is_a_legibility_floor_in_both_themes() {
        for theme in [
            crate::ui::style::draftflow_theme::draftflow_light(),
            crate::ui::style::draftflow_theme::draftflow_dark(),
        ] {
            let tint = tokens::glass_tint(&theme);
            assert!(
                (0.5..=0.9).contains(&tint.a),
                "glass tint alpha {tint:?} is outside the legibility floor"
            );

            let (top, bottom) = tokens::glass_hi(&theme);
            assert!(
                top.a > bottom.a && top.a < 0.15 && bottom.a < 0.05,
                "the glass specular ramp must fall away from a faint top: {top:?} {bottom:?}"
            );

            assert!(
                tokens::glass_edge(&theme).a > 0.0,
                "glass without an edge is a smudge, not a pane"
            );
        }
    }
}
