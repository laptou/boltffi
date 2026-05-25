use boltffi_ffi_rules::naming;

use crate::ir::definitions::FunctionDef;
use crate::render::python::{NamingConvention, PythonCallable, PythonFunction, PythonLowerError};

use super::PythonLowerer;

impl PythonLowerer<'_> {
    pub(super) fn lower_functions(&self) -> Result<Vec<PythonFunction>, PythonLowerError> {
        self.ffi_contract.functions.iter().try_fold(
            Vec::new(),
            |mut lowered_functions, function| {
                if let Some(lowered_function) = self.lower_function(function)? {
                    lowered_functions.push(lowered_function);
                }
                Ok(lowered_functions)
            },
        )
    }

    pub(super) fn lower_function(
        &self,
        function: &FunctionDef,
    ) -> Result<Option<PythonFunction>, PythonLowerError> {
        if function.is_async() {
            return Ok(None);
        }

        let Some(parameters) = self.lower_parameters(
            &format!("function `{}`", function.id.as_str()),
            &function.params,
        )?
        else {
            return Ok(None);
        };

        let Some(return_type) = self.lower_return(&function.returns) else {
            return Ok(None);
        };

        let ffi_symbol = self
            .resolve_function_symbol(function)
            .unwrap_or_else(|| naming::function_ffi_name(function.id.as_str()).into_string());

        Ok(Some(PythonFunction {
            python_name: NamingConvention::function_name(function.id.as_str()),
            callable: PythonCallable {
                native_name: NamingConvention::function_name(function.id.as_str()),
                ffi_symbol,
                parameters,
                return_type,
            },
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::PythonLowerer;
    use crate::ir::TypeCatalog;

    use super::super::test_support::{contract_parts, test_function};

    #[test]
    fn lower_function_escapes_python_keywords() {
        let catalog = TypeCatalog::default();
        let function = test_function("class", &["from"]);
        let (ffi_contract, abi_contract) = contract_parts(catalog, vec![function.clone()]);

        let lowered = PythonLowerer::new(
            &ffi_contract,
            &abi_contract,
            "demo",
            "demo",
            Some("0.1.0".to_string()),
            "demo",
        )
        .lower_function(&function)
        .expect("function lowering should succeed")
        .expect("function should lower");

        assert_eq!(lowered.python_name, "class_");
        assert_eq!(lowered.callable.parameters[0].name, "from_");
    }
}
