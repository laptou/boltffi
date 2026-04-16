use std::collections::HashMap;

use boltffi_ffi_rules::naming;

use crate::ir::definitions::{FunctionDef, ParamDef, ParamPassing, ReturnDef};
use crate::ir::types::TypeExpr;
use crate::ir::{AbiContract, FfiContract};
use crate::render::python::{
    NamingConvention, PythonFunction, PythonLowerError, PythonModule, PythonParameter, PythonType,
};

pub struct PythonLowerer<'a> {
    ffi_contract: &'a FfiContract,
    abi_contract: &'a AbiContract,
    module_name: &'a str,
    package_name: &'a str,
    package_version: Option<String>,
    library_name: &'a str,
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

        let (functions, _) = self.ffi_contract.functions.iter().try_fold(
            (Vec::new(), HashMap::<String, String>::new()),
            |(mut lowered_functions, mut seen_function_names), function| {
                let Some(lowered_function) = Self::lower_function(function)? else {
                    return Ok((lowered_functions, seen_function_names));
                };

                let source_name = function.id.as_str().to_string();

                if let Some(existing_function) = seen_function_names
                    .insert(lowered_function.python_name.clone(), source_name.clone())
                {
                    return Err(PythonLowerError::TopLevelFunctionNameCollision {
                        generated_name: lowered_function.python_name.clone(),
                        existing_function,
                        colliding_function: source_name,
                    });
                }

                lowered_functions.push(lowered_function);

                Ok((lowered_functions, seen_function_names))
            },
        )?;

        Ok(PythonModule {
            module_name: self.module_name.to_string(),
            package_name: self.package_name.to_string(),
            package_version: self
                .package_version
                .clone()
                .or_else(|| self.ffi_contract.package.version.clone()),
            library_name: self.library_name.to_string(),
            functions,
        })
    }

    fn lower_function(function: &FunctionDef) -> Result<Option<PythonFunction>, PythonLowerError> {
        if function.is_async() {
            return Ok(None);
        }

        let Some(parameters) = Self::lower_parameters(function)? else {
            return Ok(None);
        };

        let Some(return_type) = Self::lower_return(&function.returns) else {
            return Ok(None);
        };

        Ok(Some(PythonFunction {
            python_name: NamingConvention::function_name(function.id.as_str()),
            ffi_symbol: naming::function_ffi_name(function.id.as_str()).into_string(),
            parameters,
            return_type,
        }))
    }

    fn lower_parameters(
        function: &FunctionDef,
    ) -> Result<Option<Vec<PythonParameter>>, PythonLowerError> {
        let Some(parameters) = function
            .params
            .iter()
            .map(Self::lower_parameter)
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };

        Self::validate_parameter_names(function, &parameters)?;

        Ok(Some(parameters))
    }

    fn validate_parameter_names(
        function: &FunctionDef,
        parameters: &[PythonParameter],
    ) -> Result<(), PythonLowerError> {
        function
            .params
            .iter()
            .zip(parameters.iter())
            .try_fold(
                HashMap::<String, String>::new(),
                |mut seen_parameter_names, (parameter, lowered_parameter)| {
                    let source_name = parameter.name.as_str().to_string();

                    if let Some(existing_parameter) = seen_parameter_names
                        .insert(lowered_parameter.name.clone(), source_name.clone())
                    {
                        return Err(PythonLowerError::ParameterNameCollision {
                            function_name: function.id.as_str().to_string(),
                            generated_name: lowered_parameter.name.clone(),
                            existing_parameter,
                            colliding_parameter: source_name,
                        });
                    }

                    Ok(seen_parameter_names)
                },
            )
            .map(|_| ())
    }

    fn lower_parameter(parameter: &ParamDef) -> Option<PythonParameter> {
        if parameter.passing != ParamPassing::Value {
            return None;
        }

        Some(PythonParameter {
            name: NamingConvention::param_name(parameter.name.as_str()),
            type_ref: Self::lower_type(&parameter.type_expr)?,
        })
    }

    fn lower_return(return_def: &ReturnDef) -> Option<PythonType> {
        match return_def {
            ReturnDef::Void => Some(PythonType::Void),
            ReturnDef::Value(type_expr) => Self::lower_type(type_expr),
            ReturnDef::Result { .. } => None,
        }
    }

    fn lower_type(type_expr: &TypeExpr) -> Option<PythonType> {
        match type_expr {
            TypeExpr::Primitive(primitive) => Some(PythonType::Primitive(*primitive)),
            TypeExpr::Void => Some(PythonType::Void),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use boltffi_ffi_rules::callable::ExecutionKind;

    use super::PythonLowerer;
    use crate::ir::definitions::{FunctionDef, ParamDef, ParamPassing, ReturnDef};
    use crate::ir::ids::{FunctionId, ParamName};
    use crate::ir::types::{PrimitiveType, TypeExpr};
    use crate::ir::{FfiContract, Lowerer, PackageInfo, TypeCatalog};
    use crate::render::python::PythonLowerError;

    fn test_function(function_name: &str, parameter_names: &[&str]) -> FunctionDef {
        FunctionDef {
            id: FunctionId::new(function_name),
            params: parameter_names
                .iter()
                .map(|parameter_name| ParamDef {
                    name: ParamName::new(*parameter_name),
                    type_expr: TypeExpr::Primitive(PrimitiveType::I32),
                    passing: ParamPassing::Value,
                    doc: None,
                })
                .collect(),
            returns: ReturnDef::Value(TypeExpr::Primitive(PrimitiveType::I32)),
            execution_kind: ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        }
    }

    fn lower_contract(
        functions: Vec<FunctionDef>,
    ) -> Result<crate::render::python::PythonModule, PythonLowerError> {
        let ffi_contract = FfiContract {
            package: PackageInfo {
                name: "demo".to_string(),
                version: Some("0.1.0".to_string()),
            },
            catalog: TypeCatalog::default(),
            functions,
        };
        let abi_contract = Lowerer::new(&ffi_contract).to_abi_contract();

        PythonLowerer::new(
            &ffi_contract,
            &abi_contract,
            "demo",
            "demo",
            Some("0.1.0".to_string()),
            "demo",
        )
        .lower()
    }

    #[test]
    fn lower_function_escapes_python_keywords() {
        let function = test_function("class", &["from"]);

        let lowered = PythonLowerer::lower_function(&function)
            .expect("function lowering should succeed")
            .expect("function should lower");

        assert_eq!(lowered.python_name, "class_");
        assert_eq!(lowered.parameters[0].name, "from_");
    }

    #[test]
    fn lower_contract_rejects_colliding_function_names() {
        let error = lower_contract(vec![
            test_function("class", &[]),
            test_function("class_", &[]),
        ])
        .expect_err("function name collision should fail");

        assert_eq!(
            error,
            PythonLowerError::TopLevelFunctionNameCollision {
                generated_name: "class_".to_string(),
                existing_function: "class".to_string(),
                colliding_function: "class_".to_string(),
            }
        );
    }

    #[test]
    fn lower_contract_rejects_colliding_parameter_names() {
        let error = lower_contract(vec![test_function("echo", &["from", "from_"])])
            .expect_err("parameter name collision should fail");

        assert_eq!(
            error,
            PythonLowerError::ParameterNameCollision {
                function_name: "echo".to_string(),
                generated_name: "from_".to_string(),
                existing_parameter: "from".to_string(),
                colliding_parameter: "from_".to_string(),
            }
        );
    }
}
