use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde_reflection::{
    ContainerFormat, Format, Named, Registry, Samples, Tracer, TracerConfig, VariantFormat,
};

// Include the stable schema types so the same definitions are used at build
// time and at runtime. The file is self-contained and only depends on serde.
include!("src/type_registry_types.rs");

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    generate_type_registry(&out_dir);
}

// ════════════════════════════════════════════════════════════════════════════
// Type registry
// ════════════════════════════════════════════════════════════════════════════

fn generate_type_registry(out_dir: &Path) {
    let mut tracer = Tracer::new(TracerConfig::default());
    let mut samples = Samples::new();
    add_enum_samples(&mut tracer, &mut samples);

    // Trace the complete record roots used by the editor and MCP API. Nested
    // entity/object variants and their enums are added to the same registry.
    type TraceFn = fn(&mut Tracer, &Samples);
    let types: Vec<(&str, TraceFn)> = vec![
        ("EntityType", trace::<codec::EntityType>),
        ("ObjectType", trace::<codec::objects::ObjectType>),
        ("HeaderVariables", trace::<codec::document::HeaderVariables>),
        ("SummaryInfo", trace::<codec::document::SummaryInfo>),
        ("LineType", trace::<codec::LineType>),
        ("TextStyle", trace::<codec::TextStyle>),
        ("BlockRecord", trace::<codec::BlockRecord>),
        ("DimStyle", trace::<codec::DimStyle>),
        ("AppId", trace::<codec::AppId>),
        ("View", trace::<codec::View>),
        ("VPort", trace::<codec::VPort>),
        ("Ucs", trace::<codec::Ucs>),
        ("VxTableRecord", trace::<codec::VxTableRecord>),
        ("DxfClass", trace::<codec::classes::DxfClass>),
        (
            "BlockVisibilityParameter",
            trace::<codec::objects::BlockVisibilityParameter>,
        ),
        ("FieldDef", trace::<codec::document::FieldDef>),
        ("DgnLsDefinition", trace::<codec::objects::DgnLsDefinition>),
        ("DgnLsComponent", trace::<codec::objects::DgnLsComponent>),
        (
            "NotificationCollection",
            trace::<codec::notification::NotificationCollection>,
        ),
        ("Preview", trace::<codec::document::Preview>),
        ("Point", trace::<codec::Point>),
        ("Line", trace::<codec::Line>),
        ("Circle", trace::<codec::Circle>),
        ("Arc", trace::<codec::Arc>),
        ("Ellipse", trace::<codec::Ellipse>),
        ("Polyline", trace::<codec::Polyline>),
        ("Polyline2D", trace::<codec::entities::Polyline2D>),
        ("Polyline3D", trace::<codec::entities::Polyline3D>),
        ("LwPolyline", trace::<codec::LwPolyline>),
        ("MText", trace::<codec::entities::MText>),
        ("Spline", trace::<codec::Spline>),
        ("EntityCommon", trace::<codec::entities::EntityCommon>),
        ("Handle", trace::<codec::Handle>),
        ("Vector2", trace::<codec::Vector2>),
        ("Vector3", trace::<codec::Vector3>),
        ("Color", trace::<codec::Color>),
        ("Layer", trace::<codec::Layer>),
        ("XDataValue", trace::<codec::xdata::XDataValue>),
        ("XRecord", trace::<codec::objects::XRecord>),
        ("XRecordEntry", trace::<codec::objects::XRecordEntry>),
        ("XRecordValue", trace::<codec::objects::XRecordValue>),
        (
            "XRecordValueType",
            trace::<codec::objects::XRecordValueType>,
        ),
        ("XRecordSection", trace::<codec::objects::XRecordSection>),
        (
            "DictionaryCloningFlags",
            trace::<codec::objects::DictionaryCloningFlags>,
        ),
        (
            "KnownXRecordKind",
            trace::<codec::objects::KnownXRecordKind>,
        ),
        (
            "ProxyObjectReference",
            trace::<codec::objects::ProxyObjectReference>,
        ),
        (
            "ProxyReferenceKind",
            trace::<codec::objects::ProxyReferenceKind>,
        ),
    ];

    for (name, f) in types {
        f(&mut tracer, &samples);
        // serde-reflection accumulates named types as it traces; we only need
        // to ensure the seed types are recorded even if a nested trace fails.
        eprintln!(
            "[draftflow_plugin_api build] traced type registry entry: {}",
            name
        );
    }

    type TraceSimpleFn = fn(&mut Tracer);
    let enums: Vec<(&str, TraceSimpleFn)> = vec![
        (
            "AssocAnnotationKind",
            trace_simple::<codec::objects::AssocAnnotationKind>,
        ),
        (
            "AssocConstraintNodeData",
            trace_simple::<codec::objects::AssocConstraintNodeData>,
        ),
        (
            "AssocCurveValue",
            trace_simple::<codec::objects::AssocCurveValue>,
        ),
        (
            "AssocEvalValue",
            trace_simple::<codec::objects::AssocEvalValue>,
        ),
        (
            "AssocSubcurveKind",
            trace_simple::<codec::objects::AssocSubcurveKind>,
        ),
        (
            "AssocSurfaceActionKind",
            trace_simple::<codec::objects::AssocSurfaceActionKind>,
        ),
        (
            "AssocViewObjectActionParamKind",
            trace_simple::<codec::objects::AssocViewObjectActionParamKind>,
        ),
        ("AcisVersion", trace_simple::<codec::entities::AcisVersion>),
        (
            "AssociativeData",
            trace_simple::<codec::objects::AssociativeData>,
        ),
        (
            "AttachmentPointType",
            trace_simple::<codec::entities::AttachmentPointType>,
        ),
        (
            "BlockContentConnectionType",
            trace_simple::<codec::entities::BlockContentConnectionType>,
        ),
        (
            "BlockEvalValue",
            trace_simple::<codec::objects::BlockEvalValue>,
        ),
        ("BorderType", trace_simple::<codec::entities::BorderType>),
        (
            "BoundaryEdge",
            trace_simple::<codec::entities::BoundaryEdge>,
        ),
        (
            "BreakFlowDirection",
            trace_simple::<codec::entities::BreakFlowDirection>,
        ),
        (
            "CellAlignment",
            trace_simple::<codec::objects::CellAlignment>,
        ),
        (
            "CellStyleType",
            trace_simple::<codec::entities::CellStyleType>,
        ),
        ("CellType", trace_simple::<codec::entities::CellType>),
        (
            "CellValueType",
            trace_simple::<codec::entities::CellValueType>,
        ),
        (
            "ClassObjectData",
            trace_simple::<codec::objects::ClassObjectData>,
        ),
        ("ClipMode", trace_simple::<codec::entities::ClipMode>),
        ("ClipType", trace_simple::<codec::entities::ClipType>),
        (
            "CompoundEntry",
            trace_simple::<codec::compound_file::CompoundEntry>,
        ),
        (
            "CompoundPropertyValue",
            trace_simple::<codec::compound_file::CompoundPropertyValue>,
        ),
        (
            "CompoundStreamContent",
            trace_simple::<codec::compound_file::CompoundStreamContent>,
        ),
        (
            "DataObjectData",
            trace_simple::<codec::objects::DataObjectData>,
        ),
        (
            "DgnLineStyleData",
            trace_simple::<codec::objects::DgnLineStyleData>,
        ),
        (
            "DgnLsComponentData",
            trace_simple::<codec::objects::DgnLsComponentData>,
        ),
        (
            "DgnLsComponentType",
            trace_simple::<codec::objects::DgnLsComponentType>,
        ),
        (
            "DgnLsPhaseMode",
            trace_simple::<codec::objects::DgnLsPhaseMode>,
        ),
        ("DimSubtype", trace_simple::<codec::objects::DimSubtype>),
        ("Dimension", trace_simple::<codec::entities::Dimension>),
        (
            "DimensionType",
            trace_simple::<codec::entities::DimensionType>,
        ),
        (
            "DynamicBlockData",
            trace_simple::<codec::objects::DynamicBlockData>,
        ),
        (
            "EmbeddedEntity",
            trace_simple::<codec::entities::EmbeddedEntity>,
        ),
        (
            "ExtendedEntityData",
            trace_simple::<codec::entities::ExtendedEntityData>,
        ),
        (
            "FlowDirectionType",
            trace_simple::<codec::entities::FlowDirectionType>,
        ),
        (
            "HatchPatternType",
            trace_simple::<codec::entities::HatchPatternType>,
        ),
        (
            "HatchStyleType",
            trace_simple::<codec::entities::HatchStyleType>,
        ),
        (
            "HelixConstraint",
            trace_simple::<codec::entities::HelixConstraint>,
        ),
        (
            "HooklineDirection",
            trace_simple::<codec::entities::HooklineDirection>,
        ),
        (
            "HorizontalAlignment",
            trace_simple::<codec::entities::HorizontalAlignment>,
        ),
        (
            "LeaderContentType",
            trace_simple::<codec::entities::LeaderContentType>,
        ),
        (
            "LeaderCreationType",
            trace_simple::<codec::entities::LeaderCreationType>,
        ),
        (
            "LeaderDrawOrderType",
            trace_simple::<codec::objects::LeaderDrawOrderType>,
        ),
        (
            "LeaderPathType",
            trace_simple::<codec::entities::LeaderPathType>,
        ),
        (
            "LineTypeComplexContent",
            trace_simple::<codec::tables::LineTypeComplexContent>,
        ),
        (
            "LegacyEntityData",
            trace_simple::<codec::entities::LegacyEntityData>,
        ),
        (
            "MLineJustification",
            trace_simple::<codec::entities::MLineJustification>,
        ),
        ("MTextFlag", trace_simple::<codec::entities::MTextFlag>),
        (
            "MaterialProceduralValue",
            trace_simple::<codec::objects::MaterialProceduralValue>,
        ),
        (
            "MultiLeaderDrawOrderType",
            trace_simple::<codec::objects::MultiLeaderDrawOrderType>,
        ),
        (
            "MultiLeaderPathType",
            trace_simple::<codec::entities::MultiLeaderPathType>,
        ),
        (
            "ObjectContextKind",
            trace_simple::<codec::objects::ObjectContextKind>,
        ),
        (
            "NotificationType",
            trace_simple::<codec::notification::NotificationType>,
        ),
        (
            "OleFrameEnvelope",
            trace_simple::<codec::entities::OleFrameEnvelope>,
        ),
        (
            "OleObjectType",
            trace_simple::<codec::entities::OleObjectType>,
        ),
        (
            "PlotPaperUnits",
            trace_simple::<codec::objects::PlotPaperUnits>,
        ),
        ("PlotRotation", trace_simple::<codec::objects::PlotRotation>),
        ("PlotType", trace_simple::<codec::objects::PlotType>),
        (
            "PreviewFormat",
            trace_simple::<codec::document::PreviewFormat>,
        ),
        (
            "PolyfaceSmoothType",
            trace_simple::<codec::entities::PolyfaceSmoothType>,
        ),
        (
            "ProxyPayloadEncoding",
            trace_simple::<codec::objects::ProxyPayloadEncoding>,
        ),
        (
            "ResolutionUnit",
            trace_simple::<codec::objects::ResolutionUnit>,
        ),
        ("ScaledType", trace_simple::<codec::objects::ScaledType>),
        (
            "SemanticPropertyValue",
            trace_simple::<codec::objects::SemanticPropertyValue>,
        ),
        (
            "ShadePlotMode",
            trace_simple::<codec::objects::ShadePlotMode>,
        ),
        (
            "ShadePlotResolutionLevel",
            trace_simple::<codec::objects::ShadePlotResolutionLevel>,
        ),
        (
            "SolidHistoryOperation",
            trace_simple::<codec::objects::SolidHistoryOperation>,
        ),
        ("SurfaceData", trace_simple::<codec::entities::SurfaceData>),
        ("SurfaceKind", trace_simple::<codec::entities::SurfaceKind>),
        (
            "SurfaceSmoothType",
            trace_simple::<codec::entities::SurfaceSmoothType>,
        ),
        (
            "TableBorderType",
            trace_simple::<codec::objects::TableBorderType>,
        ),
        (
            "TableCellContentType",
            trace_simple::<codec::entities::TableCellContentType>,
        ),
        (
            "TableFlowDirection",
            trace_simple::<codec::objects::TableFlowDirection>,
        ),
        (
            "TextAlignmentType",
            trace_simple::<codec::entities::TextAlignmentType>,
        ),
        (
            "TextAngleType",
            trace_simple::<codec::entities::TextAngleType>,
        ),
        (
            "TextAttachmentDirectionType",
            trace_simple::<codec::entities::TextAttachmentDirectionType>,
        ),
        (
            "TextAttachmentPointType",
            trace_simple::<codec::entities::TextAttachmentPointType>,
        ),
        (
            "TextAttachmentType",
            trace_simple::<codec::entities::TextAttachmentType>,
        ),
        (
            "TextHorizontalAlignment",
            trace_simple::<codec::entities::TextHorizontalAlignment>,
        ),
        (
            "TextVerticalAlignment",
            trace_simple::<codec::entities::TextVerticalAlignment>,
        ),
        (
            "UnderlayType",
            trace_simple::<codec::entities::UnderlayType>,
        ),
        (
            "ValueUnitType",
            trace_simple::<codec::entities::ValueUnitType>,
        ),
        (
            "VbaDirectoryValue",
            trace_simple::<codec::vba::VbaDirectoryValue>,
        ),
        (
            "VerticalAlignment",
            trace_simple::<codec::entities::VerticalAlignment>,
        ),
        (
            "ViewportRenderMode",
            trace_simple::<codec::entities::ViewportRenderMode>,
        ),
        (
            "VisualStylePropertyValue",
            trace_simple::<codec::objects::VisualStylePropertyValue>,
        ),
        (
            "ViewRepSketchGeometry",
            trace_simple::<codec::objects::ViewRepSketchGeometry>,
        ),
        (
            "WipeoutClipMode",
            trace_simple::<codec::entities::WipeoutClipMode>,
        ),
        (
            "WipeoutClipType",
            trace_simple::<codec::entities::WipeoutClipType>,
        ),
        ("WireType", trace_simple::<codec::entities::WireType>),
    ];
    for (name, trace) in enums {
        trace(&mut tracer);
        eprintln!("[draftflow_plugin_api build] traced enum variants: {name}");
    }

    let traced = tracer
        .registry()
        .expect("type registry tracing failed; see stderr for individual errors");
    let mut registry = map_to_custom_schema(&traced);
    let mut section_style_tracer = Tracer::new(TracerConfig::default());
    section_style_tracer
        .trace_simple_type::<codec::entities::SectionViewStyle>()
        .expect("section view style tracing failed");
    let section_style_registry = section_style_tracer
        .registry()
        .expect("section view style registry failed");
    let section_style = section_style_registry
        .get("SectionViewStyle")
        .expect("section view style registry entry");
    registry.types.insert(
        TypeId::new("EntitySectionViewStyle"),
        map_container("EntitySectionViewStyle", section_style),
    );
    let json = serde_json::to_string_pretty(&registry).unwrap();
    fs::write(out_dir.join("type_registry.json"), json).unwrap();
}

