use boltffi_ffi_rules::naming;

use crate::ir::{AbiContract, FfiContract};

use super::{CSharpModule, CSharpOptions, NamingConvention};

/// Transforms the language-agnostic [`FfiContract`] and [`AbiContract`] into
/// a [`CSharpModule`] containing everything the C# templates need to render.
pub struct CSharpLowerer<'a> {
    ffi: &'a FfiContract,
    #[allow(dead_code)]
    abi: &'a AbiContract,
    options: &'a CSharpOptions,
}

impl<'a> CSharpLowerer<'a> {
    pub fn new(ffi: &'a FfiContract, abi: &'a AbiContract, options: &'a CSharpOptions) -> Self {
        Self { ffi, abi, options }
    }

    /// Walk the contracts and produce a C# module plan.
    pub fn lower(&self) -> CSharpModule {
        let lib_name = self
            .options
            .library_name
            .clone()
            .unwrap_or_else(|| naming::library_name(&self.ffi.package.name));

        let class_name = NamingConvention::class_name(&self.ffi.package.name);
        let namespace = NamingConvention::namespace(&self.ffi.package.name);
        let prefix = naming::ffi_prefix().to_string();

        CSharpModule {
            namespace,
            class_name,
            lib_name,
            prefix,
        }
    }
}
