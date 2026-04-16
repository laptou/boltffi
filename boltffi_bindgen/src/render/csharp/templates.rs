//! Askama template definitions that map plan structs to `.txt` template files.

use askama::Template;

use super::CSharpModule;

/// Renders the file header: auto-generated comment, `using` directives,
/// and namespace declaration.
#[derive(Template)]
#[template(path = "render_csharp/preamble.txt", escape = "none")]
pub struct PreambleTemplate<'a> {
    pub module: &'a CSharpModule,
}

/// Renders the `NativeMethods` static class containing `[DllImport]`
/// declarations for the C FFI functions.
#[derive(Template)]
#[template(path = "render_csharp/native.txt", escape = "none")]
pub struct NativeTemplate<'a> {
    pub module: &'a CSharpModule,
}