fn trace<T>(tracer: &mut Tracer, samples: &Samples)
where
    T: serde::Serialize + DeserializeOwned,
{
    let name = std::any::type_name::<T>();
    if let Err(e) = tracer.trace_type::<T>(samples) {
        eprintln!(
            "[draftflow_plugin_api build] warning: tracing {} failed: {}",
            name, e
        );
    }
}

fn trace_simple<T>(tracer: &mut Tracer)
where
    T: DeserializeOwned,
{
    if let Err(error) = tracer.trace_simple_type::<T>() {
        panic!(
            "type registry enum tracing failed for {}: {error}",
            std::any::type_name::<T>()
        );
    }
}

fn add_enum_samples(tracer: &mut Tracer, samples: &mut Samples) {
    // serde-reflection needs at least one sample value per enum variant in
    // order to reconstruct the full schema. Provide samples for the enums that
    // need concrete serialized values.
    let _ = tracer.trace_value(samples, &codec::LineWeight::ByLayer);
    let _ = tracer.trace_value(samples, &codec::LineWeight::ByBlock);
    let _ = tracer.trace_value(samples, &codec::LineWeight::Default);
    let _ = tracer.trace_value(samples, &codec::LineWeight::Value(0));

    let _ = tracer.trace_value(samples, &codec::Transparency::BY_LAYER);
    let _ = tracer.trace_value(samples, &codec::Transparency::BY_BLOCK);
    let _ = tracer.trace_value(samples, &codec::Transparency::OPAQUE);

    let _ = tracer.trace_value(samples, &codec::entities::SmoothSurfaceType::None);
    let _ = tracer.trace_value(
        samples,
        &codec::entities::SmoothSurfaceType::QuadraticBSpline,
    );
    let _ = tracer.trace_value(samples, &codec::entities::SmoothSurfaceType::CubicBSpline);
    let _ = tracer.trace_value(samples, &codec::entities::SmoothSurfaceType::Bezier);

    let _ = tracer.trace_value(samples, &codec::entities::AttachmentPoint::TopLeft);
    let _ = tracer.trace_value(samples, &codec::entities::AttachmentPoint::TopCenter);
    let _ = tracer.trace_value(samples, &codec::entities::AttachmentPoint::TopRight);
    let _ = tracer.trace_value(samples, &codec::entities::AttachmentPoint::MiddleLeft);
    let _ = tracer.trace_value(samples, &codec::entities::AttachmentPoint::MiddleCenter);
    let _ = tracer.trace_value(samples, &codec::entities::AttachmentPoint::MiddleRight);
    let _ = tracer.trace_value(samples, &codec::entities::AttachmentPoint::BottomLeft);
    let _ = tracer.trace_value(samples, &codec::entities::AttachmentPoint::BottomCenter);
    let _ = tracer.trace_value(samples, &codec::entities::AttachmentPoint::BottomRight);

    let _ = tracer.trace_value(samples, &codec::entities::DrawingDirection::LeftToRight);
    let _ = tracer.trace_value(samples, &codec::entities::DrawingDirection::TopToBottom);
    let _ = tracer.trace_value(samples, &codec::entities::DrawingDirection::ByStyle);

    let _ = tracer.trace_value(samples, &codec::entities::LineSpacingStyle::AtLeast);
    let _ = tracer.trace_value(samples, &codec::entities::LineSpacingStyle::Exactly);

    let _ = tracer.trace_value(samples, &codec::xdata::XDataValue::String(String::new()));
    let _ = tracer.trace_value(
        samples,
        &codec::xdata::XDataValue::ControlString(String::new()),
    );
    let _ = tracer.trace_value(samples, &codec::xdata::XDataValue::LayerName(String::new()));
    let _ = tracer.trace_value(samples, &codec::xdata::XDataValue::BinaryData(Vec::new()));
    let _ = tracer.trace_value(
        samples,
        &codec::xdata::XDataValue::Handle(codec::Handle::default()),
    );
    let _ = tracer.trace_value(
        samples,
        &codec::xdata::XDataValue::Point3D(codec::Vector3::default()),
    );
    let _ = tracer.trace_value(
        samples,
        &codec::xdata::XDataValue::Position3D(codec::Vector3::default()),
    );
    let _ = tracer.trace_value(
        samples,
        &codec::xdata::XDataValue::Displacement3D(codec::Vector3::default()),
    );
    let _ = tracer.trace_value(
        samples,
        &codec::xdata::XDataValue::Direction3D(codec::Vector3::default()),
    );
    let _ = tracer.trace_value(samples, &codec::xdata::XDataValue::Real(0.0));
    let _ = tracer.trace_value(samples, &codec::xdata::XDataValue::Distance(0.0));
    let _ = tracer.trace_value(samples, &codec::xdata::XDataValue::ScaleFactor(0.0));
    let _ = tracer.trace_value(samples, &codec::xdata::XDataValue::Integer16(0));
    let _ = tracer.trace_value(samples, &codec::xdata::XDataValue::Integer32(0));

    // XRecord value variants
    let _ = tracer.trace_value(
        samples,
        &codec::objects::XRecordValue::String(String::new()),
    );
    let _ = tracer.trace_value(samples, &codec::objects::XRecordValue::Double(0.0));
    let _ = tracer.trace_value(samples, &codec::objects::XRecordValue::Int16(0));
    let _ = tracer.trace_value(samples, &codec::objects::XRecordValue::Int32(0));
    let _ = tracer.trace_value(samples, &codec::objects::XRecordValue::Int64(0));
    let _ = tracer.trace_value(samples, &codec::objects::XRecordValue::Byte(0));
    let _ = tracer.trace_value(samples, &codec::objects::XRecordValue::Bool(false));
    let _ = tracer.trace_value(
        samples,
        &codec::objects::XRecordValue::Handle(codec::Handle::default()),
    );
    let _ = tracer.trace_value(
        samples,
        &codec::objects::XRecordValue::Point3D(0.0, 0.0, 0.0),
    );
    let _ = tracer.trace_value(samples, &codec::objects::XRecordValue::Chunk(Vec::new()));

    // Proxy reference kinds pulled in by XRecord.object_references
    let _ = tracer.trace_value(samples, &codec::objects::ProxyReferenceKind::Undefined);
    let _ = tracer.trace_value(samples, &codec::objects::ProxyReferenceKind::SoftOwnership);
    let _ = tracer.trace_value(samples, &codec::objects::ProxyReferenceKind::HardOwnership);
    let _ = tracer.trace_value(samples, &codec::objects::ProxyReferenceKind::SoftPointer);
    let _ = tracer.trace_value(samples, &codec::objects::ProxyReferenceKind::HardPointer);

    // KnownXRecordKind variants
    let _ = tracer.trace_value(
        samples,
        &codec::objects::KnownXRecordKind::LayerViewportAlphaOverride,
    );
    let _ = tracer.trace_value(samples, &codec::objects::KnownXRecordKind::Unknown);
}

