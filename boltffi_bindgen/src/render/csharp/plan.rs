use std::fmt;

use boltffi_ffi_rules::naming::{LibraryName, Name};

/// Represents a lowered C# module, containing everything the templates need
/// to render a `.cs` file.
#[derive(Debug, Clone)]
pub struct CSharpModule {
    /// C# namespace for the generated file (e.g., `"MyApp"`).
    pub namespace: String,
    /// Top-level class name (e.g., `"MyApp"`).
    pub class_name: String,
    /// Native library name used in `[DllImport("...")]` declarations.
    pub lib_name: Name<LibraryName>,
    /// FFI symbol prefix (e.g., `"boltffi"`).
    pub prefix: String,
    /// Records exposed by the module. Each record is rendered to its own
    /// `.cs` file as a `readonly record struct`.
    pub records: Vec<CSharpRecord>,
    /// Top-level primitive functions. Used by both the public wrapper class
    /// and the `[DllImport]` native declarations — C# P/Invoke passes
    /// primitives directly, so one struct serves both layers.
    pub functions: Vec<CSharpFunction>,
}

impl CSharpModule {
    pub fn has_functions(&self) -> bool {
        !self.functions.is_empty()
    }

    /// Whether the shared runtime helpers need `System.Text`.
    ///
    /// Top-level string params use `Encoding.UTF8.GetBytes` in the wrapper,
    /// and `WireWriter` uses `Encoding.UTF8.GetByteCount` / `GetBytes` when
    /// encoding string fields of a record. Decoding no longer needs
    /// `System.Text` — `WireReader` reads strings through
    /// `Marshal.PtrToStringUTF8`.
    pub fn needs_system_text(&self) -> bool {
        self.functions
            .iter()
            .any(|f| f.params.iter().any(|p| p.csharp_type.is_string()))
            || self.records.iter().any(CSharpRecord::has_string_fields)
    }

    /// Whether any function takes a wire-encoded record param. Blittable
    /// record params pass through the CLR as direct struct values and do
    /// not contribute here.
    pub fn has_wire_params(&self) -> bool {
        self.functions.iter().any(|f| !f.wire_writers.is_empty())
    }

    /// Whether any function returns through an `FfiBuf` — a wire-decoded
    /// string or non-blittable record. Blittable records come back as
    /// direct struct values and do not count here.
    pub fn has_ffi_buf_returns(&self) -> bool {
        self.functions
            .iter()
            .any(|f| f.return_kind.native_returns_ffi_buf())
    }

    /// Whether the `FfiBuf` struct and `FreeBuf` DllImport are emitted.
    /// Needed for wire-encoded returns, and pulled in whenever a record
    /// exists so the `WireReader` (which takes `FfiBuf`) compiles.
    pub fn needs_ffi_buf(&self) -> bool {
        self.has_ffi_buf_returns() || !self.records.is_empty()
    }

    /// Whether the stateful `WireReader` helper is emitted. Needed for
    /// wire-decoded returns and for any record's `Decode` method.
    pub fn needs_wire_reader(&self) -> bool {
        self.has_ffi_buf_returns() || !self.records.is_empty()
    }

    /// Whether the `WireWriter` helper is emitted. Needed for wire-encoded
    /// params and for any record's `WireEncodeTo` method.
    pub fn needs_wire_writer(&self) -> bool {
        self.has_wire_params() || !self.records.is_empty()
    }
}

/// A C# type keyword. Includes `Void` so return types and value types share
/// one enum; params never carry `Void` because the lowerer rejects it before
/// constructing a [`CSharpParam`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CSharpType {
    Void,
    Bool,
    SByte,
    Byte,
    Short,
    UShort,
    Int,
    UInt,
    Long,
    ULong,
    NInt,
    NUInt,
    Float,
    Double,
    String,
    /// A user-defined record, identified by its rendered PascalCase class
    /// name (e.g., `"Point"`).
    Record(String),
}

