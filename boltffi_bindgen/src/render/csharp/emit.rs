//! Orchestrates the lowerer and templates to produce the final `.cs` output.
//!
//! Also hosts the C#-syntax helpers that translate ABI ops
//! ([`ReadOp`], [`WriteOp`], [`SizeExpr`], [`ValueExpr`]) into source
//! snippets. The lowerer calls these helpers to pre-render the wire
//! expressions that end up in [`CSharpRecordField`] and
//! [`CSharpWireWriter`]. Keeping the syntax formatting here (and the
//! "which ops apply to which field" logic in [`lower`](super::lower))
//! mirrors the Java backend split.

use askama::Template as _;

use crate::ir::ops::{ReadOp, ReadSeq, SizeExpr, ValueExpr, WriteOp, WriteSeq};
use crate::ir::types::PrimitiveType;
use crate::ir::{AbiContract, FfiContract};

use super::{
    CSharpOptions, NamingConvention,
    lower::CSharpLowerer,
    plan::CSharpRecord,
    templates::{FunctionsTemplate, NativeTemplate, PreambleTemplate, RecordTemplate},
};

/// A single generated `.cs` file: its file name (relative to the output
/// directory) and its full source text.
#[derive(Debug, Clone)]
pub struct CSharpFile {
    pub file_name: String,
    pub source: String,
}

/// The rendered C# output: one file per record plus a main file with the
/// wrapper class and `[DllImport]` declarations.
#[derive(Debug, Clone)]
pub struct CSharpOutput {
    pub files: Vec<CSharpFile>,
    /// The top-level class name (used for the main file, e.g., `"MyApp"`).
    pub class_name: String,
    /// The C# namespace.
    pub namespace: String,
}

