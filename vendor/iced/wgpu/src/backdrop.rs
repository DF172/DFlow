//! The backdrop channel.
//!
//! A [`Primitive`](crate::Primitive) that declares `reads_backdrop` needs the pixels already
//! painted beneath it. Those live in the swapchain texture, which a render pass cannot both
//! write and sample, so the frame is split instead: everything below the first reader is
//! painted into [`State::base`], downsampled, blurred, and handed to the reader as a
//! sampleable view.
//!
//! Nothing is allocated and no extra pass is issued for a frame without a reader.
use crate::graphics::Viewport;

/// Stages after `base`: half, quarter, scratch, blurred.
const STAGES: usize = 5;

/// One tap of the blur, in UV units. Zero selects the copy path.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    texel: [f32; 2],
    /// `1 / size` of the render target of the current pass.
    out_inv: [f32; 2],
    /// Blur strength, in taps.
    radius: f32,
    /// `0` — bilinear copy (also the 2× downsample), `1` — five taps along `texel`.
    taps: u32,
    _pad: [u32; 2],
}

#[derive(Clone, Debug)]
struct Surface {
    /// Held so the texture is not destroyed while [`Surface::view`] samples it.
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: [u32; 2],
}

impl Surface {
    fn new(
        device: &wgpu::Device,
        label: &str,
        format: wgpu::TextureFormat,
        size: [u32; 2],
    ) -> Self {
        let size = [size[0].max(1), size[1].max(1)];

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            _texture: texture,
            view,
            size,
        }
    }
}

/// The pixels beneath a [`Primitive`](crate::Primitive), ready to sample.
pub struct Backdrop<'a> {
    /// The blurred backdrop.
    pub view: &'a wgpu::TextureView,
    /// Size of [`Backdrop::view`] in physical pixels.
    pub size: [u32; 2],
    /// Size of the frame the reader is painted into, in physical pixels.
    pub frame_size: [u32; 2],
}

/// Owns the offscreen chain of the backdrop channel.
#[derive(Debug)]
pub struct State {
    chain: Vec<Surface>,
    layout: Option<wgpu::BindGroupLayout>,
    pipeline: Option<wgpu::RenderPipeline>,
    sampler: Option<wgpu::Sampler>,
    params: Vec<wgpu::Buffer>,
    /// One bind group per slot: `0` is the blit, `1..STAGES` are the blur stages.
    binds: Vec<wgpu::BindGroup>,
    size: [u32; 2],
    format: Option<wgpu::TextureFormat>,
    radius: f32,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    /// An empty chain; the first frame with a reader builds it.
    pub fn new() -> Self {
        Self {
            chain: Vec::new(),
            layout: None,
            pipeline: None,
            sampler: None,
            params: Vec::new(),
            binds: Vec::new(),
            size: [0, 0],
            format: None,
            radius: 1.5,
        }
    }

