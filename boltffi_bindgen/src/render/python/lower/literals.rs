use crate::ir::types::PrimitiveType;

use super::PythonLowerer;

impl PythonLowerer<'_> {
    pub(super) fn primitive_c_literal(tag_type: PrimitiveType, value: i128) -> String {
        match tag_type {
            PrimitiveType::I8 => format!("((int8_t){value})"),
            PrimitiveType::U8 => format!("((uint8_t){value}u)"),
            PrimitiveType::I16 => format!("((int16_t){value})"),
            PrimitiveType::U16 => format!("((uint16_t){value}u)"),
            PrimitiveType::I32 => format!("((int32_t){value})"),
            PrimitiveType::U32 => format!("((uint32_t){value}u)"),
            PrimitiveType::I64 => format!("((int64_t){value}LL)"),
            PrimitiveType::U64 => format!("((uint64_t){value}ULL)"),
            PrimitiveType::ISize => format!("((intptr_t){value})"),
            PrimitiveType::USize => format!("((uintptr_t){value}u)"),
            PrimitiveType::Bool => {
                if value == 0 {
                    "false".to_string()
                } else {
                    "true".to_string()
                }
            }
            PrimitiveType::F32 | PrimitiveType::F64 => {
                panic!("c-style enums must not use floating tag types")
            }
        }
    }
}
