use crate::ir::types::PrimitiveType;

use super::{PythonEnumType, PythonRecordType, PythonSequenceType, PythonType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonParameter {
    pub name: String,
    pub type_ref: PythonType,
}

impl PythonParameter {
    pub fn value_binding_name(&self) -> String {
        format!("native_{}", self.name)
    }

    pub fn is_string(&self) -> bool {
        self.type_ref.is_string()
    }

    pub fn uses_buffer_input(&self) -> bool {
        self.type_ref.uses_buffer_input()
    }

    pub fn is_c_style_enum(&self) -> bool {
        self.type_ref.is_c_style_enum()
    }

    pub fn parser_state_name(&self) -> String {
        format!("{}_input", self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonCallable {
    pub native_name: String,
    pub ffi_symbol: String,
    pub parameters: Vec<PythonParameter>,
    pub return_type: PythonType,
}

impl PythonCallable {
    pub fn parameter_count(&self) -> usize {
        self.parameters.len()
    }

    pub fn takes_no_parameters(&self) -> bool {
        self.parameters.is_empty()
    }

    pub fn returns_void(&self) -> bool {
        self.return_type.is_void()
    }

    pub fn returns_string(&self) -> bool {
        self.return_type.is_string()
    }

    pub fn returns_record(&self) -> bool {
        self.return_type.is_record()
    }

    pub fn returns_c_style_enum(&self) -> bool {
        self.return_type.is_c_style_enum()
    }

    pub fn returns_bytes(&self) -> bool {
        self.return_type.is_byte_like()
    }

    pub fn returns_primitive_vector(&self) -> bool {
        self.return_type.is_primitive_vector()
    }

    pub fn returns_c_style_enum_vector(&self) -> bool {
        self.return_type.is_c_style_enum_vector()
    }

    pub fn return_primitive(&self) -> Option<PrimitiveType> {
        match &self.return_type {
            PythonType::Primitive(primitive) => Some(*primitive),
            _ => None,
        }
    }

    pub fn return_c_style_enum(&self) -> Option<&PythonEnumType> {
        self.return_type.c_style_enum()
    }

    pub fn return_record(&self) -> Option<&PythonRecordType> {
        self.return_type.record()
    }

    pub fn return_vector_primitive(&self) -> Option<PrimitiveType> {
        match &self.return_type {
            PythonType::Sequence(PythonSequenceType::PrimitiveVec(primitive)) => Some(*primitive),
            _ => None,
        }
    }

    pub fn return_vector_c_style_enum(&self) -> Option<&PythonEnumType> {
        self.return_type.sequence_c_style_enum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonFunction {
    pub python_name: String,
    pub callable: PythonCallable,
}

impl PythonFunction {
    pub fn callable(&self) -> &PythonCallable {
        &self.callable
    }

    pub fn native_callable(&self) -> PythonNativeCallable<'_> {
        PythonNativeCallable {
            module_attribute_name: self.python_name.as_str(),
            callable: &self.callable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonEnumConstructor {
    pub python_name: String,
    pub callable: PythonCallable,
}

impl PythonEnumConstructor {
    pub fn callable(&self) -> &PythonCallable {
        &self.callable
    }

    pub fn native_callable(&self) -> PythonNativeCallable<'_> {
        PythonNativeCallable {
            module_attribute_name: self.callable.native_name.as_str(),
            callable: &self.callable,
        }
    }

    pub fn parameter_annotation_signature(&self) -> String {
        self.callable
            .parameters
            .iter()
            .map(|parameter| {
                format!(
                    "{}: {}",
                    parameter.name,
                    parameter.type_ref.parameter_annotation()
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonEnumMethod {
    pub python_name: String,
    pub callable: PythonCallable,
    pub is_static: bool,
}

impl PythonEnumMethod {
    pub fn callable(&self) -> &PythonCallable {
        &self.callable
    }

    pub fn native_callable(&self) -> PythonNativeCallable<'_> {
        PythonNativeCallable {
            module_attribute_name: self.callable.native_name.as_str(),
            callable: &self.callable,
        }
    }

    pub fn public_parameters(&self) -> &[PythonParameter] {
        if self.is_static {
            &self.callable.parameters
        } else {
            &self.callable.parameters[1..]
        }
    }

    pub fn parameter_annotation_signature(&self) -> String {
        self.public_parameters()
            .iter()
            .map(|parameter| {
                format!(
                    "{}: {}",
                    parameter.name,
                    parameter.type_ref.parameter_annotation()
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonNativeCallable<'a> {
    pub module_attribute_name: &'a str,
    pub callable: &'a PythonCallable,
}
