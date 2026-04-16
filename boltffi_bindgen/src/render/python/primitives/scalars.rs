use crate::ir::types::PrimitiveType;

pub(crate) trait PythonScalarTypeExt {
    fn python_annotation(self) -> &'static str;
}

impl PythonScalarTypeExt for PrimitiveType {
    fn python_annotation(self) -> &'static str {
        match self {
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
