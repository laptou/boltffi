use crate::ir::types::PrimitiveType;
use crate::render::python::{PythonFunction, PythonParameter, PythonType};

pub(crate) trait CPythonPrimitiveTypeExt {
    fn c_type_name(self) -> &'static str;
    fn parser_name(self) -> &'static str;
    fn uses_bool_parser(self) -> bool;
    fn uses_signed_long_long_parser(self) -> bool;
    fn signed_min_macro(self) -> &'static str;
    fn signed_max_macro(self) -> &'static str;
    fn uses_unsigned_long_long_with_range_check(self) -> bool;
    fn unsigned_max_macro(self) -> &'static str;
    fn uses_u64_parser(self) -> bool;
    fn uses_isize_parser(self) -> bool;
    fn uses_usize_parser(self) -> bool;
    fn uses_f32_parser(self) -> bool;
    fn uses_f64_parser(self) -> bool;
    fn boxes_as_bool(self) -> bool;
    fn boxes_as_signed_long(self) -> bool;
    fn boxes_as_unsigned_long(self) -> bool;
    fn boxes_as_signed_long_long(self) -> bool;
    fn boxes_as_unsigned_long_long(self) -> bool;
    fn boxes_as_ssize(self) -> bool;
    fn boxes_as_size(self) -> bool;
    fn boxes_as_double(self) -> bool;
    fn rust_name(self) -> &'static str;
}

