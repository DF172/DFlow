// Module system — CadModule, ToolDef, RibbonGroup.
//
// To add a ribbon tab (Home, View, …):
//   1. Create `src/modules/my_name/` directory
//   2. Add `src/modules/my_name/mod.rs` implementing `CadModule` as `MyNameModule`
//   3. Add `pub mod my_name;` below
//   4. Add a `Box::new(my_name::MyNameModule)` line in `registry::all_modules()`
//
// Each module folder contains:
//   - mod.rs       : module definition (ribbon groups + tool layout)
//   - <tool>.rs    : one file per tool (ribbon def + future command logic)

// ── Ribbon vocabulary (CadModule, ToolDef, RibbonGroup, …) ─────────────────
//
// These types live in the dependency-free `draftflow_plugin_api` crate. Re-exported
// here to keep the long-used `crate::modules::{CadModule, ToolDef, …}` paths
// stable across the codebase.
pub use draftflow_plugin_api::ribbon::{
    CadModule, IconKind, ModuleEvent, RibbonGroup, RibbonItem, StyleKey, ToolDef,
};

// ── Module declarations ───────────────────────────────────────────────────

pub mod annotate;
pub mod draw;
pub mod insert;
pub mod layout;
pub mod manage;
pub mod model;
pub mod parametric;
pub mod view;

// ── Core module registry ─────────────────────────────────────────────────
// Hand-written `all_modules()` listing the built-in ribbon tabs.
pub mod registry;
