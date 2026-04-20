use boltffi_ffi_rules::naming;

use crate::ir::abi::CallId;
use crate::ir::definitions::{ConstructorDef, EnumDef, EnumRepr, MethodDef, Receiver};
use crate::ir::ids::EnumId;
use crate::ir::types::PrimitiveType;
use crate::render::python::{
    NamingConvention, PythonCStyleEnum, PythonCStyleEnumVariant, PythonCallable,
    PythonEnumConstructor, PythonEnumMethod, PythonEnumType, PythonLowerError, PythonParameter,
    PythonType,
};

use super::PythonLowerer;

impl PythonLowerer<'_> {
    pub(super) fn lower_c_style_enums(&self) -> Result<Vec<PythonCStyleEnum>, PythonLowerError> {
        self.ffi_contract.catalog.all_enums().try_fold(
            Vec::new(),
            |mut lowered_enums, enumeration| {
                if let Some(lowered_enum) = self.lower_c_style_enum(enumeration)? {
                    lowered_enums.push(lowered_enum);
                }
                Ok(lowered_enums)
            },
        )
    }

    fn lower_c_style_enum(
        &self,
        enumeration: &EnumDef,
    ) -> Result<Option<PythonCStyleEnum>, PythonLowerError> {
        let EnumRepr::CStyle { tag_type, .. } = &enumeration.repr else {
            return Ok(None);
        };

        let abi_enum = self.resolve_abi_enum(&enumeration.id);
        let type_ref = PythonEnumType {
            native_name_stem: naming::to_snake_case(enumeration.id.as_str()),
            class_name: NamingConvention::class_name(enumeration.id.as_str()),
            tag_type: *tag_type,
        };

        let variants = abi_enum
            .variants
            .iter()
            .enumerate()
            .map(|(ordinal, variant)| {
                let wire_tag = abi_enum.resolve_codec_tag(ordinal, variant.discriminant);

                PythonCStyleEnumVariant {
                    member_name: NamingConvention::enum_member_name(variant.name.as_str()),
                    native_value: variant.discriminant,
                    native_c_literal: Self::primitive_c_literal(*tag_type, variant.discriminant),
                    wire_tag,
                    wire_c_literal: Self::primitive_c_literal(PrimitiveType::I32, wire_tag),
                    doc: enumeration.variant_docs().get(ordinal).cloned().flatten(),
                }
            })
            .collect::<Vec<_>>();

        Self::validate_enum_variant_names(type_ref.class_name.as_str(), &variants)?;

        let constructors = enumeration.constructor_calls().try_fold(
            Vec::new(),
            |mut lowered_constructors, (call_id, constructor)| {
                if let Some(lowered_constructor) =
                    self.lower_enum_constructor(&enumeration.id, &type_ref, constructor, &call_id)?
                {
                    lowered_constructors.push(lowered_constructor);
                }
                Ok(lowered_constructors)
            },
        )?;

        let methods = enumeration.method_calls().try_fold(
            Vec::new(),
            |mut lowered_methods, (call_id, method)| {
                if let Some(lowered_method) =
                    self.lower_enum_method(&enumeration.id, &type_ref, method, &call_id)?
                {
                    lowered_methods.push(lowered_method);
                }
                Ok(lowered_methods)
            },
        )?;

        Self::validate_enum_callable_names(type_ref.class_name.as_str(), &constructors, &methods)?;

        Ok(Some(PythonCStyleEnum {
            type_ref,
            variants,
            constructors,
            methods,
        }))
    }

    fn lower_enum_constructor(
        &self,
        enum_id: &EnumId,
        enum_type: &PythonEnumType,
        constructor: &ConstructorDef,
        call_id: &CallId,
    ) -> Result<Option<PythonEnumConstructor>, PythonLowerError> {
        if constructor.is_fallible() || constructor.is_optional() {
            return Ok(None);
        }

        let public_name = constructor
            .name()
            .map(|name| NamingConvention::method_name(name.as_str()))
            .unwrap_or_else(|| "new".to_string());

        let callable_name = format!(
            "enum constructor `{}.{}()`",
            enum_type.class_name, public_name
        );

        let Some(parameters) = self.lower_parameters(&callable_name, constructor.params())? else {
            return Ok(None);
        };

        let ffi_symbol = self.resolve_call_symbol(call_id);

        Ok(Some(PythonEnumConstructor {
            python_name: public_name.clone(),
            callable: PythonCallable {
                native_name: NamingConvention::native_member_name(enum_id.as_str(), &public_name),
                ffi_symbol,
                parameters,
                return_type: PythonType::CStyleEnum(enum_type.clone()),
            },
        }))
    }

    fn lower_enum_method(
        &self,
        enum_id: &EnumId,
        enum_type: &PythonEnumType,
        method: &MethodDef,
        call_id: &CallId,
    ) -> Result<Option<PythonEnumMethod>, PythonLowerError> {
        if method.is_async() {
            return Ok(None);
        }

        let public_name = NamingConvention::method_name(method.id.as_str());
        let callable_name = format!("enum method `{}.{}()`", enum_type.class_name, public_name);
        let Some(mut parameters) = self.lower_parameters(&callable_name, &method.params)? else {
            return Ok(None);
        };

        let Some(return_type) = self.lower_return(&method.returns) else {
            return Ok(None);
        };

        let is_static = method.receiver == Receiver::Static;
        if !is_static {
            parameters.insert(
                0,
                PythonParameter {
                    name: "self".to_string(),
                    type_ref: PythonType::CStyleEnum(enum_type.clone()),
                },
            );
        }

        let ffi_symbol = self.resolve_call_symbol(call_id);

        Ok(Some(PythonEnumMethod {
            python_name: public_name.clone(),
            callable: PythonCallable {
                native_name: NamingConvention::native_member_name(enum_id.as_str(), &public_name),
                ffi_symbol,
                parameters,
                return_type,
            },
            is_static,
        }))
    }
}

