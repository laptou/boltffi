mod emit;
mod lower;
pub mod names;
mod plan;
pub mod primitives;
mod templates;

pub use emit::*;
pub use lower::KotlinLowerer;
pub use names::NamingConvention;
pub use plan::*;
pub use templates::KotlinEmitter;

use boltffi_ffi_rules::naming::{LibraryName, Name};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FactoryStyle {
    #[default]
    Constructors,
    CompanionMethods,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KotlinApiStyle {
    #[default]
    TopLevel,
    ModuleObject,
}

#[derive(Debug, Clone, Default)]
pub struct KotlinOptions {
    pub factory_style: FactoryStyle,
    pub api_style: KotlinApiStyle,
    pub module_object_name: Option<String>,
    pub library_name: Option<Name<LibraryName>>,
    pub desktop_loader: bool,
}
