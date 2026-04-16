use boltffi_ffi_rules::naming::{LibraryName, Name};

/// Represents a lowered C# module, containing everything the templates need
/// to render a `.cs` file.
#[derive(Debug, Clone)]
pub struct CSharpModule {
    /// C# namespace for the generated file (e.g., `"MyApp"`).
    pub namespace: String,
    /// Top-level class name (e.g., `"MyApp"`).
    pub class_name: String,
    /// Native library name used in `[DllImport("...")]` declarations.
    pub lib_name: Name<LibraryName>,
    /// FFI symbol prefix (e.g., `"boltffi"`).
    pub prefix: String,
}