impl CSharpType {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Void => "void",
            Self::Bool => "bool",
            Self::SByte => "sbyte",
            Self::Byte => "byte",
            Self::Short => "short",
            Self::UShort => "ushort",
            Self::Int => "int",
            Self::UInt => "uint",
            Self::Long => "long",
            Self::ULong => "ulong",
            Self::NInt => "nint",
            Self::NUInt => "nuint",
            Self::Float => "float",
            Self::Double => "double",
            Self::String => "string",
            Self::Record(name) => name.as_str(),
        }
    }

    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Bool)
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Self::String)
    }

    pub fn is_record(&self) -> bool {
        matches!(self, Self::Record(_))
    }
}

impl fmt::Display for CSharpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// A record (Rust struct) exposed as a C# `readonly record struct`.
///
/// Each record is emitted to its own `.cs` file. Blittable records (all
/// fields are primitives, layout matches Rust's `#[repr(C)]`) get a
/// `[StructLayout(LayoutKind.Sequential)]` attribute so the CLR passes
/// them directly across the P/Invoke boundary by value — no wire encoding
/// needed. Non-blittable records carry `Decode` / `WireEncodedSize` /
/// `WireEncodeTo` members and travel as wire-encoded buffers.
#[derive(Debug, Clone)]
pub struct CSharpRecord {
    /// PascalCase class name (e.g., `"Point"`).
    pub class_name: String,
    /// The record's fields, in declaration order.
    pub fields: Vec<CSharpRecordField>,
    /// Whether the record can cross the P/Invoke boundary as a direct
    /// `[StructLayout(Sequential)]` value. True when the Rust type is
    /// `#[repr(C)]` with blittable fields only.
    pub is_blittable: bool,
}

impl CSharpRecord {
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Wire helpers are only needed for non-blittable records. Blittable
    /// records skip wire encoding entirely.
    pub fn needs_wire_helpers(&self) -> bool {
        !self.is_blittable
    }

    /// Whether the record has at least one string-typed field. Used by the
    /// record template to decide whether to import `System.Text` (for
    /// `Encoding.UTF8.GetByteCount`). Required because
    /// `TreatWarningsAsErrors` flags unused usings.
    pub fn has_string_fields(&self) -> bool {
        self.fields.iter().any(|f| f.csharp_type.is_string())
    }
}

/// A field on a [`CSharpRecord`]. All wire expressions are pre-rendered by
/// the lowerer so the template can paste them verbatim.
#[derive(Debug, Clone)]
pub struct CSharpRecordField {
    /// PascalCase property name (e.g., `"X"`). Records use PascalCase
    /// property names, not camelCase, matching idiomatic C# record syntax.
    pub name: String,
    /// C# type of the field.
    pub csharp_type: CSharpType,
    /// Expression that decodes this field from a `WireReader`
    /// (e.g., `"reader.ReadF64()"` or `"Point.Decode(reader)"`).
    pub wire_decode_expr: String,
    /// Expression that produces the wire-encoded byte size of this field
    /// (e.g., `"8"`, `"WireWriter.StringWireSize(this.Name)"`).
    pub wire_size_expr: String,
    /// Statement that writes this field to a `WireWriter` named `wire`
    /// (e.g., `"wire.WriteF64(this.X)"`).
    pub wire_encode_expr: String,
}

/// A primitive function binding. Serves double duty: the template uses `name`
/// and C# types for the public static method, and `ffi_name` for the
/// `[DllImport]` entry point.
#[derive(Debug, Clone)]
pub struct CSharpFunction {
    /// PascalCase method name (e.g., `"EchoI32"`).
    pub name: String,
    /// Parameters with C# types.
    pub params: Vec<CSharpParam>,
    /// C# return type as it appears in the public wrapper signature.
    pub return_type: CSharpType,
    /// How the return value crosses the ABI. Drives how the wrapper body
    /// decodes the native return and what the `[DllImport]` signature looks
    /// like.
    pub return_kind: CSharpReturnKind,
    /// The C symbol name (e.g., `"boltffi_echo_i32"`).
    pub ffi_name: String,
    /// For each non-blittable record param, the setup code that wire-encodes
    /// it into a `byte[]` before the native call. Empty if the function has
    /// no wire-encoded params (blittable record params count as direct and
    /// do not appear here).
    pub wire_writers: Vec<CSharpWireWriter>,
}

