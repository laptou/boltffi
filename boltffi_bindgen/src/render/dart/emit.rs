use askama::Template as _;

use crate::{
    ir::{
        BuiltinId, EnumLayout, PrimitiveType, ReadOp, ReadSeq, ReturnDef, TypeExpr, ValueExpr,
        VecLayout, WriteOp, WriteSeq,
    },
    render::dart::{
        DartLibrary, NamingConvention,
        templates::{NativeFunctionsTemplate, NativeRecordTemplate, PreludeTemplate},
    },
};

pub struct DartEmitter {}

impl DartEmitter {
    pub fn emit(library: &DartLibrary) -> String {
        let mut output = String::new();

        output.push_str(PreludeTemplate {}.render().unwrap().as_str());
        output.push_str("\n\n");

        for r in &library.records {
            if let Some(layout) = &r.blittable_layout {
                output.push_str(
                    NativeRecordTemplate {
                        name: &r.name,
                        layout,
                    }
                    .render()
                    .unwrap()
                    .as_str(),
                );
                output.push_str("\n\n");
            }
        }

        output.push_str(
            NativeFunctionsTemplate {
                cfuncs: &library.native.functions,
            }
            .render()
            .unwrap()
            .as_str(),
        );
        output.push_str("\n\n");

        output
    }
}

pub fn primitive_dart_type(primitive: PrimitiveType) -> String {
    match primitive {
        PrimitiveType::Bool => "bool".to_string(),
        PrimitiveType::I8
        | PrimitiveType::U8
        | PrimitiveType::I16
        | PrimitiveType::U16
        | PrimitiveType::I32
        | PrimitiveType::U32
        | PrimitiveType::I64
        | PrimitiveType::U64
        | PrimitiveType::ISize
        | PrimitiveType::USize => "int".to_string(),
        PrimitiveType::F32 | PrimitiveType::F64 => "double".to_string(),
    }
}

pub fn primitive_native_type(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::Bool => "$$ffi.Bool",
        PrimitiveType::I8 => "$$ffi.Int8",
        PrimitiveType::I16 => "$$ffi.Int16",
        PrimitiveType::I32 => "$$ffi.Int32",
        PrimitiveType::I64 => "$$ffi.Int64",
        PrimitiveType::U8 => "$$ffi.Uint8",
        PrimitiveType::U16 => "$$ffi.Uint16",
        PrimitiveType::U32 => "$$ffi.Uint32",
        PrimitiveType::U64 => "$$ffi.Uint64",
        PrimitiveType::ISize => "$$ffi.IntPtr",
        PrimitiveType::USize => "$$ffi.UintPtr",
        PrimitiveType::F32 => "$$ffi.Float",
        PrimitiveType::F64 => "$$ffi.Double",
    }
}

fn render_type_name(name: &str) -> String {
    NamingConvention::class_name(name)
}

pub fn render_value(expr: &ValueExpr) -> String {
    match expr {
        ValueExpr::Instance => String::new(),
        ValueExpr::Var(name) => name.clone(),
        ValueExpr::Named(name) => NamingConvention::property_name(name),
        ValueExpr::Field(parent, field) => {
            let parent_str = render_value(parent);
            let field_str = NamingConvention::property_name(field.as_str());
            if parent_str.is_empty() {
                field_str
            } else {
                format!("{}.{}", parent_str, field_str)
            }
        }
    }
}