fn map_to_custom_schema(traced: &Registry) -> TypeRegistry {
    let mut types = BTreeMap::new();
    for (name, format) in traced.iter() {
        let info = map_container(name, format);
        types.insert(TypeId(name.clone()), info);
    }
    TypeRegistry { types }
}

fn map_container(name: &str, format: &ContainerFormat) -> TypeInfo {
    match format {
        ContainerFormat::Struct(fields) => TypeInfo {
            name: TypeId(name.to_string()),
            kind: TypeKind::Struct,
            fields: fields.iter().map(map_field).collect(),
            variants: vec![],
            methods: vec![],
            doc: None,
        },
        ContainerFormat::Enum(variants) => TypeInfo {
            name: TypeId(name.to_string()),
            kind: TypeKind::Enum,
            fields: vec![],
            variants: variants
                .iter()
                .map(|(idx, v)| map_variant(*idx, v))
                .collect(),
            methods: vec![],
            doc: None,
        },
        ContainerFormat::NewTypeStruct(format) => TypeInfo {
            name: TypeId(name.to_string()),
            kind: TypeKind::Newtype,
            fields: vec![FieldInfo {
                name: "0".to_string(),
                ..map_format_field(format)
            }],
            variants: vec![],
            methods: vec![],
            doc: None,
        },
        ContainerFormat::TupleStruct(formats) => TypeInfo {
            name: TypeId(name.to_string()),
            kind: TypeKind::Tuple,
            fields: formats
                .iter()
                .enumerate()
                .map(|(i, f)| FieldInfo {
                    name: i.to_string(),
                    ..map_format_field(f)
                })
                .collect(),
            variants: vec![],
            methods: vec![],
            doc: None,
        },
        ContainerFormat::UnitStruct => TypeInfo {
            name: TypeId(name.to_string()),
            kind: TypeKind::Unit,
            fields: vec![],
            variants: vec![],
            methods: vec![],
            doc: None,
        },
    }
}

