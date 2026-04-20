use crate::ir::types::PrimitiveType;

use super::plan::CSharpType;

/// Maps a BoltFFI primitive to the corresponding [`CSharpType`].
///
/// C# has native unsigned types (`byte`, `ushort`, `uint`, `ulong`) and
/// platform-sized integers (`nint`, `nuint`), so every primitive maps
/// to a distinct C# type with no information loss.
pub fn csharp_type(primitive: PrimitiveType) -> CSharpType {
    match primitive {
        PrimitiveType::Bool => CSharpType::Bool,
        PrimitiveType::I8 => CSharpType::SByte,
        PrimitiveType::U8 => CSharpType::Byte,
        PrimitiveType::I16 => CSharpType::Short,
        PrimitiveType::U16 => CSharpType::UShort,
        PrimitiveType::I32 => CSharpType::Int,
        PrimitiveType::U32 => CSharpType::UInt,
        PrimitiveType::I64 => CSharpType::Long,
        PrimitiveType::U64 => CSharpType::ULong,
        PrimitiveType::ISize => CSharpType::NInt,
        PrimitiveType::USize => CSharpType::NUInt,
        PrimitiveType::F32 => CSharpType::Float,
        PrimitiveType::F64 => CSharpType::Double,
    }
}