#[cfg(test)]
mod tests {
    use boltffi_ffi_rules::callable::ExecutionKind;

    use crate::ir::TypeCatalog;
    use crate::ir::definitions::{
        CStyleVariant, ConstructorDef, EnumDef, EnumRepr, FunctionDef, MethodDef, ParamDef,
        ParamPassing, Receiver, ReturnDef,
    };
    use crate::ir::ids::{EnumId, FunctionId, MethodId, ParamName, VariantName};
    use crate::ir::types::{PrimitiveType, TypeExpr};
    use crate::render::python::{PythonSequenceType, PythonType};

    use super::super::test_support::lower_contract;

    #[test]
    fn lower_contract_supports_c_style_enums_and_enum_vectors() {
        let mut catalog = TypeCatalog::default();
        catalog.insert_enum(EnumDef {
            id: EnumId::new("status"),
            repr: EnumRepr::CStyle {
                tag_type: PrimitiveType::I32,
                variants: vec![
                    CStyleVariant {
                        name: VariantName::new("Active"),
                        discriminant: 0,
                        doc: None,
                    },
                    CStyleVariant {
                        name: VariantName::new("Inactive"),
                        discriminant: 1,
                        doc: None,
                    },
                ],
            },
            is_error: false,
            constructors: vec![],
            methods: vec![],
            doc: None,
            deprecated: None,
        });

        let module = lower_contract(
            catalog,
            vec![
                FunctionDef {
                    id: FunctionId::new("echo_status"),
                    params: vec![ParamDef {
                        name: ParamName::new("value"),
                        type_expr: TypeExpr::Enum(EnumId::new("status")),
                        passing: ParamPassing::Value,
                        doc: None,
                    }],
                    returns: ReturnDef::Value(TypeExpr::Enum(EnumId::new("status"))),
                    execution_kind: ExecutionKind::Sync,
                    doc: None,
                    deprecated: None,
                },
                FunctionDef {
                    id: FunctionId::new("echo_vec_status"),
                    params: vec![ParamDef {
                        name: ParamName::new("values"),
                        type_expr: TypeExpr::Vec(Box::new(TypeExpr::Enum(EnumId::new("status")))),
                        passing: ParamPassing::Value,
                        doc: None,
                    }],
                    returns: ReturnDef::Value(TypeExpr::Vec(Box::new(TypeExpr::Enum(
                        EnumId::new("status"),
                    )))),
                    execution_kind: ExecutionKind::Sync,
                    doc: None,
                    deprecated: None,
                },
            ],
        )
        .expect("python lowering should succeed");

        assert_eq!(module.enums.len(), 1);
        assert_eq!(module.enums[0].type_ref.class_name, "Status");
        assert_eq!(module.enums[0].variants[0].member_name, "ACTIVE");
        assert!(matches!(
            module.functions[0].callable.parameters[0].type_ref,
            PythonType::CStyleEnum(_)
        ));
        assert!(matches!(
            module.functions[1].callable.parameters[0].type_ref,
            PythonType::Sequence(PythonSequenceType::CStyleEnumVec(_))
        ));
    }

    #[test]
    fn lower_c_style_enum_methods_and_constructors() {
        let mut catalog = TypeCatalog::default();
        catalog.insert_enum(EnumDef {
            id: EnumId::new("direction"),
            repr: EnumRepr::CStyle {
                tag_type: PrimitiveType::I32,
                variants: vec![
                    CStyleVariant {
                        name: VariantName::new("North"),
                        discriminant: 0,
                        doc: None,
                    },
                    CStyleVariant {
                        name: VariantName::new("South"),
                        discriminant: 1,
                        doc: None,
                    },
                ],
            },
            is_error: false,
            constructors: vec![ConstructorDef::NamedInit {
                name: MethodId::new("new"),
                first_param: ParamDef {
                    name: ParamName::new("raw"),
                    type_expr: TypeExpr::Primitive(PrimitiveType::I32),
                    passing: ParamPassing::Value,
                    doc: None,
                },
                rest_params: vec![],
                is_fallible: false,
                is_optional: false,
                doc: None,
                deprecated: None,
            }],
            methods: vec![
                MethodDef {
                    id: MethodId::new("cardinal"),
                    receiver: Receiver::Static,
                    params: vec![],
                    returns: ReturnDef::Value(TypeExpr::Enum(EnumId::new("direction"))),
                    execution_kind: ExecutionKind::Sync,
                    doc: None,
                    deprecated: None,
                },
                MethodDef {
                    id: MethodId::new("opposite"),
                    receiver: Receiver::RefSelf,
                    params: vec![],
                    returns: ReturnDef::Value(TypeExpr::Enum(EnumId::new("direction"))),
                    execution_kind: ExecutionKind::Sync,
                    doc: None,
                    deprecated: None,
                },
            ],
            doc: None,
            deprecated: None,
        });

        let module = lower_contract(catalog, vec![]).expect("python lowering should succeed");
        let enumeration = &module.enums[0];

        assert_eq!(enumeration.constructors[0].python_name, "new");
        assert_eq!(
            enumeration.constructors[0].callable.native_name,
            "_boltffi_direction_new"
        );
        assert_eq!(enumeration.methods[0].python_name, "cardinal");
        assert!(enumeration.methods[0].is_static);
        assert_eq!(enumeration.methods[1].python_name, "opposite");
        assert!(!enumeration.methods[1].is_static);
        assert_eq!(enumeration.methods[1].callable.parameters[0].name, "self");
    }
}