fn map_field(named: &Named<Format>) -> FieldInfo {
    let mut field = map_format_field(&named.value);
    field.name = named.name.clone();
    field
}

fn map_format_field(format: &Format) -> FieldInfo {
    if let Some(name) = primitive_format_name(format) {
        return FieldInfo {
            name: String::new(),
            type_id: TypeId(name),
            optional: false,
            is_sequence: false,
        };
    }
    match format {
        Format::TypeName(name) => FieldInfo {
            name: String::new(),
            type_id: TypeId(name.clone()),
            optional: false,
            is_sequence: false,
        },
        Format::Option(inner) => {
            let mut field = map_format_field(inner);
            field.optional = true;
            field
        }
        Format::Seq(inner) => {
            let mut field = map_format_field(inner);
            field.is_sequence = true;
            field
        }
        Format::TupleArray { content, size } => {
            let mut field = map_format_field(content);
            field.type_id = TypeId(format!("[{}; {}]", type_id_of_format(content), size));
            field.is_sequence = true;
            field
        }
        Format::Map { key, value } => FieldInfo {
            name: String::new(),
            type_id: TypeId(format!(
                "Map<{}, {}>",
                type_id_of_format(key),
                type_id_of_format(value)
            )),
            optional: false,
            is_sequence: false,
        },
        Format::Tuple(formats) => FieldInfo {
            name: String::new(),
            type_id: TypeId(format!(
                "({})",
                formats
                    .iter()
                    .map(type_id_of_format)
                    .collect::<Vec<_>>()
                    .join(",")
            )),
            optional: false,
            is_sequence: false,
        },
        Format::Variable(_) => FieldInfo {
            name: String::new(),
            type_id: TypeId("Value".to_string()),
            optional: false,
            is_sequence: false,
        },
        _ => FieldInfo {
            name: String::new(),
            type_id: TypeId("unknown".to_string()),
            optional: false,
            is_sequence: false,
        },
    }
}

