use crate::ir::types::PrimitiveType;
use crate::render::python::primitives::PythonScalarTypeExt as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonType {
    Void,
    Primitive(PrimitiveType),
}

impl PythonType {
    pub fn python_annotation(&self) -> &'static str {
        match self {
            Self::Void => "None",
            Self::Primitive(primitive) => primitive.python_annotation(),
        }
    }

    pub fn used_primitive(&self) -> Option<PrimitiveType> {
        match self {
            Self::Void => None,
            Self::Primitive(primitive) => Some(*primitive),
        }
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonParameter {
    pub name: String,
    pub type_ref: PythonType,
}

impl PythonParameter {
    pub fn primitive(&self) -> PrimitiveType {
        match &self.type_ref {
            PythonType::Void => unreachable!("python parameters cannot be void"),
            PythonType::Primitive(primitive) => *primitive,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonFunction {
    pub python_name: String,
    pub ffi_symbol: String,
    pub parameters: Vec<PythonParameter>,
    pub return_type: PythonType,
}

impl PythonFunction {
    pub fn parameter_count(&self) -> usize {
        self.parameters.len()
    }

    pub fn takes_no_parameters(&self) -> bool {
        self.parameters.is_empty()
    }

    pub fn returns_void(&self) -> bool {
        self.return_type.is_void()
    }

    pub fn return_primitive(&self) -> Option<PrimitiveType> {
        self.return_type.used_primitive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonModule {
    pub module_name: String,
    pub package_name: String,
    pub package_version: Option<String>,
    pub library_name: String,
    pub functions: Vec<PythonFunction>,
}

impl PythonModule {
    pub fn module_name_literal(&self) -> String {
        format!("{:?}", self.module_name)
    }

    pub fn package_name_literal(&self) -> String {
        format!("{:?}", self.package_name)
    }

    pub fn package_version_literal(&self) -> String {
        self.package_version
            .as_ref()
            .map(|version| format!("{version:?}"))
            .unwrap_or_else(|| "None".to_string())
    }

    pub fn exported_names(&self) -> Vec<&str> {
        self.functions
            .iter()
            .map(|function| function.python_name.as_str())
            .collect()
    }

    pub fn used_scalar_types(&self) -> Vec<PrimitiveType> {
        self.functions
            .iter()
            .flat_map(|function| {
                function
                    .parameters
                    .iter()
                    .filter_map(|parameter| parameter.type_ref.used_primitive())
                    .chain(function.return_type.used_primitive())
            })
            .fold(Vec::new(), |mut scalar_types, primitive| {
                if !scalar_types.contains(&primitive) {
                    scalar_types.push(primitive);
                }
                scalar_types
            })
    }
}
