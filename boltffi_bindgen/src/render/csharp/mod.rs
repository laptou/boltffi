//! C# backend. Generates `.cs` source files using P/Invoke (`[DllImport]`)
//! to call the C ABI functions that BoltFFI exports.

mod emit;
mod lower;
mod mappings;
mod names;
mod plan;
mod templates;

pub use emit::{CSharpEmitter, CSharpOutput};
pub use names::NamingConvention;
pub use plan::*;

use boltffi_ffi_rules::naming::{LibraryName, Name};

#[derive(Debug, Clone, Default)]
pub struct CSharpOptions {
    /// Override the native library name used in `[DllImport("...")]` declarations.
    /// Defaults to the crate/package name when `None`.
    pub library_name: Option<Name<LibraryName>>,
}
