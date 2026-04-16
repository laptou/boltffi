//! Orchestrates the lowerer and templates to produce the final `.cs` source output.

use askama::Template as _;

use crate::ir::{AbiContract, FfiContract};

use super::{
    CSharpOptions,
    lower::CSharpLowerer,
    templates::{NativeTemplate, PreambleTemplate},
};

/// The rendered C# output: source code plus metadata for file naming.
#[derive(Debug, Clone)]
pub struct CSharpOutput {
    /// The generated C# source code.
    pub source: String,
    /// The top-level class name (used for the file name, e.g., `"MyApp.cs"`).
    pub class_name: String,
    /// The C# namespace.
    pub namespace: String,
}

/// Entry point for C# code generation. Creates the lowerer, walks the
/// contracts, feeds the plan into templates, and produces a [`CSharpOutput`].
pub struct CSharpEmitter;

impl CSharpEmitter {
    pub fn emit(ffi: &FfiContract, abi: &AbiContract, options: &CSharpOptions) -> CSharpOutput {
        let lowerer = CSharpLowerer::new(ffi, abi, options);
        let module = lowerer.lower();

        let mut source = String::new();

        source.push_str(&PreambleTemplate { module: &module }.render().unwrap());
        source.push('\n');
        source.push_str(&NativeTemplate { module: &module }.render().unwrap());
        source.push('\n');

        CSharpOutput {
            class_name: module.class_name,
            namespace: module.namespace,
            source,
        }
    }
}