fn primitive_format_name(format: &Format) -> Option<String> {
    let name = match format {
        Format::Unit => "()",
        Format::Bool => "bool",
        Format::I8 => "i8",
        Format::I16 => "i16",
        Format::I32 => "i32",
        Format::I64 => "i64",
        Format::I128 => "i128",
        Format::U8 => "u8",
        Format::U16 => "u16",
        Format::U32 => "u32",
        Format::U64 => "u64",
        Format::U128 => "u128",
        Format::F32 => "f32",
        Format::F64 => "f64",
        Format::Char => "char",
        Format::Str => "String",
        Format::Bytes => "bytes",
        _ => return None,
    };
    Some(name.to_string())
}

fn type_id_of_format(format: &Format) -> String {
    if let Some(name) = primitive_format_name(format) {
        return name;
    }
    match format {
        Format::TypeName(name) => name.clone(),
        Format::Option(inner) => format!("Option<{}>", type_id_of_format(inner)),
        Format::Seq(inner) => format!("Vec<{}>", type_id_of_format(inner)),
        Format::TupleArray { content, size } => {
            format!("[{}; {}]", type_id_of_format(content), size)
        }
        Format::Map { key, value } => format!(
            "Map<{}, {}>",
            type_id_of_format(key),
            type_id_of_format(value)
        ),
        Format::Tuple(formats) => format!(
            "({})",
            formats
                .iter()
                .map(type_id_of_format)
                .collect::<Vec<_>>()
                .join(",")
        ),
        Format::Variable(_) => "Value".to_string(),
        _ => "unknown".to_string(),
    }
}

fn map_variant(discriminant: u32, named: &Named<VariantFormat>) -> EnumVariantInfo {
    let fields = match &named.value {
        VariantFormat::Unit => vec![],
        VariantFormat::Variable(_) => vec![],
        VariantFormat::NewType(format) => vec![FieldInfo {
            name: "0".to_string(),
            ..map_format_field(format)
        }],
        VariantFormat::Tuple(formats) => formats
            .iter()
            .enumerate()
            .map(|(i, f)| FieldInfo {
                name: i.to_string(),
                ..map_format_field(f)
            })
            .collect(),
        VariantFormat::Struct(fields) => fields.iter().map(map_field).collect(),
    };
    EnumVariantInfo {
        name: named.name.clone(),
        discriminant,
        fields,
    }
}

