use crate::ir::types::PrimitiveType;

use super::{PythonCallable, PythonEnumConstructor, PythonEnumMethod, PythonNativeCallable};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonEnumType {
    pub native_name_stem: String,
    pub class_name: String,
    pub tag_type: PrimitiveType,
}

impl PythonEnumType {
    pub fn type_object_name(&self) -> String {
        format!("boltffi_python_{}_type", self.native_name_stem)
    }

    pub fn parser_name(&self) -> String {
        format!("boltffi_python_parse_{}", self.native_name_stem)
    }

    pub fn boxer_name(&self) -> String {
        format!("boltffi_python_box_{}", self.native_name_stem)
    }

    pub fn vector_parser_name(&self) -> String {
        format!("boltffi_python_parse_vec_{}", self.native_name_stem)
    }

    pub fn vector_decoder_name(&self) -> String {
        format!("boltffi_python_decode_owned_vec_{}", self.native_name_stem)
    }

    pub fn native_to_wire_tag_name(&self) -> String {
        format!(
            "boltffi_python_{}_native_to_wire_tag",
            self.native_name_stem
        )
    }

    pub fn box_from_wire_tag_name(&self) -> String {
        format!("boltffi_python_box_{}_from_wire_tag", self.native_name_stem)
    }

    pub fn type_literal(&self) -> String {
        self.class_name.clone()
    }

    pub fn registration_function_name(&self) -> String {
        format!("_register_{}", self.native_name_stem)
    }

    pub fn registration_wrapper_name(&self) -> String {
        format!("boltffi_python_wrapper_register_{}", self.native_name_stem)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonCStyleEnumVariant {
    pub member_name: String,
    pub native_value: i128,
    pub native_c_literal: String,
    pub wire_tag: i128,
    pub wire_c_literal: String,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonCStyleEnum {
    pub type_ref: PythonEnumType,
    pub variants: Vec<PythonCStyleEnumVariant>,
    pub constructors: Vec<PythonEnumConstructor>,
    pub methods: Vec<PythonEnumMethod>,
}

impl PythonCStyleEnum {
    pub fn class_name(&self) -> &str {
        &self.type_ref.class_name
    }

    pub fn callables(&self) -> impl Iterator<Item = &PythonCallable> {
        self.constructors
            .iter()
            .map(PythonEnumConstructor::callable)
            .chain(self.methods.iter().map(PythonEnumMethod::callable))
    }

    pub fn native_callables(&self) -> impl Iterator<Item = PythonNativeCallable<'_>> {
        self.constructors
            .iter()
            .map(PythonEnumConstructor::native_callable)
            .chain(self.methods.iter().map(PythonEnumMethod::native_callable))
    }

    pub fn has_native_callables(&self) -> bool {
        !self.constructors.is_empty() || !self.methods.is_empty()
    }
}