impl CSharpOutput {
    /// Concatenation of every file's source text. Convenience for tests
    /// and spot-checks that only care about "did this snippet appear
    /// anywhere in the generated code?"
    #[cfg(test)]
    pub fn combined_source(&self) -> String {
        self.files
            .iter()
            .map(|f| f.source.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Entry point for C# code generation.
pub struct CSharpEmitter;

impl CSharpEmitter {
    pub fn emit(ffi: &FfiContract, abi: &AbiContract, options: &CSharpOptions) -> CSharpOutput {
        let lowerer = CSharpLowerer::new(ffi, abi, options);
        let module = lowerer.lower();

        let mut files: Vec<CSharpFile> = module
            .records
            .iter()
            .map(|record| CSharpFile {
                file_name: format!("{}.cs", record.class_name),
                source: RecordTemplate {
                    record,
                    namespace: &module.namespace,
                }
                .render()
                .expect("record template failed"),
            })
            .collect();

        let mut main_source = String::new();
        main_source.push_str(&PreambleTemplate { module: &module }.render().unwrap());
        main_source.push('\n');
        main_source.push_str(&FunctionsTemplate { module: &module }.render().unwrap());
        main_source.push_str(&NativeTemplate { module: &module }.render().unwrap());
        main_source.push('\n');

        files.push(CSharpFile {
            file_name: format!("{}.cs", module.class_name),
            source: main_source,
        });

        CSharpOutput {
            files,
            class_name: module.class_name,
            namespace: module.namespace,
        }
    }
}

// ---------------------------------------------------------------------------
// Render helpers: ABI ops -> C# syntax snippets.
//
// These are the C# counterparts of the functions in render/java/emit.rs.
// Each takes a [`ReadSeq`] / [`WriteSeq`] / [`SizeExpr`] / [`ValueExpr`]
// node from the ABI and produces the C# source that implements it.
//
// Scope: supports the subset of ops we need for records with primitive,
// string, and nested-record fields. Vec / Option / Enum / Builtin / Custom
// etc. will be added in follow-up PRs — today they panic so the gap is
// surfaced loudly rather than silently producing broken output.
// ---------------------------------------------------------------------------

/// Render a [`ValueExpr`] as a C# value access path.
///
/// [`Instance`](ValueExpr::Instance) becomes `this.` (trailing dot joined
/// by the field walk). A [`Field`](ValueExpr::Field) chain walks outward
/// producing e.g. `this.Origin.X`. Parameter references keep their
/// camelCase name; field references convert to PascalCase property names
/// to match the record struct definition.
pub fn render_value(expr: &ValueExpr) -> String {
    match expr {
        ValueExpr::Instance => "this".to_string(),
        ValueExpr::Var(name) => name.clone(),
        ValueExpr::Named(name) => NamingConvention::field_name(name),
        ValueExpr::Field(parent, field) => {
            let parent_str = render_value(parent);
            let field_str = NamingConvention::property_name(field.as_str());
            format!("{}.{}", parent_str, field_str)
        }
    }
}

pub fn primitive_read_method(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::Bool => "ReadBool",
        PrimitiveType::I8 => "ReadI8",
        PrimitiveType::U8 => "ReadU8",
        PrimitiveType::I16 => "ReadI16",
        PrimitiveType::U16 => "ReadU16",
        PrimitiveType::I32 => "ReadI32",
        PrimitiveType::U32 => "ReadU32",
        PrimitiveType::I64 => "ReadI64",
        PrimitiveType::U64 => "ReadU64",
        PrimitiveType::ISize => "ReadNInt",
        PrimitiveType::USize => "ReadNUInt",
        PrimitiveType::F32 => "ReadF32",
        PrimitiveType::F64 => "ReadF64",
    }
}

pub fn primitive_write_method(primitive: PrimitiveType) -> &'static str {
    match primitive {
        PrimitiveType::Bool => "WriteBool",
        PrimitiveType::I8 => "WriteI8",
        PrimitiveType::U8 => "WriteU8",
        PrimitiveType::I16 => "WriteI16",
        PrimitiveType::U16 => "WriteU16",
        PrimitiveType::I32 => "WriteI32",
        PrimitiveType::U32 => "WriteU32",
        PrimitiveType::I64 => "WriteI64",
        PrimitiveType::U64 => "WriteU64",
        PrimitiveType::ISize => "WriteNInt",
        PrimitiveType::USize => "WriteNUInt",
        PrimitiveType::F32 => "WriteF32",
        PrimitiveType::F64 => "WriteF64",
    }
}

/// Render the first op of a [`ReadSeq`] as a decode expression.
///
/// Today each [`ReadSeq`] we handle has exactly one top-level op — either
/// a primitive, a string, or a nested record. Container ops (Option, Vec,
/// Enum, Result) will land in follow-up PRs.
pub fn emit_reader_read(seq: &ReadSeq) -> String {
    let op = seq.ops.first().expect("read ops");
    match op {
        ReadOp::Primitive { primitive, .. } => {
            format!("reader.{}()", primitive_read_method(*primitive))
        }
        ReadOp::String { .. } => "reader.ReadString()".to_string(),
        ReadOp::Bytes { .. } => "reader.ReadBytes()".to_string(),
        ReadOp::Record { id, .. } => {
            format!(
                "{}.Decode(reader)",
                NamingConvention::class_name(id.as_str())
            )
        }
        other => panic!("unsupported C# read op: {:?}", other),
    }
}

/// Render the first op of a [`WriteSeq`] as a statement that writes its
/// value into the `WireWriter` named by `writer_name`.
pub fn emit_write_expr(seq: &WriteSeq, writer_name: &str) -> String {
    let op = seq.ops.first().expect("write ops");
    match op {
        WriteOp::Primitive { primitive, value } => {
            format!(
                "{}.{}({})",
                writer_name,
                primitive_write_method(*primitive),
                render_value(value)
            )
        }
        WriteOp::String { value } => {
            format!("{}.WriteString({})", writer_name, render_value(value))
        }
        WriteOp::Bytes { value } => {
            format!("{}.WriteBytes({})", writer_name, render_value(value))
        }
        WriteOp::Record { value, .. } => {
            format!("{}.WireEncodeTo({})", render_value(value), writer_name)
        }
        other => panic!("unsupported C# write op: {:?}", other),
    }
}

/// Render a [`SizeExpr`] as a C# expression that evaluates to the
/// wire-encoded byte size.
///
/// The IR's convention for variable-length types is a
/// `Sum([Fixed(4), StringLen(v)])` or `Sum([Fixed(4), BytesLen(v)])`:
/// the outer `Sum` already accounts for the 4-byte length prefix, so
/// `StringLen` and `BytesLen` must render as just the payload byte
/// count. Doubling up (e.g. rendering `StringLen` as `4 + byte_count`)
/// would over-count by 4 bytes per string.
pub fn emit_size_expr(size: &SizeExpr) -> String {
    match size {
        SizeExpr::Fixed(value) => value.to_string(),
        SizeExpr::StringLen(value) => {
            format!("Encoding.UTF8.GetByteCount({})", render_value(value))
        }
        SizeExpr::BytesLen(value) => format!("{}.Length", render_value(value)),
        SizeExpr::WireSize { value, .. } => {
            format!("{}.WireEncodedSize()", render_value(value))
        }
        SizeExpr::Sum(parts) => {
            let rendered = parts
                .iter()
                .map(emit_size_expr)
                .collect::<Vec<_>>()
                .join(" + ");
            format!("({})", rendered)
        }
        other => panic!("unsupported C# size expr: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Ignore unused-import warnings for CSharpRecord in emit.rs while the
// record template type is defined in templates.rs.
// ---------------------------------------------------------------------------
#[allow(dead_code)]
const _: fn(&CSharpRecord) = |_| {};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Lowerer as IrLowerer;
    use crate::ir::contract::{FfiContract, PackageInfo};
    use crate::ir::definitions::{FunctionDef, ParamDef, ParamPassing, ReturnDef};
    use crate::ir::ids::{FunctionId, ParamName};
    use crate::ir::types::{PrimitiveType, TypeExpr};
    use boltffi_ffi_rules::callable::ExecutionKind;

    fn empty_contract() -> FfiContract {
        FfiContract {
            package: PackageInfo {
                name: "demo_lib".to_string(),
                version: None,
            },
            functions: vec![],
            catalog: Default::default(),
        }
    }

    fn primitive_function(
        name: &str,
        params: Vec<(&str, PrimitiveType)>,
        returns: ReturnDef,
    ) -> FunctionDef {
        FunctionDef {
            id: FunctionId::new(name),
            params: params
                .into_iter()
                .map(|(param_name, prim)| ParamDef {
                    name: ParamName::new(param_name),
                    type_expr: TypeExpr::Primitive(prim),
                    passing: ParamPassing::Value,
                    doc: None,
                })
                .collect(),
            returns,
            execution_kind: ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        }
    }

    fn emit_contract(contract: &FfiContract) -> CSharpOutput {
        let abi = IrLowerer::new(contract).to_abi_contract();
        CSharpEmitter::emit(contract, &abi, &CSharpOptions::default())
    }

    fn assert_source_contains(source: &str, snippet: &str, expecting: &str) {
        assert!(
            source.contains(snippet),
            "expecting {expecting}\n  missing snippet: {snippet:?}"
        );
    }

    fn assert_source_lacks(source: &str, snippet: &str, expecting: &str) {
        assert!(
            !source.contains(snippet),
            "expecting {expecting}\n  unexpected snippet: {snippet:?}"
        );
    }

    #[test]
    fn emit_primitive_function_generates_wrapper_and_native() {
        let mut contract = empty_contract();
        contract.functions.push(primitive_function(
            "echo_i32",
            vec![("value", PrimitiveType::I32)],
            ReturnDef::Value(TypeExpr::Primitive(PrimitiveType::I32)),
        ));

        let src = emit_contract(&contract).combined_source();

        assert!(src.contains("public static int EchoI32(int value)"));
        assert!(src.contains("return NativeMethods.EchoI32(value);"));
        assert!(src.contains(r#"[DllImport(LibName, EntryPoint = "boltffi_echo_i32")]"#));
        assert!(src.contains("internal static extern int EchoI32(int value);"));
    }

    #[test]
    fn emit_void_function_omits_return_keyword() {
        let mut contract = empty_contract();
        contract
            .functions
            .push(primitive_function("noop", vec![], ReturnDef::Void));

        let src = emit_contract(&contract).combined_source();

        assert!(src.contains("public static void Noop()"));
        assert!(src.contains("NativeMethods.Noop();"));
        assert!(!src.contains("return NativeMethods.Noop()"));
    }

    #[test]
    fn emit_unsigned_types_use_csharp_unsigned_keywords() {
        let mut contract = empty_contract();
        contract.functions.push(primitive_function(
            "unsigned_echo",
            vec![
                ("a", PrimitiveType::U8),
                ("b", PrimitiveType::U16),
                ("c", PrimitiveType::U32),
                ("d", PrimitiveType::U64),
            ],
            ReturnDef::Value(TypeExpr::Primitive(PrimitiveType::U32)),
        ));

        let src = emit_contract(&contract).combined_source();

        assert!(src.contains("uint UnsignedEcho(byte a, ushort b, uint c, ulong d)"));
    }

    #[test]
    fn emit_namespace_and_class_use_pascal_case() {
        let contract = empty_contract();
        let output = emit_contract(&contract);

        assert_eq!(output.namespace, "DemoLib");
        assert_eq!(output.class_name, "DemoLib");
        assert!(output.combined_source().contains("namespace DemoLib"));
    }

    #[test]
    fn emit_escapes_csharp_keywords_in_param_names() {
        let mut contract = empty_contract();
        contract.functions.push(primitive_function(
            "test_keywords",
            vec![("int", PrimitiveType::I32), ("value", PrimitiveType::I32)],
            ReturnDef::Void,
        ));

        let src = emit_contract(&contract).combined_source();

        assert!(src.contains("@int"));
    }

    fn function_with_types(
        name: &str,
        params: Vec<(&str, TypeExpr)>,
        returns: ReturnDef,
    ) -> FunctionDef {
        FunctionDef {
            id: FunctionId::new(name),
            params: params
                .into_iter()
                .map(|(param_name, type_expr)| ParamDef {
                    name: ParamName::new(param_name),
                    type_expr,
                    passing: ParamPassing::Value,
                    doc: None,
                })
                .collect(),
            returns,
            execution_kind: ExecutionKind::Sync,
            doc: None,
            deprecated: None,
        }
    }

    /// C# P/Invoke marshals `bool` as a 4-byte Win32 BOOL by default, but
    /// BoltFFI's C ABI uses a 1-byte native bool, so the generated native
    /// signature must force `UnmanagedType.I1` for both param and return.
    #[test]
    fn emit_bool_function_uses_i1_marshalling_for_native_signature() {
        let mut contract = empty_contract();
        contract.functions.push(primitive_function(
            "flip",
            vec![("value", PrimitiveType::Bool)],
            ReturnDef::Value(TypeExpr::Primitive(PrimitiveType::Bool)),
        ));

        let src = emit_contract(&contract).combined_source();

        assert!(src.contains("public static bool Flip(bool value)"));
        assert!(src.contains("[return: MarshalAs(UnmanagedType.I1)]"));
        assert!(src.contains(
            "internal static extern bool Flip([MarshalAs(UnmanagedType.I1)] bool value);"
        ));
    }

    /// The C ABI for a `String` parameter is `(const uint8_t* ptr, uintptr_t len)`,
    /// which on the C# side becomes a `byte[]` + `UIntPtr` pair. The wrapper
    /// exposes a plain `string` and UTF-8 encodes it just before the native call.
    #[test]
    fn emit_string_param_marshals_as_byte_array_and_length() {
        let mut contract = empty_contract();
        contract.functions.push(function_with_types(
            "string_length",
            vec![("v", TypeExpr::String)],
            ReturnDef::Value(TypeExpr::Primitive(PrimitiveType::U32)),
        ));

        let src = emit_contract(&contract).combined_source();

        assert_source_contains(
            &src,
            "public static uint StringLength(string v)",
            "the public wrapper to expose the string param as a plain C# `string`",
        );
        assert_source_contains(
            &src,
            "byte[] _vBytes = Encoding.UTF8.GetBytes(v);",
            "UTF-8 encoding of the string into a managed byte[] before the P/Invoke call",
        );
        assert_source_contains(
            &src,
            "NativeMethods.StringLength(_vBytes, (UIntPtr)_vBytes.Length)",
            "the native call to receive the encoded byte[] and its length as two separate arguments",
        );
        assert_source_contains(
            &src,
            "internal static extern uint StringLength(byte[] v, UIntPtr vLen);",
            "the P/Invoke declaration to split a string param into (byte[], UIntPtr) matching the C ABI",
        );
    }

    /// A `String` return is wire-encoded by Rust into a length-prefixed
    /// `FfiBuf` (i32 length + UTF-8 bytes). The wrapper decodes via
    /// `WireReader.ReadString` and must release the native allocation with
    /// `FreeBuf` even if decoding throws.
    #[test]
    fn emit_string_return_decodes_ffibuf_and_frees() {
        let mut contract = empty_contract();
        contract.functions.push(function_with_types(
            "echo_string",
            vec![("v", TypeExpr::String)],
            ReturnDef::Value(TypeExpr::String),
        ));

        let src = emit_contract(&contract).combined_source();

        assert_source_contains(
            &src,
            "public static string EchoString(string v)",
            "the public wrapper to hide the FfiBuf and expose a normal `string` return",
        );
        assert_source_contains(
            &src,
            "FfiBuf _buf = NativeMethods.EchoString(",
            "the native return captured in an `FfiBuf _buf` local so it can be decoded and freed",
        );
        assert_source_contains(
            &src,
            "new WireReader(_buf).ReadString()",
            "WireReader stateful decode of the FfiBuf-carried string, shared with the record decode path",
        );
        assert_source_contains(
            &src,
            "NativeMethods.FreeBuf(_buf);",
            "a FreeBuf call in a finally block so the Rust allocator reclaims the buffer even if decoding throws",
        );
        assert_source_contains(
            &src,
            "internal static extern FfiBuf EchoString(byte[] v, UIntPtr vLen);",
            "the P/Invoke signature to return FfiBuf rather than a bare string",
        );
    }

    /// The `FfiBuf` struct, `WireReader`, and `FreeBuf` DllImport are only
    /// needed when a module actually traffics in wire-encoded returns —
    /// primitive-only output should not carry the extra helpers.
    #[test]
    fn emit_string_helpers_only_appear_when_strings_are_used() {
        let mut primitive_only = empty_contract();
        primitive_only.functions.push(primitive_function(
            "add",
            vec![("a", PrimitiveType::I32), ("b", PrimitiveType::I32)],
            ReturnDef::Value(TypeExpr::Primitive(PrimitiveType::I32)),
        ));
        let primitive_src = emit_contract(&primitive_only).combined_source();

        assert_source_lacks(
            &primitive_src,
            "FfiBuf",
            "no FfiBuf struct or references in primitive-only output",
        );
        assert_source_lacks(
            &primitive_src,
            "WireReader",
            "no WireReader helper class in primitive-only output",
        );
        assert_source_lacks(
            &primitive_src,
            "FreeBuf",
            "no FreeBuf DllImport in primitive-only output",
        );
        assert_source_lacks(
            &primitive_src,
            "using System.Text;",
            "no System.Text using directive when Encoding.UTF8 is never referenced",
        );

        let mut with_string = empty_contract();
        with_string.functions.push(function_with_types(
            "echo",
            vec![("v", TypeExpr::String)],
            ReturnDef::Value(TypeExpr::String),
        ));
        let string_src = emit_contract(&with_string).combined_source();

        assert_source_contains(
            &string_src,
            "internal struct FfiBuf",
            "the FfiBuf struct when strings are used (mirrors the Rust FfiBuf_u8 layout)",
        );
        assert_source_contains(
            &string_src,
            "WireReader",
            "a WireReader helper when strings are used",
        );
        assert_source_contains(
            &string_src,
            r#"[DllImport(LibName, EntryPoint = "boltffi_free_buf")]"#,
            "a DllImport binding to boltffi_free_buf when strings are used",
        );
        assert_source_contains(
            &string_src,
            "using System.Text;",
            "the System.Text using directive so Encoding.UTF8 resolves in the wrapper and WireReader",
        );
    }

    /// Regression: record-only string usage still needs `System.Text` in
    /// the main file because the shared `WireReader` / `WireWriter`
    /// helpers live there, not in the record file.
    #[test]
    fn emit_record_only_string_fields_import_system_text_in_main_file() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(record_with_fields(
            "person",
            false,
            vec![
                ("name", TypeExpr::String),
                ("age", TypeExpr::Primitive(PrimitiveType::U32)),
            ],
        ));
        contract.functions.push(function_with_types(
            "echo_person",
            vec![("p", TypeExpr::Record(RecordId::new("person")))],
            ReturnDef::Value(TypeExpr::Record(RecordId::new("person"))),
        ));

        let output = emit_contract(&contract);
        let main_source = output
            .files
            .iter()
            .find(|f| f.file_name == "DemoLib.cs")
            .expect("DemoLib.cs")
            .source
            .as_str();

        assert_source_contains(
            main_source,
            "using System.Text;",
            "the main file needs System.Text when record string fields make WireWriter use Encoding.UTF8.GetBytes/GetByteCount",
        );
        assert_source_contains(
            main_source,
            "Marshal.PtrToStringUTF8",
            "WireReader string decode still lives in the main file for record-only string usage",
        );
    }

    /// The shared bounds check avoids `_pos + n` overflow on malformed
    /// large lengths and still routes failures through the backend's
    /// "corrupt wire" exception path.
    #[test]
    fn emit_wire_reader_require_uses_overflow_safe_guard() {
        let mut contract = empty_contract();
        contract.functions.push(function_with_types(
            "echo",
            vec![("v", TypeExpr::String)],
            ReturnDef::Value(TypeExpr::String),
        ));

        let src = emit_contract(&contract).combined_source();

        assert_source_contains(
            &src,
            "if (n < 0 || n > _length - _pos) throw new InvalidOperationException(\"corrupt wire: truncated \" + kind);",
            "WireReader.Require must compare against remaining bytes instead of overflowing `_pos + n`",
        );
    }

    /// Regression: the IR wraps variable-length sizes in
    /// `Sum([Fixed(4), StringLen(v)])`; `StringLen` must render as the
    /// payload byte count alone, not `4 + byte_count`, otherwise a string
    /// field's wire size is over-counted by 4 bytes.
    #[test]
    fn emit_size_expr_for_string_len_renders_payload_only() {
        use crate::ir::ids::ParamName;
        use crate::ir::ops::{SizeExpr, ValueExpr};

        let size = SizeExpr::Sum(vec![
            SizeExpr::Fixed(4),
            SizeExpr::StringLen(ValueExpr::Named(
                ParamName::new("name").as_str().to_string(),
            )),
        ]);
        assert_eq!(
            emit_size_expr(&size),
            "(4 + Encoding.UTF8.GetByteCount(name))"
        );
    }

    // ----- Record tests -----

    use crate::ir::definitions::{FieldDef, RecordDef};
    use crate::ir::ids::{FieldName, RecordId};

    /// Build a record with the given fields. `is_repr_c = true` lets the
    /// IR classify it as blittable when every field is a primitive.
    fn record_with_fields(id: &str, is_repr_c: bool, fields: Vec<(&str, TypeExpr)>) -> RecordDef {
        RecordDef {
            id: RecordId::new(id),
            is_repr_c,
            is_error: false,
            fields: fields
                .into_iter()
                .map(|(name, type_expr)| FieldDef {
                    name: FieldName::new(name),
                    type_expr,
                    doc: None,
                    default: None,
                })
                .collect(),
            constructors: vec![],
            methods: vec![],
            doc: None,
            deprecated: None,
        }
    }

    /// A blittable record (`#[repr(C)]`, all primitive fields) should get
    /// the `[StructLayout(LayoutKind.Sequential)]` attribute so the CLR
    /// lays it out the same way Rust does and can pass it by value across
    /// the P/Invoke boundary without any wire encoding.
    #[test]
    fn emit_blittable_record_gets_struct_layout_attribute() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(record_with_fields(
            "point",
            true,
            vec![
                ("x", TypeExpr::Primitive(PrimitiveType::F64)),
                ("y", TypeExpr::Primitive(PrimitiveType::F64)),
            ],
        ));
        contract.functions.push(function_with_types(
            "echo_point",
            vec![("p", TypeExpr::Record(RecordId::new("point")))],
            ReturnDef::Value(TypeExpr::Record(RecordId::new("point"))),
        ));

        let output = emit_contract(&contract);
        let src = output.combined_source();

        assert_source_contains(
            &src,
            "[StructLayout(LayoutKind.Sequential)]",
            "Sequential layout attribute so Rust's #[repr(C)] layout matches the C# struct",
        );
        assert_source_contains(
            &src,
            "public readonly record struct Point(",
            "readonly record struct declaration — value type with generated equality",
        );
    }

    /// A blittable record used as a function param/return must pass
    /// directly across P/Invoke without any byte[] buffer or FfiBuf. The
    /// wrapper should be a one-liner forwarding to NativeMethods; the
    /// native signature should use the struct type.
    #[test]
    fn emit_blittable_record_passes_by_value_across_p_invoke() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(record_with_fields(
            "point",
            true,
            vec![
                ("x", TypeExpr::Primitive(PrimitiveType::F64)),
                ("y", TypeExpr::Primitive(PrimitiveType::F64)),
            ],
        ));
        contract.functions.push(function_with_types(
            "echo_point",
            vec![("p", TypeExpr::Record(RecordId::new("point")))],
            ReturnDef::Value(TypeExpr::Record(RecordId::new("point"))),
        ));

        let src = emit_contract(&contract).combined_source();

        assert_source_contains(
            &src,
            "public static Point EchoPoint(Point p)",
            "wrapper exposes the blittable record directly",
        );
        assert_source_contains(
            &src,
            "return NativeMethods.EchoPoint(p);",
            "single-line delegating body — no WireWriter, no FfiBuf",
        );
        assert_source_contains(
            &src,
            "internal static extern Point EchoPoint(Point p);",
            "DllImport takes and returns the struct directly",
        );
        assert_source_lacks(
            &src,
            "WireWriter(p.WireEncodedSize())",
            "no WireWriter setup for a blittable param — that would defeat the zero-copy win",
        );
    }

    /// A non-blittable record (one with a string field) must NOT carry
    /// `[StructLayout(Sequential)]` — its memory layout doesn't need to
    /// match Rust's because it travels as wire-encoded bytes, not as a
    /// struct value.
    #[test]
    fn emit_non_blittable_record_omits_struct_layout_attribute() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(record_with_fields(
            "person",
            false,
            vec![
                ("name", TypeExpr::String),
                ("age", TypeExpr::Primitive(PrimitiveType::U32)),
            ],
        ));

        let output = emit_contract(&contract);
        let person_source = output
            .files
            .iter()
            .find(|f| f.file_name == "Person.cs")
            .expect("Person.cs")
            .source
            .as_str();

        assert!(
            !person_source.contains("[StructLayout"),
            "non-blittable record should not carry Sequential layout, but got:\n{person_source}"
        );
        assert!(
            person_source.contains("public readonly record struct Person("),
            "still a record struct just without the layout attribute"
        );
    }

    /// A non-blittable record param travels as a wire-encoded byte array.
    /// The wrapper must: (a) open a `using` WireWriter scoped to the
    /// buffer's lifetime, (b) call the record's `WireEncodeTo`, (c) grab
    /// the bytes, (d) pass `(byte[], UIntPtr)` to native, (e) decode the
    /// return and free the FfiBuf.
    #[test]
    fn emit_non_blittable_record_param_uses_wire_writer_and_byte_array() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(record_with_fields(
            "person",
            false,
            vec![
                ("name", TypeExpr::String),
                ("age", TypeExpr::Primitive(PrimitiveType::U32)),
            ],
        ));
        contract.functions.push(function_with_types(
            "echo_person",
            vec![("p", TypeExpr::Record(RecordId::new("person")))],
            ReturnDef::Value(TypeExpr::Record(RecordId::new("person"))),
        ));

        let src = emit_contract(&contract).combined_source();

        assert_source_contains(
            &src,
            "using var _wire_p = new WireWriter(p.WireEncodedSize());",
            "WireWriter rented with the record's exact encoded size, disposed at scope end",
        );
        assert_source_contains(
            &src,
            "p.WireEncodeTo(_wire_p);",
            "record encodes itself into the WireWriter via its generated method",
        );
        assert_source_contains(
            &src,
            "byte[] _pBytes = _wire_p.ToArray();",
            "bytes materialized before the native call",
        );
        assert_source_contains(
            &src,
            "FfiBuf _buf = NativeMethods.EchoPerson(_pBytes, (UIntPtr)_pBytes.Length);",
            "native call hands the (byte[], UIntPtr) pair",
        );
        assert_source_contains(
            &src,
            "return Person.Decode(new WireReader(_buf));",
            "return decodes the FfiBuf via the record's Decode method",
        );
        assert_source_contains(
            &src,
            "NativeMethods.FreeBuf(_buf);",
            "FreeBuf in finally so Rust reclaims the buffer even on decode failure",
        );
        assert_source_contains(
            &src,
            "internal static extern FfiBuf EchoPerson(byte[] p, UIntPtr pLen);",
            "DllImport signature splits the record into (byte[], UIntPtr) and returns FfiBuf",
        );
    }