pub fn type_expr_dart_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Primitive(p) => primitive_dart_type(*p),
        TypeExpr::String => "String".to_string(),
        TypeExpr::Bytes => "Uint8List".to_string(),
        TypeExpr::Vec(inner) => match inner.as_ref() {
            TypeExpr::Primitive(primitive) => match primitive {
                PrimitiveType::I32 => "Int32List".to_string(),
                PrimitiveType::U32 => "Uint32List".to_string(),
                PrimitiveType::I16 => "Int16List".to_string(),
                PrimitiveType::U16 => "Uint16List".to_string(),
                PrimitiveType::I64 => "Int64List".to_string(),
                PrimitiveType::U64 => "Uint64List".to_string(),
                PrimitiveType::ISize => "Int64List".to_string(),
                PrimitiveType::USize => "Uint64List".to_string(),
                PrimitiveType::F32 => "Float32List".to_string(),
                PrimitiveType::F64 => "Float64List".to_string(),
                PrimitiveType::U8 => "Uint8List".to_string(),
                PrimitiveType::I8 => "Int8List".to_string(),
                PrimitiveType::Bool => "Uint8List".to_string(),
            },
            _ => format!("List<{}>", type_expr_dart_type(inner)),
        },
        TypeExpr::Option(inner) => format!("{}?", type_expr_dart_type(inner)),
        TypeExpr::Result { ok, err } => format!(
            "BoltFFIResult<{}, {}>",
            type_expr_dart_type(ok),
            type_expr_dart_type(err)
        ),
        TypeExpr::Record(id) => render_type_name(id.as_str()),
        TypeExpr::Enum(id) => render_type_name(id.as_str()),
        TypeExpr::Custom(id) => render_type_name(id.as_str()),
        TypeExpr::Builtin(id) => match id.as_str() {
            "Duration" => "Duration".to_string(),
            "SystemTime" => "Datetime".to_string(),
            "Uuid" => "(int, int)".to_string(), // NOTE: not builtin
            "Url" => "Uri".to_string(),
            _ => "String".to_string(),
        },
        TypeExpr::Handle(class_id) => render_type_name(class_id.as_str()),
        TypeExpr::Callback(callback_id) => render_type_name(callback_id.as_str()),
        TypeExpr::Void => "void".to_string(),
    }
}

pub fn return_def_dart_type(return_def: &ReturnDef) -> String {
    match return_def {
        ReturnDef::Void => "void".to_string(),
        ReturnDef::Value(type_expr) => type_expr_dart_type(type_expr),
        ReturnDef::Result { ok, err } => format!(
            "BoltFFIResult<{}, {}>",
            type_expr_dart_type(ok),
            type_expr_dart_type(err)
        ),
    }
}

pub fn primitive_as_num(primitive: PrimitiveType, value: &str) -> String {
    match primitive {
        PrimitiveType::Bool => format!("({} ? 1 : 0)", value),
        PrimitiveType::I8
        | PrimitiveType::U8
        | PrimitiveType::I16
        | PrimitiveType::U16
        | PrimitiveType::I32
        | PrimitiveType::U32
        | PrimitiveType::I64
        | PrimitiveType::U64
        | PrimitiveType::ISize
        | PrimitiveType::USize
        | PrimitiveType::F32
        | PrimitiveType::F64 => value.to_string(),
    }
}

pub fn primitive_blittable_write_method(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::I8 => "setInt8",
        PrimitiveType::Bool | PrimitiveType::U8 => "setUint8",
        PrimitiveType::I16 => "setInt16",
        PrimitiveType::U16 => "setUint16",
        PrimitiveType::I32 => "setInt32",
        PrimitiveType::U32 => "setUint32",
        PrimitiveType::I64 | PrimitiveType::ISize => "setInt64",
        PrimitiveType::U64 | PrimitiveType::USize => "setUint64",
        PrimitiveType::F32 => "setFloat32",
        PrimitiveType::F64 => "setFloat64",
    }
}

pub fn emit_write_blittable_value(
    offset: &str,
    primitive: PrimitiveType,
    value: &str,
    writer_name: &str,
) -> String {
    format!(
        "{}.{}({}, {}, $$typed_data.Endian.little)",
        writer_name,
        primitive_blittable_write_method(primitive),
        offset,
        primitive_as_num(primitive, value)
    )
}