impl CSharpFunction {
    pub fn is_void(&self) -> bool {
        matches!(self.return_kind, CSharpReturnKind::Void)
    }

    /// Comma-joined param declarations as they appear in the public
    /// wrapper signature.
    pub fn wrapper_param_list(&self) -> String {
        self.params
            .iter()
            .map(CSharpParam::wrapper_declaration)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Comma-joined param declarations as they appear in the
    /// `[DllImport]` native signature.
    pub fn native_param_list(&self) -> String {
        self.params
            .iter()
            .map(CSharpParam::native_declaration)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Comma-joined call arguments handed to the native invocation.
    pub fn native_call_args(&self) -> String {
        self.params
            .iter()
            .map(CSharpParam::native_call_arg)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The return type used in the `[DllImport]` signature. Wire-encoded
    /// returns come back as an `FfiBuf`; everything else (primitives,
    /// bools, blittable records) uses the C# type directly.
    pub fn native_return_type(&self) -> String {
        if self.return_kind.native_returns_ffi_buf() {
            "FfiBuf".to_string()
        } else {
            self.return_type.to_string()
        }
    }
}

/// How a function's return value is delivered across the ABI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CSharpReturnKind {
    /// No return value.
    Void,
    /// Returned directly. Primitives, bools, and blittable records all
    /// share this path — the CLR already knows how to marshal them.
    Direct,
    /// The native function returns an `FfiBuf`. The wrapper copies the
    /// bytes into a managed `string` via `WireReader.ReadString` and
    /// frees the buffer.
    WireDecodeString,
    /// The native function returns an `FfiBuf` carrying a wire-encoded
    /// record. The wrapper wraps it in a `WireReader` and calls
    /// `{class_name}.Decode(reader)` to reconstruct the record.
    WireDecodeRecord { class_name: String },
}

impl CSharpReturnKind {
    pub fn is_void(&self) -> bool {
        matches!(self, Self::Void)
    }

    pub fn is_direct(&self) -> bool {
        matches!(self, Self::Direct)
    }

    pub fn is_wire_decode_string(&self) -> bool {
        matches!(self, Self::WireDecodeString)
    }

    pub fn is_wire_decode_record(&self) -> bool {
        matches!(self, Self::WireDecodeRecord { .. })
    }

    /// Whether the native (DllImport) signature returns an `FfiBuf`.
    pub fn native_returns_ffi_buf(&self) -> bool {
        matches!(self, Self::WireDecodeString | Self::WireDecodeRecord { .. })
    }

    /// For `WireDecodeRecord`, the decoded class name (e.g., `"Point"`);
    /// `None` for every other kind. Templates use this to emit
    /// `{class_name}.Decode`.
    pub fn decode_class_name(&self) -> Option<&str> {
        match self {
            Self::WireDecodeRecord { class_name } => Some(class_name),
            _ => None,
        }
    }

    /// The `return` statement that goes inside the `try` block of a
    /// wire-decoded call body. `buf_var` is the local name holding the
    /// `FfiBuf` from the native call. Returns `None` for non-wire-decoded
    /// kinds so callers cannot misuse an empty-string fallback as valid
    /// generated code.
    pub fn wire_decode_return(&self, buf_var: &str) -> Option<String> {
        match self {
            Self::WireDecodeString => {
                Some(format!("return new WireReader({}).ReadString();", buf_var))
            }
            Self::WireDecodeRecord { class_name } => Some(format!(
                "return {}.Decode(new WireReader({}));",
                class_name, buf_var
            )),
            _ => None,
        }
    }
}

/// A parameter in a C# function.
#[derive(Debug, Clone)]
pub struct CSharpParam {
    /// camelCase parameter name, keyword-escaped with `@` if needed.
    pub name: String,
    /// C# type as it appears in the public wrapper signature.
    pub csharp_type: CSharpType,
    /// How the parameter crosses the ABI.
    pub kind: CSharpParamKind,
}

impl CSharpParam {
    /// Declaration as it appears in the public wrapper signature,
    /// e.g. `"int value"`, `"string v"`, `"Point point"`.
    pub fn wrapper_declaration(&self) -> String {
        format!("{} {}", self.csharp_type, self.name)
    }

    /// Declaration as it appears in the `[DllImport]` signature — this
    /// is where the different marshalling paths diverge:
    /// - Primitives and blittable records pass through directly.
    /// - Bool needs the `[MarshalAs(UnmanagedType.I1)]` attribute
    ///   because P/Invoke defaults to the 4-byte Win32 BOOL.
    /// - Strings and wire-encoded records are split into
    ///   `(byte[] x, UIntPtr xLen)`.
    pub fn native_declaration(&self) -> String {
        match &self.kind {
            CSharpParamKind::Utf8Bytes | CSharpParamKind::WireEncoded { .. } => {
                format!("byte[] {name}, UIntPtr {name}Len", name = self.name)
            }
            CSharpParamKind::Direct if self.csharp_type.is_bool() => {
                format!("[MarshalAs(UnmanagedType.I1)] bool {}", self.name)
            }
            CSharpParamKind::Direct => {
                format!("{} {}", self.csharp_type, self.name)
            }
        }
    }

    /// The argument expression to hand to the native call — either the
    /// raw param, or the pre-encoded byte array plus its length.
    pub fn native_call_arg(&self) -> String {
        match &self.kind {
            CSharpParamKind::Direct => self.name.clone(),
            CSharpParamKind::Utf8Bytes => {
                let buf = format!("_{}Bytes", self.name);
                format!("{buf}, (UIntPtr){buf}.Length")
            }
            CSharpParamKind::WireEncoded { binding_name } => {
                format!("{binding_name}, (UIntPtr){binding_name}.Length")
            }
        }
    }

    /// The one-line setup statement that prepares this param before the
    /// native call, or `None` when the param passes through directly.
    /// UTF-8 encoding is the only inline setup; record wire encoding
    /// needs a `using` block and is handled separately via
    /// [`CSharpFunction::wire_writers`].
    pub fn setup_statement(&self) -> Option<String> {
        match &self.kind {
            CSharpParamKind::Utf8Bytes => Some(format!(
                "byte[] _{name}Bytes = Encoding.UTF8.GetBytes({name});",
                name = self.name
            )),
            _ => None,
        }
    }
}

/// How a parameter is marshalled across the C# / C ABI boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CSharpParamKind {
    /// Passed directly as a primitive (bool, int, double, etc.).
    Direct,
    /// A managed `string` that must be UTF-8 encoded into a `byte[]`
    /// and passed as `(byte[], UIntPtr)` to the native call.
    Utf8Bytes,
    /// A record that must be wire-encoded into a `byte[]` by a
    /// `WireWriter` and passed as `(byte[], UIntPtr)`. `binding_name`
    /// is the local variable holding the encoded byte array.
    WireEncoded { binding_name: String },
}

/// Bookkeeping for a single record param that must be wire-encoded into a
/// `byte[]` before the native call. The template wraps these setup lines
/// in a `using` block so each `WireWriter` is disposed (and its rented
/// buffer recycled) even if the native call throws.
#[derive(Debug, Clone)]
pub struct CSharpWireWriter {
    /// The `_wire_foo` local name for the `WireWriter` instance.
    pub binding_name: String,
    /// The `_fooBytes` local name for the resulting `byte[]`.
    pub bytes_binding_name: String,
    /// The original (camelCase) param name, used to find the corresponding
    /// `CSharpParam` at render time.
    pub param_name: String,
    /// Expression rendered against the param that returns its wire-encoded
    /// byte size (e.g., `"point.WireEncodedSize()"`).
    pub size_expr: String,
    /// Statement that writes the param's contents into the `WireWriter`
    /// named by `binding_name` (e.g., `"point.WireEncodeTo(_wire_point)"`).
    pub encode_expr: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn function_with_return(
        return_type: CSharpType,
        return_kind: CSharpReturnKind,
    ) -> CSharpFunction {
        CSharpFunction {
            name: "Test".to_string(),
            params: vec![],
            return_type,
            return_kind,
            ffi_name: "boltffi_test".to_string(),
            wire_writers: vec![],
        }
    }

