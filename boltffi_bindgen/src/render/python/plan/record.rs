use crate::ir::types::PrimitiveType;

use super::{PythonCallable, PythonNativeCallable, PythonParameter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecordType {
    pub native_name_stem: String,
    pub class_name: String,
    pub c_type_name: String,
}

impl PythonRecordType {
    pub fn type_object_name(&self) -> String {
        format!("boltffi_python_{}_type", self.native_name_stem)
    }

    pub fn parser_name(&self) -> String {
        format!("boltffi_python_parse_{}", self.native_name_stem)
    }

    pub fn boxer_name(&self) -> String {
        format!("boltffi_python_box_{}", self.native_name_stem)
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
pub struct PythonRecordField {
    pub python_name: String,
    pub native_name: String,
    pub primitive: PrimitiveType,
}

impl PythonRecordField {
    pub fn annotation(&self) -> &'static str {
        match self.primitive {
            PrimitiveType::Bool => "bool",
            PrimitiveType::F32 | PrimitiveType::F64 => "float",
            PrimitiveType::I8
            | PrimitiveType::U8
            | PrimitiveType::I16
            | PrimitiveType::U16
            | PrimitiveType::I32
            | PrimitiveType::U32
            | PrimitiveType::I64
            | PrimitiveType::U64
            | PrimitiveType::ISize
            | PrimitiveType::USize => "int",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecordFields {
    first: PythonRecordField,
    remaining: Vec<PythonRecordField>,
}

impl PythonRecordFields {
    pub fn try_from_vec(fields: Vec<PythonRecordField>) -> Option<Self> {
        let mut fields = fields.into_iter();
        let first = fields.next()?;

        Some(Self {
            first,
            remaining: fields.collect(),
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &PythonRecordField> {
        std::iter::once(&self.first).chain(self.remaining.iter())
    }

    pub fn first(&self) -> &PythonRecordField {
        &self.first
    }

    pub fn len(&self) -> usize {
        1 + self.remaining.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecordConstructor {
    pub python_name: String,
    pub callable: PythonCallable,
}

impl PythonRecordConstructor {
    pub fn callable(&self) -> &PythonCallable {
        &self.callable
    }

    pub fn native_callable(&self) -> PythonNativeCallable<'_> {
        PythonNativeCallable {
            module_attribute_name: self.callable.native_name.as_str(),
            callable: &self.callable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecordMethod {
    pub python_name: String,
    pub callable: PythonCallable,
    pub is_static: bool,
}

impl PythonRecordMethod {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonRecord {
    pub type_ref: PythonRecordType,
    pub fields: PythonRecordFields,
    pub constructors: Vec<PythonRecordConstructor>,
    pub methods: Vec<PythonRecordMethod>,
}

impl PythonRecord {
    pub fn new(
        type_ref: PythonRecordType,
        fields: Vec<PythonRecordField>,
        constructors: Vec<PythonRecordConstructor>,
        methods: Vec<PythonRecordMethod>,
    ) -> Option<Self> {
        Some(Self {
            type_ref,
            fields: PythonRecordFields::try_from_vec(fields)?,
            constructors,
            methods,
        })
    }

    pub fn class_name(&self) -> &str {
        &self.type_ref.class_name
    }

    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    pub fn callables(&self) -> impl Iterator<Item = &PythonCallable> {
        self.constructors
            .iter()
            .map(PythonRecordConstructor::callable)
            .chain(self.methods.iter().map(PythonRecordMethod::callable))
    }

    pub fn native_callables(&self) -> impl Iterator<Item = PythonNativeCallable<'_>> {
        self.constructors
            .iter()
            .map(PythonRecordConstructor::native_callable)
            .chain(self.methods.iter().map(PythonRecordMethod::native_callable))
    }

    pub fn has_native_callables(&self) -> bool {
        !self.constructors.is_empty() || !self.methods.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::types::PrimitiveType;

    use super::{PythonRecord, PythonRecordField, PythonRecordType};

    #[test]
    fn direct_python_records_require_at_least_one_field() {
        let record = PythonRecord::new(
            PythonRecordType {
                native_name_stem: "empty".to_string(),
                class_name: "Empty".to_string(),
                c_type_name: "___Empty".to_string(),
            },
            vec![],
            vec![],
            vec![],
        );

        assert!(record.is_none());
    }

    #[test]
    fn direct_python_records_keep_first_field_accessible() {
        let record = PythonRecord::new(
            PythonRecordType {
                native_name_stem: "point".to_string(),
                class_name: "Point".to_string(),
                c_type_name: "___Point".to_string(),
            },
            vec![PythonRecordField {
                python_name: "x".to_string(),
                native_name: "x".to_string(),
                primitive: PrimitiveType::F64,
            }],
            vec![],
            vec![],
        )
        .expect("non-empty direct python record should be constructed");

        assert_eq!(record.fields.first().python_name, "x");
        assert_eq!(record.field_count(), 1);
    }
}