pub fn primitive_blittable_read_method(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::I8 => "getInt8",
        PrimitiveType::Bool | PrimitiveType::U8 => "getUint8",
        PrimitiveType::I16 => "getInt16",
        PrimitiveType::U16 => "getUint16",
        PrimitiveType::I32 => "getInt32",
        PrimitiveType::U32 => "getUint32",
        PrimitiveType::I64 | PrimitiveType::ISize => "getInt64",
        PrimitiveType::U64 | PrimitiveType::USize => "getUint64",
        PrimitiveType::F32 => "getFloat32",
        PrimitiveType::F64 => "getFloat64",
    }
}

pub fn emit_read_blittable_value(
    offset: &str,
    primitive: PrimitiveType,
    bytes_name: &str,
) -> String {
    format!(
        "{}.{}({}, $$typed_data.Endian.little)",
        bytes_name,
        primitive_blittable_read_method(primitive),
        offset,
    )
}

pub fn primitive_write_method(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::Bool => "writeBool",
        PrimitiveType::I8 => "writeI8",
        PrimitiveType::U8 => "writeU8",
        PrimitiveType::I16 => "writeI16",
        PrimitiveType::U16 => "writeU16",
        PrimitiveType::I32 => "writeI32",
        PrimitiveType::U32 => "writeU32",
        PrimitiveType::I64 | PrimitiveType::ISize => "writeI64",
        PrimitiveType::U64 | PrimitiveType::USize => "writeU64",
        PrimitiveType::F32 => "writeF32",
        PrimitiveType::F64 => "writeF64",
    }
}

fn emit_write_primitive(primitive: PrimitiveType, writer_name: &str, value: &str) -> String {
    format!(
        "{}.{}({})",
        writer_name,
        primitive_write_method(primitive),
        value
    )
}

fn enum_tag_write_expr(tag_type: PrimitiveType, writer_name: &str, value_expr: &str) -> String {
    let write_method = primitive_write_method(tag_type);

    format!("{}.{}({})", writer_name, write_method, value_expr)
}

fn emit_write_builtin(id: &BuiltinId, writer_name: &str, value: &str) -> String {
    match id.as_str() {
        "Duration" => format!("{}.writeDuration({})", writer_name, value),
        "SystemTime" => format!("{}.writeInstant({})", writer_name, value),
        "Uuid" => format!("{}.writeUuid({})", writer_name, value),
        "Url" => format!("{}.writeUri({})", writer_name, value),
        _ => format!("{}.writeString({})", writer_name, value),
    }
}

fn write_seq_dart_type(seq: &WriteSeq) -> String {
    match seq.ops.first() {
        Some(WriteOp::Primitive { primitive, .. }) => {
            type_expr_dart_type(&TypeExpr::Primitive(*primitive))
        }
        Some(WriteOp::String { .. }) => "String".to_string(),
        Some(WriteOp::Bytes { .. }) => "Uint8List".to_string(),
        Some(WriteOp::Builtin { id, .. }) => type_expr_dart_type(&TypeExpr::Builtin(id.clone())),
        Some(WriteOp::Record { id, .. }) => render_type_name(id.as_str()),
        Some(WriteOp::Enum { id, .. }) => render_type_name(id.as_str()),
        Some(WriteOp::Custom { id, .. }) => render_type_name(id.as_str()),
        Some(WriteOp::Vec { element_type, .. }) => {
            type_expr_dart_type(&TypeExpr::Vec(Box::new(element_type.clone())))
        }
        Some(WriteOp::Option { some, .. }) => format!("{}?", write_seq_dart_type(some)),
        Some(WriteOp::Result { ok, err, .. }) => format!(
            "BoltFFIResult<{}, {}>",
            write_seq_dart_type(ok),
            write_seq_dart_type(err)
        ),
        _ => "dynamic".to_string(),
    }
}

fn emit_write_vec(
    _value: &str,
    _element_type: &TypeExpr,
    _element: &WriteSeq,
    _layout: &VecLayout,
) -> String {
    String::new()
}

pub fn emit_write_expr(_seq: &WriteSeq, _writer_name: &str) -> String {
    String::new()
}