    fn param(name: &str, csharp_type: CSharpType, kind: CSharpParamKind) -> CSharpParam {
        CSharpParam {
            name: name.to_string(),
            csharp_type,
            kind,
        }
    }

    #[rstest]
    #[case::void(CSharpType::Void, CSharpReturnKind::Void, true)]
    #[case::int(CSharpType::Int, CSharpReturnKind::Direct, false)]
    #[case::bool(CSharpType::Bool, CSharpReturnKind::Direct, false)]
    #[case::double(CSharpType::Double, CSharpReturnKind::Direct, false)]
    fn is_void(
        #[case] return_type: CSharpType,
        #[case] return_kind: CSharpReturnKind,
        #[case] expected: bool,
    ) {
        assert_eq!(
            function_with_return(return_type, return_kind).is_void(),
            expected
        );
    }

    #[test]
    fn record_type_display_uses_class_name() {
        let ty = CSharpType::Record("Point".to_string());
        assert_eq!(ty.to_string(), "Point");
        assert!(ty.is_record());
    }

    // ----- CSharpParam render helpers -----

    #[test]
    fn wrapper_declaration_puts_type_before_name() {
        let p = param("value", CSharpType::Int, CSharpParamKind::Direct);
        assert_eq!(p.wrapper_declaration(), "int value");
    }

