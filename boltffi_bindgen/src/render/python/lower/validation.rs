use std::collections::HashMap;

use crate::ir::definitions::ParamDef;
use crate::render::python::{
    NamingConvention, PythonCStyleEnum, PythonCStyleEnumVariant, PythonEnumConstructor,
    PythonEnumMethod, PythonFunction, PythonLowerError, PythonParameter, PythonRecord,
    PythonRecordConstructor, PythonRecordField, PythonRecordMethod,
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
        records: &[PythonRecord],
        enums: &[PythonCStyleEnum],
    ) -> Result<(), PythonLowerError> {
        let reserved_top_level_names = if enums.is_empty() {
            HashMap::new()
        } else {
            HashMap::from([(
                NamingConvention::int_enum_base_name().to_string(),
                format!(
                    "imported enum base `{}`",
                    NamingConvention::int_enum_base_name()
                ),
            )])
        };

        functions
            .iter()
            .map(|function| {
                (
                    function.python_name.clone(),
                    format!("function `{}`", function.python_name),
                )
            })
            .chain(records.iter().map(|record| {
                (
                    record.type_ref.class_name.clone(),
                    format!("record `{}`", record.type_ref.class_name),
                )
            }))
            .chain(enums.iter().map(|enumeration| {
                (
                    enumeration.type_ref.class_name.clone(),
                    format!("enum `{}`", enumeration.type_ref.class_name),
                )
            }))
            .try_fold(
                reserved_top_level_names,
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
        records: &[PythonRecord],
        enums: &[PythonCStyleEnum],
    ) -> Result<(), PythonLowerError> {
        let mut seen_native_module_names = HashMap::<String, String>::new();

        if !functions.is_empty()
            || records.iter().any(PythonRecord::has_native_callables)
            || enums.iter().any(PythonCStyleEnum::has_native_callables)
        {
            seen_native_module_names.insert(
                NamingConvention::native_loader_name().to_string(),
                format!(
                    "internal native helper `{}`",
                    NamingConvention::native_loader_name()
                ),
            );
        }

        records
            .iter()
            .map(|record| {
                (
                    record.type_ref.registration_function_name(),
                    format!(
                        "record registration helper `{}`",
                        record.type_ref.registration_function_name()
                    ),
                )
            })
            .chain(enums.iter().map(|enumeration| {
                (
                    enumeration.type_ref.registration_function_name(),
                    format!(
                        "enum registration helper `{}`",
                        enumeration.type_ref.registration_function_name()
                    ),
                )
            }))
            .chain(functions.iter().map(|function| {
                (
                    function.python_name.clone(),
                    format!("function `{}`", function.python_name),
                )
            }))
            .chain(records.iter().flat_map(|record| {
                record.constructors.iter().map(|constructor| {
                    (
                        constructor.callable.native_name.clone(),
                        format!(
                            "record constructor `{}.{}()`",
                            record.class_name(),
                            constructor.python_name
                        ),
                    )
                })
            }))
            .chain(records.iter().flat_map(|record| {
                record.methods.iter().map(|method| {
                    (
                        method.callable.native_name.clone(),
                        format!(
                            "record method `{}.{}()`",
                            record.class_name(),
                            method.python_name
                        ),
                    )
                })
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

    pub(super) fn validate_record_field_names(
        record_name: &str,
        fields: &[PythonRecordField],
    ) -> Result<(), PythonLowerError> {
        fields
            .iter()
            .try_fold(HashMap::<String, String>::new(), |mut seen_names, field| {
                if NamingConvention::is_reserved_record_field_name(&field.python_name) {
                    return Err(PythonLowerError::RecordFieldNameCollision {
                        record_name: record_name.to_string(),
                        generated_name: field.python_name.clone(),
                        existing_field: format!(
                            "reserved record field name `{}`",
                            field.python_name
                        ),
                        colliding_field: field.native_name.clone(),
                    });
                }

                if let Some(existing_field) =
                    seen_names.insert(field.python_name.clone(), field.native_name.clone())
                {
                    return Err(PythonLowerError::RecordFieldNameCollision {
                        record_name: record_name.to_string(),
                        generated_name: field.python_name.clone(),
                        existing_field,
                        colliding_field: field.native_name.clone(),
                    });
                }

                Ok(seen_names)
            })?;

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

    pub(super) fn validate_record_callable_names(
        record_name: &str,
        fields: &[PythonRecordField],
        constructors: &[PythonRecordConstructor],
        methods: &[PythonRecordMethod],
    ) -> Result<(), PythonLowerError> {
        let seen_names =
            fields
                .iter()
                .fold(HashMap::<String, String>::new(), |mut seen_names, field| {
                    seen_names.insert(
                        field.python_name.clone(),
                        format!("field `{}`", field.python_name),
                    );
                    seen_names
                });

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
            .try_fold(seen_names, |mut seen_names, (generated_name, subject)| {
                if NamingConvention::is_reserved_record_callable_name(&generated_name) {
                    return Err(PythonLowerError::RecordCallableNameCollision {
                        record_name: record_name.to_string(),
                        generated_name: generated_name.clone(),
                        existing_subject: format!(
                            "reserved record callable name `{generated_name}`"
                        ),
                        colliding_subject: subject,
                    });
                }

                if let Some(existing_subject) =
                    seen_names.insert(generated_name.clone(), subject.clone())
                {
                    return Err(PythonLowerError::RecordCallableNameCollision {
                        record_name: record_name.to_string(),
                        generated_name,
                        existing_subject,
                        colliding_subject: subject,
                    });
                }

                Ok(seen_names)
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use boltffi_ffi_rules::callable::ExecutionKind;

    use crate::ir::TypeCatalog;
    use crate::ir::definitions::{
        CStyleVariant, ConstructorDef, EnumDef, EnumRepr, FieldDef, FunctionDef, MethodDef,
        Receiver, RecordDef, ReturnDef,
    };
    use crate::ir::ids::{EnumId, FunctionId, MethodId, RecordId, VariantName};
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
            vec![FunctionDef {
                id: FunctionId::new("_initialize_loader"),
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
            vec![FunctionDef {
                id: FunctionId::new("_register_status"),
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

    #[test]
    fn reject_record_field_name_collisions() {
        let mut catalog = TypeCatalog::default();
        catalog.insert_record(RecordDef {
            id: RecordId::new("Config"),
            is_repr_c: true,
            is_error: false,
            fields: vec![
                FieldDef {
                    name: "class".into(),
                    type_expr: TypeExpr::Primitive(PrimitiveType::I32),
                    doc: None,
                    default: None,
                },
                FieldDef {
                    name: "class_".into(),
                    type_expr: TypeExpr::Primitive(PrimitiveType::I32),
                    doc: None,
                    default: None,
                },
            ],
            constructors: vec![],
            methods: vec![],
            doc: None,
            deprecated: None,
        });

        let error =
            lower_contract(catalog, vec![]).expect_err("record field collision should fail");

        assert!(matches!(
            error,
            PythonLowerError::RecordFieldNameCollision {
                generated_name,
                ..
            } if generated_name == "class_"
        ));
    }

    #[test]
    fn reject_reserved_record_field_names() {
        let mut catalog = TypeCatalog::default();
        catalog.insert_record(RecordDef {
            id: RecordId::new("Config"),
            is_repr_c: true,
            is_error: false,
            fields: vec![FieldDef {
                name: "__dict__".into(),
                type_expr: TypeExpr::Primitive(PrimitiveType::I32),
                doc: None,
                default: None,
            }],
            constructors: vec![],
            methods: vec![],
            doc: None,
            deprecated: None,
        });

        let error =
            lower_contract(catalog, vec![]).expect_err("reserved record field name should fail");

        assert!(matches!(
            error,
            PythonLowerError::RecordFieldNameCollision {
                generated_name,
                ..
            } if generated_name == "__dict__"
        ));
    }

    #[test]
    fn reject_record_method_names_that_shadow_fields() {
        let mut catalog = TypeCatalog::default();
        catalog.insert_record(RecordDef {
            id: RecordId::new("Point"),
            is_repr_c: true,
            is_error: false,
            fields: vec![FieldDef {
                name: "x".into(),
                type_expr: TypeExpr::Primitive(PrimitiveType::F64),
                doc: None,
                default: None,
            }],
            constructors: vec![],
            methods: vec![MethodDef {
                id: MethodId::new("x"),
                receiver: Receiver::RefSelf,
                params: vec![],
                returns: ReturnDef::Value(TypeExpr::Primitive(PrimitiveType::F64)),
                execution_kind: ExecutionKind::Sync,
                doc: None,
                deprecated: None,
            }],
            doc: None,
            deprecated: None,
        });

        let error =
            lower_contract(catalog, vec![]).expect_err("record callable collision should fail");

        assert!(matches!(
            error,
            PythonLowerError::RecordCallableNameCollision {
                generated_name,
                ..
            } if generated_name == "x"
        ));
    }

    #[test]
    fn reject_record_names_that_shadow_int_enum_base() {
        let mut catalog = TypeCatalog::default();
        catalog.insert_record(RecordDef {
            id: RecordId::new("IntEnum"),
            is_repr_c: true,
            is_error: false,
            fields: vec![FieldDef {
                name: "value".into(),
                type_expr: TypeExpr::Primitive(PrimitiveType::I32),
                doc: None,
                default: None,
            }],
            constructors: vec![],
            methods: vec![],
            doc: None,
            deprecated: None,
        });
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

        let error = lower_contract(catalog, vec![])
            .expect_err("reserved IntEnum top-level name should fail");

        assert!(matches!(
            error,
            PythonLowerError::TopLevelNameCollision {
                generated_name,
                ..
            } if generated_name == "IntEnum"
        ));
    }
}