pub fn primitive_read_method(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::Bool => "readBool",
        PrimitiveType::I8 => "readI8",
        PrimitiveType::U8 => "readU8",
        PrimitiveType::I16 => "readI16",
        PrimitiveType::U16 => "readU16",
        PrimitiveType::I32 => "readI32",
        PrimitiveType::U32 => "readU32",
        PrimitiveType::I64 | PrimitiveType::ISize => "readI64",
        PrimitiveType::U64 | PrimitiveType::USize => "readU64",
        PrimitiveType::F32 => "readF32",
        PrimitiveType::F64 => "readF64",
    }
}

fn emit_reader_vec(element_type: &TypeExpr, element: &ReadSeq, layout: &VecLayout) -> String {
    match layout {
        VecLayout::Blittable { .. } => match element_type {
            TypeExpr::Primitive(primitive) => {
                let method = match primitive {
                    PrimitiveType::U8 | PrimitiveType::Bool => "readUint8List",
                    PrimitiveType::I8 => "readInt8List",
                    PrimitiveType::I16 => "readInt16List",
                    PrimitiveType::U16 => "readUint16List",
                    PrimitiveType::I32 => "readInt32List",
                    PrimitiveType::U32 => "readUint32List",
                    PrimitiveType::U64 | PrimitiveType::USize => "readUint64List",
                    PrimitiveType::I64 | PrimitiveType::ISize => "readInt64List",
                    PrimitiveType::F32 => "readFloat32List",
                    PrimitiveType::F64 => "readFloat64List",
                };
                format!("reader.{}()", method)
            }
            _ => {
                let inner = emit_reader_read(element);
                format!("reader.readList((reader) => {})", inner)
            }
        },
        VecLayout::Encoded => {
            let inner = emit_reader_read(element);
            format!("reader.readList((reader) => {})", inner)
        }
    }
}

pub fn emit_reader_read(seq: &ReadSeq) -> String {
    let op = seq.ops.first().expect("read ops");
    match op {
        ReadOp::Primitive { primitive, .. } => {
            format!("reader.{}()", primitive_read_method(*primitive))
        }
        ReadOp::String { .. } => "reader.readString()".to_string(),
        ReadOp::Bytes { .. } => "reader.readUint8List()".to_string(),
        ReadOp::Record { id, .. } => {
            format!("{}.decode(reader)", render_type_name(id.as_str()))
        }
        ReadOp::Enum { id, layout, .. } => match layout {
            EnumLayout::CStyle {
                tag_type,
                is_error: false,
                ..
            } => {
                format!(
                    "{}.fromValue(reader.{}())",
                    render_type_name(id.as_str()),
                    primitive_read_method(*tag_type),
                )
            }
            EnumLayout::CStyle { is_error: true, .. }
            | EnumLayout::Data { .. }
            | EnumLayout::Recursive => {
                format!("{}.decode(reader)", render_type_name(id.as_str()))
            }
        },
        ReadOp::Option { some, .. } => {
            let inner = emit_reader_read(some);
            format!("reader.readOptional((reader) => {})", inner)
        }
        ReadOp::Vec {
            element_type,
            element,
            layout,
            ..
        } => emit_reader_vec(element_type, element, layout),
        ReadOp::Result { ok, err, .. } => {
            let _ok_expr = emit_reader_read(ok);
            let _err_expr = emit_reader_read(err);
            todo!()
        }
        ReadOp::Builtin { id, .. } => match id.as_str() {
            "Duration" => "reader.readDuration()".to_string(),
            "SystemTime" => "reader.readInstant()".to_string(),
            "Uuid" => "reader.readUuid()".to_string(),
            "Url" => "reader.readUri()".to_string(),
            _ => "reader.readString()".to_string(),
        },
        ReadOp::Custom { id, .. } => {
            format!("{}.decode(reader)", render_type_name(id.as_str()))
        }
    }
}