    #[test]
    fn wrapper_declaration_uses_record_class_name() {
        let p = param(
            "point",
            CSharpType::Record("Point".to_string()),
            CSharpParamKind::Direct,
        );
        assert_eq!(p.wrapper_declaration(), "Point point");
    }

    /// Direct primitives pass through the native declaration unchanged.
    #[test]
    fn native_declaration_direct_primitive_matches_wrapper() {
        let p = param("value", CSharpType::Int, CSharpParamKind::Direct);
        assert_eq!(p.native_declaration(), "int value");
    }

    /// P/Invoke marshals `bool` as a 4-byte Win32 BOOL by default, but the
    /// C ABI uses a 1-byte native bool, so the `DllImport` signature must
    /// force `UnmanagedType.I1`. The public wrapper side stays plain.
    #[test]
    fn native_declaration_bool_gets_marshal_attribute() {
        let p = param("flag", CSharpType::Bool, CSharpParamKind::Direct);
        assert_eq!(
            p.native_declaration(),
            "[MarshalAs(UnmanagedType.I1)] bool flag"
        );
    }

    /// Blittable record params use `Direct` kind and pass by value, so the
    /// native declaration is just the struct name — no byte[] split.
    #[test]
    fn native_declaration_blittable_record_passes_by_value() {
        let p = param(
            "point",
            CSharpType::Record("Point".to_string()),
            CSharpParamKind::Direct,
        );
        assert_eq!(p.native_declaration(), "Point point");
    }

    /// String params split into two arguments to match the C ABI
    /// `(const uint8_t* ptr, uintptr_t len)`.
    #[test]
    fn native_declaration_string_splits_into_bytes_and_length() {
        let p = param("v", CSharpType::String, CSharpParamKind::Utf8Bytes);
        assert_eq!(p.native_declaration(), "byte[] v, UIntPtr vLen");
    }