    /// How far the backdrop is smeared, in blur taps.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.max(0.0);
    }

    /// Returns `true` when the chain matches the viewport size and surface format.
    pub(crate) fn is_ready(&self, size: [u32; 2]) -> bool {
        self.chain.len() == STAGES && self.size == size && self.format.is_some()
    }

    fn is_current(&self, size: [u32; 2], format: wgpu::TextureFormat) -> bool {
        self.is_ready(size) && self.format == Some(format)
    }

    /// The full-resolution target the layers beneath a reader are painted into.
    pub fn base(&self) -> Option<&wgpu::TextureView> {
        self.chain.first().map(|stage| &stage.view)
    }

    /// The blurred backdrop a reader samples.
    pub fn blurred(&self) -> Option<BackdropView> {
        let stage = self.chain.last()?;

        Some(BackdropView {
            view: stage.view.clone(),
            size: stage.size,
        })
    }

    /// (Re)allocates the chain when the viewport or the surface format moved.
    pub fn ensure(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, size: [u32; 2]) {
        if size[0] == 0 || size[1] == 0 || self.is_current(size, format) {
            return;
        }

        self.size = size;
        self.format = Some(format);
        self.chain = Vec::new();

        let stages = [
            size,
            [size[0].div_ceil(2), size[1].div_ceil(2)],
            [size[0].div_ceil(4), size[1].div_ceil(4)],
            [size[0].div_ceil(4), size[1].div_ceil(4)],
            [size[0].div_ceil(4), size[1].div_ceil(4)],
        ];

        for (index, size) in stages.iter().enumerate() {
            self.chain.push(Surface::new(
                device,
                &format!("iced_wgpu.backdrop.{index}"),
                format,
                *size,
            ));
        }

        if self.pipeline.is_none() {
            self.create_pipeline(device, format);
        }

        self.create_binds(device);
    }

    fn create_pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("iced_wgpu.backdrop.layout"),
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
            label: Some("iced_wgpu.backdrop.pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("iced_wgpu.backdrop.shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shader/backdrop.wgsl"
            ))),
        });

        self.pipeline = Some(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("iced_wgpu.backdrop.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
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
        }));

        self.sampler = Some(device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("iced_wgpu.backdrop.sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        }));

        self.params = (0..STAGES)
            .map(|index| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("iced_wgpu.backdrop.params.{index}")),
                    size: std::mem::size_of::<Params>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();

        self.layout = Some(layout);
    }

    /// Slot `0` reads `base` and paints the frame; slots `1..` walk the blur chain.
    fn create_binds(&mut self, device: &wgpu::Device) {
        let (Some(layout), Some(sampler)) = (self.layout.as_ref(), self.sampler.as_ref()) else {
            return;
        };

        self.binds = (0..STAGES)
            .map(|slot| {
                let source = if slot == 0 { 0 } else { slot - 1 };

                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("iced_wgpu.backdrop.bind.{slot}")),
                    layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &self.chain[source].view,
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.params[slot].as_entire_binding(),
                        },
                    ],
                })
            })
            .collect();
    }

    fn pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        label: &str,
        target: &wgpu::TextureView,
        slot: usize,
    ) -> Option<wgpu::RenderPass<'a>> {
        let (Some(pipeline), Some(bind)) = (self.pipeline.as_ref(), self.binds.get(slot)) else {
            return None;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);

        Some(pass)
    }

    /// Writes the parameters of every stage, then moves `base` onto the frame and blurs it.
    pub fn compose(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &wgpu::TextureView,
    ) {
        if self.binds.len() < STAGES || self.chain.len() < STAGES {
            return;
        }

        let copy = |size: [u32; 2]| Params {
            texel: [0.0; 2],
            out_inv: [1.0 / size[0] as f32, 1.0 / size[1] as f32],
            radius: 0.0,
            taps: 0,
            _pad: [0; 2],
        };

        let quarter = self.chain[2].size;
        let blur = |axis: usize| Params {
            texel: if axis == 0 {
                [1.0 / quarter[0] as f32, 0.0]
            } else {
                [0.0, 1.0 / quarter[1] as f32]
            },
            out_inv: [1.0 / quarter[0] as f32, 1.0 / quarter[1] as f32],
            radius: self.radius,
            taps: 1,
            _pad: [0; 2],
        };

        // Slot 0 is the only stage that paints the frame: it moves `base` onto it 1:1.
        queue.write_buffer(&self.params[0], 0, bytemuck::bytes_of(&copy(self.size)));
        let _ = self.pass(encoder, "iced_wgpu.backdrop.blit", frame, 0);

        // Slots 1.. walk the chain: two bilinear halvings, then a separable five-tap blur.
        for slot in 1..STAGES {
            let params = match slot {
                1 | 2 => copy(self.chain[slot].size),
                3 => blur(0),
                _ => blur(1),
            };

            queue.write_buffer(&self.params[slot], 0, bytemuck::bytes_of(&params));

            let _ = self.pass(
                encoder,
                "iced_wgpu.backdrop.blur",
                &self.chain[slot].view,
                slot,
            );
        }
    }

    /// The viewport size the chain is built for.
    pub fn physical_size(viewport: &Viewport) -> [u32; 2] {
        [viewport.physical_width(), viewport.physical_height()]
    }
}

/// An owned handle to the blurred backdrop, so a reader can be handed one without
/// borrowing the [`State`] it lives in.
#[derive(Clone, Debug)]
pub struct BackdropView {
    view: wgpu::TextureView,
    size: [u32; 2],
}

impl BackdropView {
    /// The texture view to sample.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Size of the view in physical pixels.
    pub fn size(&self) -> [u32; 2] {
        self.size
    }
}
