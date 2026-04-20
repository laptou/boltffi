use crate::ir::definitions::{ParamDef, ParamPassing, ReturnDef};
use crate::ir::ids::EnumId;
use crate::ir::types::TypeExpr;
use crate::render::python::{
    NamingConvention, PythonEnumType, PythonLowerError, PythonParameter, PythonSequenceType,
    PythonType,
};

use super::PythonLowerer;

impl PythonLowerer<'_> {
    pub(super) fn lower_parameters<'parameter>(
        &self,
        callable_name: &str,
        parameters: impl IntoIterator<Item = &'parameter ParamDef>,
    ) -> Result<Option<Vec<PythonParameter>>, PythonLowerError> {
        let source_parameters = parameters.into_iter().collect::<Vec<_>>();

        let Some(lowered_parameters) = source_parameters
            .iter()
            .copied()
            .map(|parameter| self.lower_parameter(parameter))
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };

        Self::validate_parameter_names(callable_name, &source_parameters, &lowered_parameters)?;

        Ok(Some(lowered_parameters))
    }

    fn lower_parameter(&self, parameter: &ParamDef) -> Option<PythonParameter> {
        if parameter.passing != ParamPassing::Value {
            return None;
        }

        Some(PythonParameter {
            name: NamingConvention::param_name(parameter.name.as_str()),
            type_ref: self.lower_type(&parameter.type_expr)?,
        })
    }

    pub(super) fn lower_return(&self, return_def: &ReturnDef) -> Option<PythonType> {
        match return_def {
            ReturnDef::Void => Some(PythonType::Void),
            ReturnDef::Value(type_expr) => self.lower_type(type_expr),
            ReturnDef::Result { .. } => None,
        }
    }

    fn lower_type(&self, type_expr: &TypeExpr) -> Option<PythonType> {
        match type_expr {
            TypeExpr::Primitive(primitive) => Some(PythonType::Primitive(*primitive)),
            TypeExpr::Enum(enum_id) => self
                .lower_c_style_enum_type(enum_id)
                .map(PythonType::CStyleEnum),
            TypeExpr::String => Some(PythonType::String),
            TypeExpr::Bytes => Some(PythonType::Sequence(PythonSequenceType::Bytes)),
            TypeExpr::Vec(inner) => self.lower_vector(inner),
            TypeExpr::Void => Some(PythonType::Void),
            _ => None,
        }
    }

    fn lower_vector(&self, element_type: &TypeExpr) -> Option<PythonType> {
        match element_type {
            TypeExpr::Primitive(primitive) => Some(PythonType::Sequence(
                PythonSequenceType::PrimitiveVec(*primitive),
            )),
            TypeExpr::Enum(enum_id) => self
                .lower_c_style_enum_type(enum_id)
                .map(PythonSequenceType::CStyleEnumVec)
                .map(PythonType::Sequence),
            _ => None,
        }
    }

    fn lower_c_style_enum_type(&self, enum_id: &EnumId) -> Option<PythonEnumType> {
        let enumeration = self.ffi_contract.catalog.resolve_enum(enum_id)?;
        let crate::ir::definitions::EnumRepr::CStyle { tag_type, .. } = &enumeration.repr else {
            return None;
        };

        Some(PythonEnumType {
            native_name_stem: boltffi_ffi_rules::naming::to_snake_case(enum_id.as_str()),
            class_name: NamingConvention::class_name(enum_id.as_str()),
            tag_type: *tag_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use boltffi_ffi_rules::callable::ExecutionKind;

    use crate::ir::TypeCatalog;
    use crate::ir::definitions::{FunctionDef, ParamDef, ParamPassing, ReturnDef};
    use crate::ir::ids::{FunctionId, ParamName};
    use crate::ir::types::{PrimitiveType, TypeExpr};
    use crate::render::python::{PythonSequenceType, PythonType};

    use super::super::test_support::lower_contract;

    #[test]
    fn lower_function_supports_string_parameters_and_returns() {
        let function = FunctionDef {
            id: FunctionId::new("echo_string"),
            params: vec![ParamDef {
                name: ParamName::new("value"),
                type_expr: TypeExpr::String,
                passing: ParamPassing::Value,
                doc: None,
            }],
            returns: ReturnDef::Value(TypeExpr::String),
            execution_kind: ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        };

        let module = lower_contract(TypeCatalog::default(), vec![function])
            .expect("python lowering should succeed");
        let lowered = &module.functions[0];

        assert_eq!(lowered.callable.parameters[0].type_ref, PythonType::String);
        assert_eq!(lowered.callable.return_type, PythonType::String);
    }

    #[test]
    fn lower_function_supports_bytes_parameters_and_returns() {
        let function = FunctionDef {
            id: FunctionId::new("echo_bytes"),
            params: vec![ParamDef {
                name: ParamName::new("value"),
                type_expr: TypeExpr::Bytes,
                passing: ParamPassing::Value,
                doc: None,
            }],
            returns: ReturnDef::Value(TypeExpr::Bytes),
            execution_kind: ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        };

        let module = lower_contract(TypeCatalog::default(), vec![function])
            .expect("python lowering should succeed");
        let lowered = &module.functions[0];

        assert_eq!(
            lowered.callable.parameters[0].type_ref,
            PythonType::Sequence(PythonSequenceType::Bytes)
        );
        assert_eq!(
            lowered.callable.return_type,
            PythonType::Sequence(PythonSequenceType::Bytes)
        );
    }

    #[test]
    fn lower_function_supports_primitive_vector_parameters_and_returns() {
        let function = FunctionDef {
            id: FunctionId::new("echo_vec_i32"),
            params: vec![ParamDef {
                name: ParamName::new("values"),
                type_expr: TypeExpr::Vec(Box::new(TypeExpr::Primitive(PrimitiveType::I32))),
                passing: ParamPassing::Value,
                doc: None,
            }],
            returns: ReturnDef::Value(TypeExpr::Vec(Box::new(TypeExpr::Primitive(
                PrimitiveType::I32,
            )))),
            execution_kind: ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        };

        let module = lower_contract(TypeCatalog::default(), vec![function])
            .expect("python lowering should succeed");
        let lowered = &module.functions[0];

        assert_eq!(
            lowered.callable.parameters[0].type_ref,
            PythonType::Sequence(PythonSequenceType::PrimitiveVec(PrimitiveType::I32))
        );
        assert_eq!(
            lowered.callable.return_type,
            PythonType::Sequence(PythonSequenceType::PrimitiveVec(PrimitiveType::I32))
        );
    }
}
