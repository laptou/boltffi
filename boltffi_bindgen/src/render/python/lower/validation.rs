use std::collections::HashMap;

use crate::ir::definitions::ParamDef;
use crate::render::python::{
    NamingConvention, PythonCStyleEnum, PythonCStyleEnumVariant, PythonEnumConstructor,
    PythonEnumMethod, PythonFunction, PythonLowerError, PythonParameter,
};

use super::PythonLowerer;

impl PythonLowerer<'_> {
    pub(super) fn validate_parameter_names(
        callable_name: &str,
        source_parameters: &[&ParamDef],
        lowered_parameters: &[PythonParameter],
    ) -> Result<(), PythonLowerError> {
        source_parameters
            .iter()
            .zip(lowered_parameters.iter())
            .try_fold(
                HashMap::<String, String>::new(),
                |mut seen_parameter_names, (source_parameter, lowered_parameter)| {
                    let source_name = source_parameter.name.as_str().to_string();

                    if let Some(existing_parameter) = seen_parameter_names
                        .insert(lowered_parameter.name.clone(), source_name.clone())
                    {
                        return Err(PythonLowerError::ParameterNameCollision {
                            callable_name: callable_name.to_string(),
                            generated_name: lowered_parameter.name.clone(),
                            existing_parameter,
                            colliding_parameter: source_name,
                        });
                    }

                    Ok(seen_parameter_names)
                },
            )?;

        Ok(())
    }

    pub(super) fn validate_top_level_names(
        functions: &[PythonFunction],
        enums: &[PythonCStyleEnum],
    ) -> Result<(), PythonLowerError> {
        functions
            .iter()
            .map(|function| {
                (
                    function.python_name.clone(),
                    format!("function `{}`", function.python_name),
                )
            })
            .chain(enums.iter().map(|enumeration| {
                (
                    enumeration.type_ref.class_name.clone(),
                    format!("enum `{}`", enumeration.type_ref.class_name),
                )
            }))
            .try_fold(
                HashMap::<String, String>::new(),
                |mut seen_names, (generated_name, subject)| {
                    if let Some(existing_subject) =
                        seen_names.insert(generated_name.clone(), subject.clone())
                    {
                        return Err(PythonLowerError::TopLevelNameCollision {
                            generated_name,
                            existing_subject,
                            colliding_subject: subject,
                        });
                    }

                    Ok(seen_names)
                },
            )?;

        Ok(())
    }

    pub(super) fn validate_native_module_names(
        functions: &[PythonFunction],
        enums: &[PythonCStyleEnum],
    ) -> Result<(), PythonLowerError> {
        let mut seen_native_module_names = HashMap::<String, String>::new();

        if !functions.is_empty() || enums.iter().any(PythonCStyleEnum::has_native_callables) {
            seen_native_module_names.insert(
                NamingConvention::native_loader_name().to_string(),
                format!(
                    "internal native helper `{}`",
                    NamingConvention::native_loader_name()
                ),
            );
        }

        enums
            .iter()
            .map(|enumeration| {
                (
                    enumeration.type_ref.registration_function_name(),
                    format!(
                        "enum registration helper `{}`",
                        enumeration.type_ref.registration_function_name()
                    ),
                )
            })
            .chain(functions.iter().map(|function| {
                (
                    function.python_name.clone(),
                    format!("function `{}`", function.python_name),
                )
            }))
            .chain(enums.iter().flat_map(|enumeration| {
                enumeration.constructors.iter().map(|constructor| {
                    (
                        constructor.callable.native_name.clone(),
                        format!(
                            "enum constructor `{}.{}()`",
                            enumeration.class_name(),
                            constructor.python_name
                        ),
                    )
                })
            }))
            .chain(enums.iter().flat_map(|enumeration| {
                enumeration.methods.iter().map(|method| {
                    (
                        method.callable.native_name.clone(),
                        format!(
                            "enum method `{}.{}()`",
                            enumeration.class_name(),
                            method.python_name
                        ),
                    )
                })
            }))
            .try_fold(
                seen_native_module_names,
                |mut seen_names, (generated_name, subject)| {
                    if let Some(existing_subject) =
                        seen_names.insert(generated_name.clone(), subject.clone())
                    {
                        return Err(PythonLowerError::NativeModuleNameCollision {
                            generated_name,
                            existing_subject,
                            colliding_subject: subject,
                        });
                    }

                    Ok(seen_names)
                },
            )?;

        Ok(())
    }

    pub(super) fn validate_enum_variant_names(
        enum_name: &str,
        variants: &[PythonCStyleEnumVariant],
    ) -> Result<(), PythonLowerError> {
        variants
            .iter()
            .try_fold(HashMap::<String, i128>::new(), |mut seen_names, variant| {
                if let Some(existing_variant) =
                    seen_names.insert(variant.member_name.clone(), variant.native_value)
                {
                    return Err(PythonLowerError::EnumMemberNameCollision {
                        enum_name: enum_name.to_string(),
                        generated_name: variant.member_name.clone(),
                        existing_variant: existing_variant.to_string(),
                        colliding_variant: variant.native_value.to_string(),
                    });
                }

                Ok(seen_names)
            })?;

        Ok(())
    }

    pub(super) fn validate_enum_callable_names(
        enum_name: &str,
        constructors: &[PythonEnumConstructor],
        methods: &[PythonEnumMethod],
    ) -> Result<(), PythonLowerError> {
        constructors
            .iter()
            .map(|constructor| {
                (
                    constructor.python_name.clone(),
                    format!("constructor `{}`", constructor.python_name),
                )
            })
            .chain(methods.iter().map(|method| {
                (
                    method.python_name.clone(),
                    format!("method `{}`", method.python_name),
                )
            }))
            .try_fold(
                HashMap::<String, String>::new(),
                |mut seen_names, (generated_name, subject)| {
                    if NamingConvention::is_reserved_int_enum_callable_name(&generated_name) {
                        return Err(PythonLowerError::EnumCallableNameCollision {
                            enum_name: enum_name.to_string(),
                            generated_name: generated_name.clone(),
                            existing_subject: format!(
                                "reserved IntEnum callable name `{generated_name}`"
                            ),
                            colliding_subject: subject,
                        });
                    }

                    if let Some(existing_subject) =
                        seen_names.insert(generated_name.clone(), subject.clone())
                    {
                        return Err(PythonLowerError::EnumCallableNameCollision {
                            enum_name: enum_name.to_string(),
                            generated_name,
                            existing_subject,
                            colliding_subject: subject,
                        });
                    }

                    Ok(seen_names)
                },
            )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use boltffi_ffi_rules::callable::ExecutionKind;

    use crate::ir::TypeCatalog;
    use crate::ir::definitions::{
        CStyleVariant, ConstructorDef, EnumDef, EnumRepr, MethodDef, Receiver, ReturnDef,
    };
    use crate::ir::ids::{EnumId, MethodId, VariantName};
    use crate::ir::types::{PrimitiveType, TypeExpr};
    use crate::render::python::PythonLowerError;

    use super::super::test_support::lower_contract;

    fn test_enum_with_reserved_callable_name(method_name: &str) -> EnumDef {
        EnumDef {
            id: EnumId::new("status"),
            repr: EnumRepr::CStyle {
                tag_type: PrimitiveType::I32,
                variants: vec![CStyleVariant {
                    name: VariantName::new("Active"),
                    discriminant: 0,
                    doc: None,
                }],
            },
            is_error: false,
            constructors: vec![ConstructorDef::NamedFactory {
                name: MethodId::new(method_name),
                is_fallible: false,
                is_optional: false,
                doc: None,
                deprecated: None,
            }],
            methods: vec![MethodDef {
                id: MethodId::new(method_name),
                receiver: Receiver::Static,
                params: vec![],
                returns: ReturnDef::Value(TypeExpr::Enum(EnumId::new("status"))),
                execution_kind: ExecutionKind::Sync,
                doc: None,
                deprecated: None,
            }],
            doc: None,
            deprecated: None,
        }
    }

    #[test]
    fn reject_reserved_int_enum_member_names_for_constructors_and_methods() {
        let mut catalog = TypeCatalog::default();
        catalog.insert_enum(test_enum_with_reserved_callable_name("name"));

        let error =
            lower_contract(catalog, vec![]).expect_err("reserved enum callable should fail");

        assert!(matches!(
            error,
            PythonLowerError::EnumCallableNameCollision {
                generated_name,
                ..
            } if generated_name == "name"
        ));
    }

    #[test]
    fn reject_reserved_int_enum_hook_names_for_constructors_and_methods() {
        let mut catalog = TypeCatalog::default();
        catalog.insert_enum(test_enum_with_reserved_callable_name("__new__"));

        let error = lower_contract(catalog, vec![]).expect_err("reserved enum hook should fail");

        assert!(matches!(
            error,
            PythonLowerError::EnumCallableNameCollision {
                generated_name,
                ..
            } if generated_name == "__new__"
        ));
    }

    #[test]
    fn reject_native_loader_name_for_functions() {
        let error = lower_contract(
            TypeCatalog::default(),
            vec![crate::ir::definitions::FunctionDef {
                id: crate::ir::ids::FunctionId::new("_initialize_loader"),
                params: vec![],
                returns: ReturnDef::Void,
                execution_kind: ExecutionKind::Sync,
                doc: None,
                deprecated: None,
            }],
        )
        .expect_err("native loader name should fail");

        assert!(matches!(
            error,
            PythonLowerError::NativeModuleNameCollision {
                generated_name,
                ..
            } if generated_name == "_initialize_loader"
        ));
    }

    #[test]
    fn reject_enum_registration_names_for_functions() {
        let mut catalog = TypeCatalog::default();
        catalog.insert_enum(EnumDef {
            id: EnumId::new("status"),
            repr: EnumRepr::CStyle {
                tag_type: PrimitiveType::I32,
                variants: vec![CStyleVariant {
                    name: VariantName::new("Active"),
                    discriminant: 0,
                    doc: None,
                }],
            },
            is_error: false,
            constructors: vec![],
            methods: vec![],
            doc: None,
            deprecated: None,
        });

        let error = lower_contract(
            catalog,
            vec![crate::ir::definitions::FunctionDef {
                id: crate::ir::ids::FunctionId::new("_register_status"),
                params: vec![],
                returns: ReturnDef::Void,
                execution_kind: ExecutionKind::Sync,
                doc: None,
                deprecated: None,
            }],
        )
        .expect_err("enum registration name should fail");

        assert!(matches!(
            error,
            PythonLowerError::NativeModuleNameCollision {
                generated_name,
                ..
            } if generated_name == "_register_status"
        ));
    }
}
