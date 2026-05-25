use crate::ir::abi::CallId;
use crate::ir::definitions::{ConstructorDef, FieldDef, MethodDef, Receiver, RecordDef, ReturnDef};
use crate::ir::types::TypeExpr;
use crate::render::python::{
    NamingConvention, PythonCallable, PythonLowerError, PythonParameter, PythonRecord,
    PythonRecordConstructor, PythonRecordField, PythonRecordMethod, PythonRecordType, PythonType,
};

use super::PythonLowerer;

impl PythonLowerer<'_> {
    pub(super) fn lower_records(&self) -> Result<Vec<PythonRecord>, PythonLowerError> {
        self.ffi_contract.catalog.all_records().try_fold(
            Vec::new(),
            |mut lowered_records, record| {
                if let Some(lowered_record) = self.lower_record(record)? {
                    lowered_records.push(lowered_record);
                }
                Ok(lowered_records)
            },
        )
    }

    fn lower_record(&self, record: &RecordDef) -> Result<Option<PythonRecord>, PythonLowerError> {
        let Some(type_ref) = self.lower_blittable_record_type(&record.id) else {
            return Ok(None);
        };

        let fields = record
            .fields
            .iter()
            .map(Self::lower_record_field)
            .collect::<Vec<_>>();

        Self::validate_record_field_names(type_ref.class_name.as_str(), &fields)?;

        let constructors = record.constructor_calls().try_fold(
            Vec::new(),
            |mut lowered_constructors, (call_id, constructor)| {
                if let Some(lowered_constructor) =
                    self.lower_record_constructor(record, &type_ref, constructor, &call_id)?
                {
                    lowered_constructors.push(lowered_constructor);
                }
                Ok(lowered_constructors)
            },
        )?;

        let methods = record.method_calls().try_fold(
            Vec::new(),
            |mut lowered_methods, (call_id, method)| {
                if let Some(lowered_method) =
                    self.lower_record_method(record, &type_ref, method, &call_id)?
                {
                    lowered_methods.push(lowered_method);
                }
                Ok(lowered_methods)
            },
        )?;

        Self::validate_record_callable_names(
            type_ref.class_name.as_str(),
            &fields,
            &constructors,
            &methods,
        )?;

        Ok(Some(
            PythonRecord::new(type_ref, fields, constructors, methods)
                .expect("direct python record should always have at least one field"),
        ))
    }

    fn lower_record_field(field: &FieldDef) -> PythonRecordField {
        let TypeExpr::Primitive(primitive) = &field.type_expr else {
            unreachable!("blittable python records must contain only primitive fields");
        };

        PythonRecordField {
            python_name: NamingConvention::record_field_name(field.name.as_str()),
            native_name: field.name.as_str().to_string(),
            primitive: *primitive,
        }
    }

    fn lower_record_constructor(
        &self,
        record: &RecordDef,
        record_type: &PythonRecordType,
        constructor: &ConstructorDef,
        call_id: &CallId,
    ) -> Result<Option<PythonRecordConstructor>, PythonLowerError> {
        if constructor.is_fallible() || constructor.is_optional() {
            return Ok(None);
        }

        let public_name = constructor
            .name()
            .map(|name| NamingConvention::method_name(name.as_str()))
            .unwrap_or_else(|| "new".to_string());

        let callable_name = format!(
            "record constructor `{}.{}()`",
            record_type.class_name, public_name
        );

        let Some(parameters) = self.lower_parameters(&callable_name, constructor.params())? else {
            return Ok(None);
        };

        Ok(Some(PythonRecordConstructor {
            python_name: public_name.clone(),
            callable: PythonCallable {
                native_name: NamingConvention::native_member_name(record.id.as_str(), &public_name),
                ffi_symbol: self.resolve_call_symbol(call_id),
                parameters,
                return_type: PythonType::Record(record_type.clone()),
            },
        }))
    }

    fn lower_record_method(
        &self,
        record: &RecordDef,
        record_type: &PythonRecordType,
        method: &MethodDef,
        call_id: &CallId,
    ) -> Result<Option<PythonRecordMethod>, PythonLowerError> {
        if method.is_async() {
            return Ok(None);
        }

        let public_name = NamingConvention::method_name(method.id.as_str());
        let callable_name = format!(
            "record method `{}.{}()`",
            record_type.class_name, public_name
        );
        let Some(mut parameters) = self.lower_parameters(&callable_name, &method.params)? else {
            return Ok(None);
        };

        let return_type = if method.receiver == Receiver::RefMutSelf
            && matches!(method.returns, ReturnDef::Void)
        {
            PythonType::Record(record_type.clone())
        } else {
            let Some(lowered_return_type) = self.lower_return(&method.returns) else {
                return Ok(None);
            };
            lowered_return_type
        };

        let is_static = method.receiver == Receiver::Static;
        if !is_static {
            parameters.insert(
                0,
                PythonParameter {
                    name: "self".to_string(),
                    type_ref: PythonType::Record(record_type.clone()),
                },
            );
        }

        Ok(Some(PythonRecordMethod {
            python_name: public_name.clone(),
            callable: PythonCallable {
                native_name: NamingConvention::native_member_name(record.id.as_str(), &public_name),
                ffi_symbol: self.resolve_call_symbol(call_id),
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
        ConstructorDef, FieldDef, MethodDef, ParamDef, ParamPassing, Receiver, RecordDef, ReturnDef,
    };
    use crate::ir::ids::{MethodId, ParamName, RecordId};
    use crate::ir::types::{PrimitiveType, TypeExpr};
    use crate::render::python::PythonType;

    use super::super::test_support::lower_contract;

    fn point_record() -> RecordDef {
        RecordDef {
            id: RecordId::new("Point"),
            is_repr_c: true,
            is_error: false,
            fields: vec![
                FieldDef {
                    name: "x".into(),
                    type_expr: TypeExpr::Primitive(PrimitiveType::F64),
                    doc: None,
                    default: None,
                },
                FieldDef {
                    name: "y".into(),
                    type_expr: TypeExpr::Primitive(PrimitiveType::F64),
                    doc: None,
                    default: None,
                },
            ],
            constructors: vec![
                ConstructorDef::Default {
                    params: vec![
                        ParamDef {
                            name: ParamName::new("x"),
                            type_expr: TypeExpr::Primitive(PrimitiveType::F64),
                            passing: ParamPassing::Value,
                            doc: None,
                        },
                        ParamDef {
                            name: ParamName::new("y"),
                            type_expr: TypeExpr::Primitive(PrimitiveType::F64),
                            passing: ParamPassing::Value,
                            doc: None,
                        },
                    ],
                    is_fallible: false,
                    is_optional: false,
                    doc: None,
                    deprecated: None,
                },
                ConstructorDef::NamedFactory {
                    name: MethodId::new("origin"),
                    is_fallible: false,
                    is_optional: false,
                    doc: None,
                    deprecated: None,
                },
            ],
            methods: vec![
                MethodDef {
                    id: MethodId::new("distance"),
                    receiver: Receiver::RefSelf,
                    params: vec![],
                    returns: ReturnDef::Value(TypeExpr::Primitive(PrimitiveType::F64)),
                    execution_kind: ExecutionKind::Sync,
                    doc: None,
                    deprecated: None,
                },
                MethodDef {
                    id: MethodId::new("scale"),
                    receiver: Receiver::RefMutSelf,
                    params: vec![ParamDef {
                        name: ParamName::new("factor"),
                        type_expr: TypeExpr::Primitive(PrimitiveType::F64),
                        passing: ParamPassing::Value,
                        doc: None,
                    }],
                    returns: ReturnDef::Void,
                    execution_kind: ExecutionKind::Sync,
                    doc: None,
                    deprecated: None,
                },
            ],
            doc: None,
            deprecated: None,
        }
    }

    #[test]
    fn lower_contract_supports_blittable_records() {
        let mut catalog = TypeCatalog::default();
        catalog.insert_record(point_record());

        let module = lower_contract(catalog, vec![]).expect("python lowering should succeed");

        assert_eq!(module.records.len(), 1);
        assert_eq!(module.records[0].type_ref.class_name, "Point");
        assert_eq!(module.records[0].fields.first().python_name, "x");
        assert_eq!(module.records[0].fields.first().native_name, "x");
        assert_eq!(
            module.records[0].constructors[0].callable.return_type,
            PythonType::Record(module.records[0].type_ref.clone())
        );
        assert_eq!(
            module.records[0].methods[1].callable.return_type,
            PythonType::Record(module.records[0].type_ref.clone())
        );
    }
}