    /// A nested record's `WireEncodeTo` must delegate to the inner
    /// record's `WireEncodeTo` via the field accessor, and its `Decode`
    /// must call the inner record's `Decode`. This is the recursive
    /// glue that lets records contain records.
    #[test]
    fn emit_nested_record_encode_decode_delegates_to_inner_record() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(record_with_fields(
            "inner",
            false,
            vec![("label", TypeExpr::String)],
        ));
        contract.catalog.insert_record(record_with_fields(
            "outer",
            false,
            vec![("inner", TypeExpr::Record(RecordId::new("inner")))],
        ));

        let output = emit_contract(&contract);
        let outer = output
            .files
            .iter()
            .find(|f| f.file_name == "Outer.cs")
            .expect("Outer.cs")
            .source
            .as_str();

        assert!(
            outer.contains("Inner.Decode(reader)"),
            "nested field decode walks into the inner record's Decode, but Outer.cs was:\n{outer}"
        );
        assert!(
            outer.contains("this.Inner.WireEncodeTo(wire);"),
            "nested field encode walks into the inner record's WireEncodeTo, but Outer.cs was:\n{outer}"
        );
    }

    /// Record files should only import `System.Text` when a string field
    /// is present (needed for `Encoding.UTF8.GetByteCount` in the size
    /// expression). `TreatWarningsAsErrors` in downstream projects flags
    /// unused usings, so a blittable-only record must stay clean.
    #[test]
    fn emit_record_imports_system_text_only_when_string_fields_present() {
        let mut contract = empty_contract();
        contract.catalog.insert_record(record_with_fields(
            "point",
            true,
            vec![
                ("x", TypeExpr::Primitive(PrimitiveType::F64)),
                ("y", TypeExpr::Primitive(PrimitiveType::F64)),
            ],
        ));
        contract.catalog.insert_record(record_with_fields(
            "person",
            false,
            vec![
                ("name", TypeExpr::String),
                ("age", TypeExpr::Primitive(PrimitiveType::U32)),
            ],
        ));

        let output = emit_contract(&contract);
        let point = output
            .files
            .iter()
            .find(|f| f.file_name == "Point.cs")
            .unwrap()
            .source
            .as_str();
        let person = output
            .files
            .iter()
            .find(|f| f.file_name == "Person.cs")
            .unwrap()
            .source
            .as_str();

        assert!(
            !point.contains("using System.Text;"),
            "Point.cs (blittable, no strings) should not import System.Text"
        );
        assert!(
            person.contains("using System.Text;"),
            "Person.cs (has string field) needs System.Text for Encoding.UTF8.GetByteCount"
        );
        // And the inverse: StructLayout's using stays on blittable only.
        assert!(
            point.contains("using System.Runtime.InteropServices;"),
            "Point.cs uses StructLayout so it imports InteropServices"
        );
        assert!(
            !person.contains("using System.Runtime.InteropServices;"),
            "Person.cs has no StructLayout so it should not import InteropServices"
        );
    }

    /// Functions that mix string and non-string params must only emit the
    /// UTF-8 prep line for the string args and pass non-string args through
    /// unchanged.
    #[test]
    fn emit_mixed_string_and_primitive_params_only_encodes_strings() {
        let mut contract = empty_contract();
        contract.functions.push(function_with_types(
            "repeat_string",
            vec![
                ("v", TypeExpr::String),
                ("count", TypeExpr::Primitive(PrimitiveType::U32)),
            ],
            ReturnDef::Value(TypeExpr::String),
        ));

        let src = emit_contract(&contract).combined_source();

        assert_source_contains(
            &src,
            "byte[] _vBytes = Encoding.UTF8.GetBytes(v);",
            "UTF-8 encoding only for the string param `v`",
        );
        assert_source_lacks(
            &src,
            "Encoding.UTF8.GetBytes(count)",
            "no UTF-8 encoding for the primitive `count` param",
        );
        assert_source_contains(
            &src,
            "NativeMethods.RepeatString(_vBytes, (UIntPtr)_vBytes.Length, count)",
            "the native call to expand only the string into (bytes, length) and pass the primitive through unchanged",
        );
        assert_source_contains(
            &src,
            "internal static extern FfiBuf RepeatString(byte[] v, UIntPtr vLen, uint count);",
            "the P/Invoke signature to split only the string into byte[]+UIntPtr, keeping the primitive uint direct",
        );
    }
}
