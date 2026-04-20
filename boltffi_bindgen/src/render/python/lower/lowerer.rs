use crate::ir::{AbiContract, FfiContract};
use crate::render::python::{PythonLowerError, PythonModule};

pub struct PythonLowerer<'a> {
    pub(super) ffi_contract: &'a FfiContract,
    pub(super) abi_contract: &'a AbiContract,
    pub(super) module_name: &'a str,
    pub(super) package_name: &'a str,
    pub(super) package_version: Option<String>,
    pub(super) library_name: &'a str,
}

impl<'a> PythonLowerer<'a> {
    pub fn new(
        ffi_contract: &'a FfiContract,
        abi_contract: &'a AbiContract,
        module_name: &'a str,
        package_name: &'a str,
        package_version: Option<String>,
        library_name: &'a str,
    ) -> Self {
        Self {
            ffi_contract,
            abi_contract,
            module_name,
            package_name,
            package_version,
            library_name,
        }
    }

    pub fn lower(&self) -> Result<PythonModule, PythonLowerError> {
        debug_assert_eq!(
            self.abi_contract.callbacks.len(),
            self.ffi_contract.catalog.all_callbacks().count()
        );

        let functions = self.lower_functions()?;
        let enums = self.lower_c_style_enums()?;

        Self::validate_top_level_names(&functions, &enums)?;
        Self::validate_native_module_names(&functions, &enums)?;

        Ok(PythonModule {
            module_name: self.module_name.to_string(),
            package_name: self.package_name.to_string(),
            package_version: self
                .package_version
                .clone()
                .or_else(|| self.ffi_contract.package.version.clone()),
            library_name: self.library_name.to_string(),
            free_buffer_symbol: self.abi_contract.free_buf.to_string(),
            enums,
            functions,
        })
    }
}