    /// Wire-encoded record params use the same `byte[] + UIntPtr` split
    /// as strings because the C ABI signature is identical.
    #[test]
    fn native_declaration_wire_encoded_record_splits_into_bytes_and_length() {
        let p = param(
            "person",
            CSharpType::Record("Person".to_string()),
            CSharpParamKind::WireEncoded {
                binding_name: "_personBytes".to_string(),
            },
        );
        assert_eq!(p.native_declaration(), "byte[] person, UIntPtr personLen");
    }

    #[test]
    fn native_call_arg_direct_passes_name() {
        let p = param("value", CSharpType::Int, CSharpParamKind::Direct);
        assert_eq!(p.native_call_arg(), "value");
    }

    #[test]
    fn native_call_arg_utf8_bytes_passes_buffer_and_length() {
        let p = param("v", CSharpType::String, CSharpParamKind::Utf8Bytes);
        assert_eq!(p.native_call_arg(), "_vBytes, (UIntPtr)_vBytes.Length");
    }

    #[test]
    fn native_call_arg_wire_encoded_uses_binding_name() {
        let p = param(
            "person",
            CSharpType::Record("Person".to_string()),
            CSharpParamKind::WireEncoded {
                binding_name: "_personBytes".to_string(),
            },
        );
        assert_eq!(
            p.native_call_arg(),
            "_personBytes, (UIntPtr)_personBytes.Length"
        );
    }

