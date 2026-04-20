//! Askama template definitions that map plan structs to `.txt` template files.
//!
//! Snapshot tests live next to the template declarations so that a
//! template change shows up as a single `.snap` diff rather than rippling
//! across every emit-level test. Plan-level unit tests remain in
//! [`plan`](super::plan); template tests pin the *rendered shape* given a
//! specific plan fixture.

use askama::Template;

use super::plan::{CSharpModule, CSharpRecord};

/// Renders the file header: auto-generated comment, `using` directives,
/// and namespace declaration.
#[derive(Template)]
#[template(path = "render_csharp/preamble.txt", escape = "none")]
pub struct PreambleTemplate<'a> {
    pub module: &'a CSharpModule,
}

/// Renders the public static wrapper class with methods that delegate
/// to the native P/Invoke declarations.
#[derive(Template)]
#[template(path = "render_csharp/functions.txt", escape = "none")]
pub struct FunctionsTemplate<'a> {
    pub module: &'a CSharpModule,
}

/// Renders the `NativeMethods` static class containing `[DllImport]`
/// declarations for the C FFI functions.
#[derive(Template)]
#[template(path = "render_csharp/native.txt", escape = "none")]
pub struct NativeTemplate<'a> {
    pub module: &'a CSharpModule,
}

/// Renders a single record as a standalone `.cs` file. Each record becomes
/// a `readonly record struct`, with a `[StructLayout(Sequential)]`
/// attribute for blittable records (passed directly across P/Invoke) and
/// wire encode/decode helpers for the wire-encoded path.
#[derive(Template)]
#[template(path = "render_csharp/record.txt", escape = "none")]
pub struct RecordTemplate<'a> {
    pub record: &'a CSharpRecord,
    pub namespace: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::csharp::plan::{CSharpRecord, CSharpRecordField, CSharpType};

    fn record_field(
        name: &str,
        csharp_type: CSharpType,
        decode: &str,
        size: &str,
        encode: &str,
    ) -> CSharpRecordField {
        CSharpRecordField {
            name: name.to_string(),
            csharp_type,
            wire_decode_expr: decode.to_string(),
            wire_size_expr: size.to_string(),
            wire_encode_expr: encode.to_string(),
        }
    }

    /// Point: the canonical blittable record — two f64 fields, `#[repr(C)]`
    /// in Rust. Carries `[StructLayout(Sequential)]` and still emits wire
    /// helpers so it can be embedded inside a non-blittable record's
    /// wire encode/decode path without a second code shape.
    #[test]
    fn snapshot_blittable_record_point() {
        let record = CSharpRecord {
            class_name: "Point".to_string(),
            is_blittable: true,
            fields: vec![
                record_field(
                    "X",
                    CSharpType::Double,
                    "reader.ReadF64()",
                    "8",
                    "wire.WriteF64(this.X)",
                ),
                record_field(
                    "Y",
                    CSharpType::Double,
                    "reader.ReadF64()",
                    "8",
                    "wire.WriteF64(this.Y)",
                ),
            ],
        };
        let template = RecordTemplate {
            record: &record,
            namespace: "Demo",
        };
        insta::assert_snapshot!(template.render().unwrap());
    }

    /// Person: the canonical non-blittable record — a string field (which
    /// forces the wire path) plus a primitive. No StructLayout attribute.
    /// Imports `System.Text` because the size expression uses
    /// `Encoding.UTF8.GetByteCount`.
    #[test]
    fn snapshot_non_blittable_record_person_with_string() {
        let record = CSharpRecord {
            class_name: "Person".to_string(),
            is_blittable: false,
            fields: vec![
                record_field(
                    "Name",
                    CSharpType::String,
                    "reader.ReadString()",
                    "(4 + Encoding.UTF8.GetByteCount(this.Name))",
                    "wire.WriteString(this.Name)",
                ),
                record_field(
                    "Age",
                    CSharpType::UInt,
                    "reader.ReadU32()",
                    "4",
                    "wire.WriteU32(this.Age)",
                ),
            ],
        };
        let template = RecordTemplate {
            record: &record,
            namespace: "Demo",
        };
        insta::assert_snapshot!(template.render().unwrap());
    }

    /// Line: a record whose fields are themselves records. The decode
    /// expression for a record-typed field is `Point.Decode(reader)` and
    /// the encode is `this.Start.WireEncodeTo(wire)` — the recursive
    /// glue that lets records compose.
    #[test]
    fn snapshot_nested_record_line() {
        let record = CSharpRecord {
            class_name: "Line".to_string(),
            is_blittable: false,
            fields: vec![
                record_field(
                    "Start",
                    CSharpType::Record("Point".to_string()),
                    "Point.Decode(reader)",
                    "16",
                    "this.Start.WireEncodeTo(wire)",
                ),
                record_field(
                    "End",
                    CSharpType::Record("Point".to_string()),
                    "Point.Decode(reader)",
                    "16",
                    "this.End.WireEncodeTo(wire)",
                ),
            ],
        };
        let template = RecordTemplate {
            record: &record,
            namespace: "Demo",
        };
        insta::assert_snapshot!(template.render().unwrap());
    }

    /// A fieldless record: the template must still produce valid C# —
    /// `WireEncodedSize` returns 0 and `WireEncodeTo` is an empty method.
    #[test]
    fn snapshot_empty_record() {
        let record = CSharpRecord {
            class_name: "Unit".to_string(),
            is_blittable: true,
            fields: vec![],
        };
        let template = RecordTemplate {
            record: &record,
            namespace: "Demo",
        };
        insta::assert_snapshot!(template.render().unwrap());
    }
}