impl CPythonPrimitiveTypeExt for PrimitiveType {
    fn c_type_name(self) -> &'static str {
        match self {
            PrimitiveType::Bool => "bool",
            PrimitiveType::I8 => "int8_t",
            PrimitiveType::U8 => "uint8_t",
            PrimitiveType::I16 => "int16_t",
            PrimitiveType::U16 => "uint16_t",
            PrimitiveType::I32 => "int32_t",
            PrimitiveType::U32 => "uint32_t",
            PrimitiveType::I64 => "int64_t",
            PrimitiveType::U64 => "uint64_t",
            PrimitiveType::ISize => "intptr_t",
            PrimitiveType::USize => "uintptr_t",
            PrimitiveType::F32 => "float",
            PrimitiveType::F64 => "double",
        }
    }

    fn parser_name(self) -> &'static str {
        match self {
            PrimitiveType::Bool => "boltffi_python_parse_bool",
            PrimitiveType::I8 => "boltffi_python_parse_i8",
            PrimitiveType::U8 => "boltffi_python_parse_u8",
            PrimitiveType::I16 => "boltffi_python_parse_i16",
            PrimitiveType::U16 => "boltffi_python_parse_u16",
            PrimitiveType::I32 => "boltffi_python_parse_i32",
            PrimitiveType::U32 => "boltffi_python_parse_u32",
            PrimitiveType::I64 => "boltffi_python_parse_i64",
            PrimitiveType::U64 => "boltffi_python_parse_u64",
            PrimitiveType::ISize => "boltffi_python_parse_isize",
            PrimitiveType::USize => "boltffi_python_parse_usize",
            PrimitiveType::F32 => "boltffi_python_parse_f32",
            PrimitiveType::F64 => "boltffi_python_parse_f64",
        }
    }

    fn uses_bool_parser(self) -> bool {
        matches!(self, PrimitiveType::Bool)
    }

    fn uses_signed_long_long_parser(self) -> bool {
        matches!(
            self,
            PrimitiveType::I8 | PrimitiveType::I16 | PrimitiveType::I32 | PrimitiveType::I64
        )
    }

    fn signed_min_macro(self) -> &'static str {
        match self {
            PrimitiveType::I8 => "INT8_MIN",
            PrimitiveType::I16 => "INT16_MIN",
            PrimitiveType::I32 => "INT32_MIN",
            PrimitiveType::I64 => "INT64_MIN",
            _ => unreachable!(),
        }
    }

    fn signed_max_macro(self) -> &'static str {
        match self {
            PrimitiveType::I8 => "INT8_MAX",
            PrimitiveType::I16 => "INT16_MAX",
            PrimitiveType::I32 => "INT32_MAX",
            PrimitiveType::I64 => "INT64_MAX",
            _ => unreachable!(),
        }
    }

    fn uses_unsigned_long_long_with_range_check(self) -> bool {
        matches!(
            self,
            PrimitiveType::U8 | PrimitiveType::U16 | PrimitiveType::U32
        )
    }

    fn unsigned_max_macro(self) -> &'static str {
        match self {
            PrimitiveType::U8 => "UINT8_MAX",
            PrimitiveType::U16 => "UINT16_MAX",
            PrimitiveType::U32 => "UINT32_MAX",
            _ => unreachable!(),
        }
    }

    fn uses_u64_parser(self) -> bool {
        matches!(self, PrimitiveType::U64)
    }

    fn uses_isize_parser(self) -> bool {
        matches!(self, PrimitiveType::ISize)
    }

    fn uses_usize_parser(self) -> bool {
        matches!(self, PrimitiveType::USize)
    }

    fn uses_f32_parser(self) -> bool {
        matches!(self, PrimitiveType::F32)
    }

    fn uses_f64_parser(self) -> bool {
        matches!(self, PrimitiveType::F64)
    }

    fn boxes_as_bool(self) -> bool {
        matches!(self, PrimitiveType::Bool)
    }

    fn boxes_as_signed_long(self) -> bool {
        matches!(
            self,
            PrimitiveType::I8 | PrimitiveType::I16 | PrimitiveType::I32
        )
    }

    fn boxes_as_unsigned_long(self) -> bool {
        matches!(
            self,
            PrimitiveType::U8 | PrimitiveType::U16 | PrimitiveType::U32
        )
    }

    fn boxes_as_signed_long_long(self) -> bool {
        matches!(self, PrimitiveType::I64)
    }

    fn boxes_as_unsigned_long_long(self) -> bool {
        matches!(self, PrimitiveType::U64)
    }

    fn boxes_as_ssize(self) -> bool {
        matches!(self, PrimitiveType::ISize)
    }

    fn boxes_as_size(self) -> bool {
        matches!(self, PrimitiveType::USize)
    }

    fn boxes_as_double(self) -> bool {
        matches!(self, PrimitiveType::F32 | PrimitiveType::F64)
    }

    fn rust_name(self) -> &'static str {
        match self {
            PrimitiveType::Bool => "bool",
            PrimitiveType::I8 => "i8",
            PrimitiveType::U8 => "u8",
            PrimitiveType::I16 => "i16",
            PrimitiveType::U16 => "u16",
            PrimitiveType::I32 => "i32",
            PrimitiveType::U32 => "u32",
            PrimitiveType::I64 => "i64",
            PrimitiveType::U64 => "u64",
            PrimitiveType::ISize => "isize",
            PrimitiveType::USize => "usize",
            PrimitiveType::F32 => "f32",
            PrimitiveType::F64 => "f64",
        }
    }
}

pub(crate) trait CPythonTypeExt {
    fn c_type_name(&self) -> &'static str;
}

impl CPythonTypeExt for PythonType {
    fn c_type_name(&self) -> &'static str {
        match self {
            PythonType::Void => "void",
            PythonType::Primitive(primitive) => primitive.c_type_name(),
        }
    }
}

pub(crate) trait CPythonParameterExt {
    fn c_type_name(&self) -> &'static str;
}

impl CPythonParameterExt for PythonParameter {
    fn c_type_name(&self) -> &'static str {
        self.primitive().c_type_name()
    }
}

pub(crate) trait CPythonFunctionExt {
    fn wrapper_name(&self) -> String;
    fn function_pointer_typedef_name(&self) -> String;
    fn function_pointer_name(&self) -> String;
}

impl CPythonFunctionExt for PythonFunction {
    fn wrapper_name(&self) -> String {
        format!("boltffi_python_{}", self.python_name)
    }

    fn function_pointer_typedef_name(&self) -> String {
        format!("boltffi_python_{}_symbol_fn", self.python_name)
    }

    fn function_pointer_name(&self) -> String {
        format!("boltffi_python_{}_symbol", self.python_name)
    }
}