    /// Only UTF-8 string params have an inline setup statement. Direct
    /// params need no prep; wire-encoded records use a `using` block
    /// that is emitted around the call, not as a flat setup line.
    #[rstest]
    #[case::direct(CSharpParamKind::Direct, None)]
    #[case::wire_encoded(
        CSharpParamKind::WireEncoded { binding_name: "_personBytes".to_string() },
        None,
    )]
    fn setup_statement_non_string_has_none(
        #[case] kind: CSharpParamKind,
        #[case] expected: Option<&str>,
    ) {
        let p = param("x", CSharpType::Int, kind);
        assert_eq!(p.setup_statement().as_deref(), expected);
    }

    #[test]
    fn setup_statement_utf8_bytes_encodes_string() {
        let p = param("v", CSharpType::String, CSharpParamKind::Utf8Bytes);
        assert_eq!(
            p.setup_statement().as_deref(),
            Some("byte[] _vBytes = Encoding.UTF8.GetBytes(v);"),
        );
    }

    // ----- CSharpFunction render helpers -----

    fn function_with_params(
        params: Vec<CSharpParam>,
        return_type: CSharpType,
        return_kind: CSharpReturnKind,
    ) -> CSharpFunction {
        CSharpFunction {
            name: "Test".to_string(),
            params,
            return_type,
            return_kind,
            ffi_name: "boltffi_test".to_string(),
            wire_writers: vec![],
        }
    }

    #[test]
    fn wrapper_param_list_joins_with_comma_space() {
        let f = function_with_params(
            vec![
                param("a", CSharpType::Int, CSharpParamKind::Direct),
                param("b", CSharpType::String, CSharpParamKind::Utf8Bytes),
            ],
            CSharpType::Void,
            CSharpReturnKind::Void,
        );
        assert_eq!(f.wrapper_param_list(), "int a, string b");
    }

    #[test]
    fn wrapper_param_list_empty_for_no_params() {
        let f = function_with_params(vec![], CSharpType::Void, CSharpReturnKind::Void);
        assert_eq!(f.wrapper_param_list(), "");
    }

    /// The native param list exposes each slot's marshalling shape — a
    /// string expands to a pair, bool gets a MarshalAs, and primitives
    /// stay bare. This is the one place the different shapes must line
    /// up, so we pin it with a mixed-shape case.
    #[test]
    fn native_param_list_expands_each_slot_by_kind() {
        let f = function_with_params(
            vec![
                param("flag", CSharpType::Bool, CSharpParamKind::Direct),
                param("v", CSharpType::String, CSharpParamKind::Utf8Bytes),
                param("count", CSharpType::UInt, CSharpParamKind::Direct),
                param(
                    "person",
                    CSharpType::Record("Person".to_string()),
                    CSharpParamKind::WireEncoded {
                        binding_name: "_personBytes".to_string(),
                    },
                ),
            ],
            CSharpType::Void,
            CSharpReturnKind::Void,
        );
        assert_eq!(
            f.native_param_list(),
            "[MarshalAs(UnmanagedType.I1)] bool flag, byte[] v, UIntPtr vLen, uint count, byte[] person, UIntPtr personLen",
        );
    }

    #[test]
    fn native_call_args_mirror_param_shapes() {
        let f = function_with_params(
            vec![
                param("v", CSharpType::String, CSharpParamKind::Utf8Bytes),
                param("count", CSharpType::UInt, CSharpParamKind::Direct),
            ],
            CSharpType::Void,
            CSharpReturnKind::Void,
        );
        assert_eq!(
            f.native_call_args(),
            "_vBytes, (UIntPtr)_vBytes.Length, count",
        );
    }

    /// Wire-encoded returns (string, non-blittable record) come back as
    /// an `FfiBuf` in the native signature regardless of the wrapper's
    /// public return type.
    #[rstest]
    #[case::void(CSharpType::Void, CSharpReturnKind::Void, "void")]
    #[case::primitive(CSharpType::Int, CSharpReturnKind::Direct, "int")]
    #[case::blittable_record(
        CSharpType::Record("Point".to_string()),
        CSharpReturnKind::Direct,
        "Point",
    )]
    #[case::string(CSharpType::String, CSharpReturnKind::WireDecodeString, "FfiBuf")]
    #[case::wire_record(
        CSharpType::Record("Person".to_string()),
        CSharpReturnKind::WireDecodeRecord { class_name: "Person".to_string() },
        "FfiBuf",
    )]
    fn native_return_type_reflects_ffi_buf_paths(
        #[case] return_type: CSharpType,
        #[case] return_kind: CSharpReturnKind,
        #[case] expected: &str,
    ) {
        assert_eq!(
            function_with_return(return_type, return_kind).native_return_type(),
            expected
        );
    }

    #[test]
    fn wire_decode_return_for_string_uses_read_string() {
        let kind = CSharpReturnKind::WireDecodeString;
        assert_eq!(
            kind.wire_decode_return("_buf").as_deref(),
            Some("return new WireReader(_buf).ReadString();"),
        );
    }

    #[test]
    fn wire_decode_return_for_record_calls_decode() {
        let kind = CSharpReturnKind::WireDecodeRecord {
            class_name: "Person".to_string(),
        };
        assert_eq!(
            kind.wire_decode_return("_buf").as_deref(),
            Some("return Person.Decode(new WireReader(_buf));"),
        );
    }

    #[rstest]
    #[case::void(CSharpReturnKind::Void)]
    #[case::direct(CSharpReturnKind::Direct)]
    fn wire_decode_return_none_for_non_wire_kinds(#[case] kind: CSharpReturnKind) {
        assert_eq!(kind.wire_decode_return("_buf"), None);
    }

    #[test]
    fn decode_class_name_some_only_for_wire_decode_record() {
        assert_eq!(
            CSharpReturnKind::WireDecodeRecord {
                class_name: "Point".to_string()
            }
            .decode_class_name(),
            Some("Point"),
        );
        assert_eq!(CSharpReturnKind::WireDecodeString.decode_class_name(), None);
        assert_eq!(CSharpReturnKind::Void.decode_class_name(), None);
        assert_eq!(CSharpReturnKind::Direct.decode_class_name(), None);
    }
}
