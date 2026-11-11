//! # DraftFlow CAD ribbon contract and embedded type registry
//!
//! The dependency-free ribbon vocabulary (`CadModule`, `RibbonGroup`, …) the
//! built-in tabs and the automation surface share, plus the type registry
//! generated from `draftflowcodec` at build time.
//!
//! The crate is intentionally dependency-light (serde only at runtime) so the
//! editor and its tooling can read these definitions without pulling the full
//! `draftflowcodec` tree.

pub mod ribbon;
pub mod type_registry;
pub mod type_registry_types;

pub use ribbon::{CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, StyleKey, ToolDef};
pub use type_registry::{
    get_embedded_type_registry_json, EnumVariantInfo, FieldInfo, MethodInfo, ParameterInfo, TypeId,
    TypeInfo, TypeKind, TypeRegistry,
};
