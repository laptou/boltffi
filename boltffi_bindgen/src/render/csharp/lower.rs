use std::collections::HashSet;

use boltffi_ffi_rules::naming;

use crate::ir::abi::{AbiCall, AbiParam, AbiRecord, CallId, ParamRole};
use crate::ir::definitions::{FieldDef, FunctionDef, ParamDef, ParamPassing, RecordDef, ReturnDef};
use crate::ir::ids::{FieldName, RecordId};
use crate::ir::ops::{ReadOp, ReadSeq, WriteOp, WriteSeq};
use crate::ir::types::TypeExpr;
use crate::ir::{AbiContract, FfiContract};

use super::emit;
use super::mappings;
use super::plan::{
    CSharpFunction, CSharpModule, CSharpParam, CSharpParamKind, CSharpRecord, CSharpRecordField,
    CSharpReturnKind, CSharpType, CSharpWireWriter,
};
use super::{CSharpOptions, NamingConvention};

/// Transforms the language-agnostic [`FfiContract`] and [`AbiContract`] into
/// a [`CSharpModule`] containing everything the C# templates need to render.
pub struct CSharpLowerer<'a> {
    ffi: &'a FfiContract,
    abi: &'a AbiContract,
    options: &'a CSharpOptions,
    /// Records that are fully supported — every field resolves to a type the
    /// C# backend can currently render. Populated up front because whether
    /// a record is supported can depend on whether *other* records are
    /// supported, so we need a fixed-point pass before lowering individual
    /// functions or records.
    supported_records: HashSet<String>,
}

impl<'a> CSharpLowerer<'a> {
    pub fn new(ffi: &'a FfiContract, abi: &'a AbiContract, options: &'a CSharpOptions) -> Self {
        let supported_records = Self::compute_supported_records(ffi);
        Self {
            ffi,
            abi,
            options,
            supported_records,
        }
    }

    /// Fixed-point computation: a record is supported when every one of its
    /// fields is a supported type. Since fields may themselves be records,
    /// we iterate until the supported set stops growing.
    ///
    /// Mirrors `JavaLowerer::compute_supported_records`.
    fn compute_supported_records(ffi: &FfiContract) -> HashSet<String> {
        let mut supported = HashSet::new();
        let mut changed = true;
        while changed {
            changed = false;
            for record in ffi.catalog.all_records() {
                let id = record.id.as_str().to_string();
                if supported.contains(&id) {
                    continue;
                }
                let all_fields_ok = record.fields.iter().all(|field| match &field.type_expr {
                    TypeExpr::Primitive(_) | TypeExpr::String | TypeExpr::Void => true,
                    TypeExpr::Record(ref_id) => supported.contains(ref_id.as_str()),
                    _ => false,
                });
                if all_fields_ok {
                    supported.insert(id);
                    changed = true;
                }
            }
        }
        supported
    }

    /// Walk the contracts and produce a C# module plan.
    pub fn lower(&self) -> CSharpModule {
        let lib_name = self
            .options
            .library_name
            .clone()
            .unwrap_or_else(|| naming::library_name(&self.ffi.package.name));

        let class_name = NamingConvention::class_name(&self.ffi.package.name);
        let namespace = NamingConvention::namespace(&self.ffi.package.name);
        let prefix = naming::ffi_prefix().to_string();

        let records: Vec<CSharpRecord> = self
            .ffi
            .catalog
            .all_records()
            .filter(|r| self.supported_records.contains(r.id.as_str()))
            .map(|r| self.lower_record(r))
            .collect();

        let functions: Vec<CSharpFunction> = self
            .ffi
            .functions
            .iter()
            .filter_map(|f| self.lower_function(f))
            .collect();

        CSharpModule {
            namespace,
            class_name,
            lib_name,
            prefix,
            records,
            functions,
        }
    }

    /// Converts a Rust FFI function definition into its C# representation,
    /// mapping Rust types to C# types and snake_case names to PascalCase.
    ///
    /// Returns `None` for functions whose signatures include types not yet
    /// supported by the C# backend.
    fn lower_function(&self, function: &FunctionDef) -> Option<CSharpFunction> {
        if function.is_async() {
            return None;
        }

        if !function.params.iter().all(|p| self.is_supported_param(p)) {
            return None;
        }

        let return_type = self.lower_return(&function.returns)?;
        let return_kind = self.return_kind(&function.returns, &return_type);

        let wire_writers = self.wire_writers_for_params(function)?;

        let params: Vec<CSharpParam> = function
            .params
            .iter()
            .map(|p| self.lower_param(p, &wire_writers))
            .collect::<Option<Vec<_>>>()?;

        Some(CSharpFunction {
            name: NamingConvention::method_name(function.id.as_str()),
            ffi_name: naming::function_ffi_name(function.id.as_str()).into_string(),
            params,
            return_type,
            return_kind,
            wire_writers,
        })
    }

    fn return_kind(&self, return_def: &ReturnDef, return_type: &CSharpType) -> CSharpReturnKind {
        if return_type.is_void() {
            return CSharpReturnKind::Void;
        }
        match return_def {
            ReturnDef::Value(TypeExpr::String) => CSharpReturnKind::WireDecodeString,
            ReturnDef::Value(TypeExpr::Record(id)) if !self.is_blittable_record(id) => {
                CSharpReturnKind::WireDecodeRecord {
                    class_name: NamingConvention::class_name(id.as_str()),
                }
            }
            // Primitives, bools, and blittable records are all direct:
            // the CLR marshals them across P/Invoke without any wrapper help.
            _ => CSharpReturnKind::Direct,
        }
    }

    fn is_blittable_record(&self, id: &RecordId) -> bool {
        self.abi_record_for(id).is_some_and(|r| r.is_blittable)
    }

    fn is_supported_param(&self, param: &ParamDef) -> bool {
        param.passing == ParamPassing::Value && self.is_supported_type(&param.type_expr)
    }

    fn is_supported_type(&self, ty: &TypeExpr) -> bool {
        match ty {
            TypeExpr::Primitive(_) | TypeExpr::String | TypeExpr::Void => true,
            TypeExpr::Record(id) => self.supported_records.contains(id.as_str()),
            _ => false,
        }
    }

    fn lower_param(
        &self,
        param: &ParamDef,
        wire_writers: &[CSharpWireWriter],
    ) -> Option<CSharpParam> {
        if param.passing != ParamPassing::Value {
            return None;
        }

        let csharp_type = self.lower_type(&param.type_expr)?;
        let kind = match &param.type_expr {
            TypeExpr::String => CSharpParamKind::Utf8Bytes,
            TypeExpr::Record(id) if !self.is_blittable_record(id) => {
                let writer = wire_writers
                    .iter()
                    .find(|w| w.param_name == param.name.as_str())?;
                CSharpParamKind::WireEncoded {
                    binding_name: writer.bytes_binding_name.clone(),
                }
            }
            // Primitives, bools, and blittable records pass directly —
            // the CLR marshals them across P/Invoke with no extra setup.
            _ => CSharpParamKind::Direct,
        };

        Some(CSharpParam {
            name: NamingConvention::field_name(param.name.as_str()),
            csharp_type,
            kind,
        })
    }

    fn lower_return(&self, return_def: &ReturnDef) -> Option<CSharpType> {
        match return_def {
            ReturnDef::Void => Some(CSharpType::Void),
            ReturnDef::Value(type_expr) => self.lower_type(type_expr),
            ReturnDef::Result { .. } => None,
        }
    }

    fn lower_type(&self, type_expr: &TypeExpr) -> Option<CSharpType> {
        match type_expr {
            TypeExpr::Void => Some(CSharpType::Void),
            TypeExpr::Primitive(primitive) => Some(mappings::csharp_type(*primitive)),
            TypeExpr::String => Some(CSharpType::String),
            TypeExpr::Record(id) if self.supported_records.contains(id.as_str()) => Some(
                CSharpType::Record(NamingConvention::class_name(id.as_str())),
            ),
            _ => None,
        }
    }

    fn lower_record(&self, record: &RecordDef) -> CSharpRecord {
        let class_name = NamingConvention::class_name(record.id.as_str());
        let fields = record
            .fields
            .iter()
            .map(|field| self.lower_record_field(&record.id, field))
            .collect();
        let is_blittable = self.is_blittable_record(&record.id);
        CSharpRecord {
            class_name,
            fields,
            is_blittable,
        }
    }

    fn lower_record_field(&self, record_id: &RecordId, field: &FieldDef) -> CSharpRecordField {
        let decode_seq = self
            .record_field_read_seq(record_id, &field.name)
            .expect("record field decode ops");
        let encode_seq = self
            .record_field_write_seq(record_id, &field.name)
            .expect("record field encode ops");
        let csharp_type = self
            .lower_type(&field.type_expr)
            .expect("record field type must be supported");
        CSharpRecordField {
            name: NamingConvention::property_name(field.name.as_str()),
            csharp_type,
            wire_decode_expr: emit::emit_reader_read(&decode_seq),
            wire_size_expr: emit::emit_size_expr(&encode_seq.size),
            wire_encode_expr: emit::emit_write_expr(&encode_seq, "wire"),
        }
    }

    fn record_field_read_seq(
        &self,
        record_id: &RecordId,
        field_name: &FieldName,
    ) -> Option<ReadSeq> {
        self.abi_record_for(record_id)
            .and_then(|record| match record.decode_ops.ops.first() {
                Some(ReadOp::Record { fields, .. }) => fields
                    .iter()
                    .find(|field| field.name == *field_name)
                    .map(|field| field.seq.clone()),
                _ => None,
            })
    }

    fn record_field_write_seq(
        &self,
        record_id: &RecordId,
        field_name: &FieldName,
    ) -> Option<WriteSeq> {
        self.abi_record_for(record_id)
            .and_then(|record| match record.encode_ops.ops.first() {
                Some(WriteOp::Record { fields, .. }) => fields
                    .iter()
                    .find(|field| field.name == *field_name)
                    .map(|field| field.seq.clone()),
                _ => None,
            })
    }

    fn abi_record_for(&self, record_id: &RecordId) -> Option<&AbiRecord> {
        self.abi
            .records
            .iter()
            .find(|record| record.id == *record_id)
    }

    /// Build one [`CSharpWireWriter`] per record param, in param order.
    /// Returns `None` if the function's ABI call cannot be found (should
    /// not happen for validated contracts).
    fn wire_writers_for_params(&self, function: &FunctionDef) -> Option<Vec<CSharpWireWriter>> {
        let call = self.abi_call_for_function(function)?;
        Some(
            call.params
                .iter()
                .filter_map(|abi_param| self.wire_writer_for_param(abi_param))
                .collect(),
        )
    }

    fn wire_writer_for_param(&self, param: &AbiParam) -> Option<CSharpWireWriter> {
        let encode_ops = match &param.role {
            ParamRole::Input {
                encode_ops: Some(encode_ops),
                ..
            } => encode_ops.clone(),
            _ => return None,
        };
        // Only record params need a WireWriter setup block.
        // Strings keep their existing direct-byte[] path.
        // Blittable record params pass through P/Invoke as struct values
        // and bypass wire encoding entirely.
        let record_id = match encode_ops.ops.first()? {
            WriteOp::Record { id, .. } => id,
            _ => return None,
        };
        if self.is_blittable_record(record_id) {
            return None;
        }
        let param_name = param.name.as_str().to_string();
        let binding_name = format!("_wire_{}", param_name);
        let bytes_binding_name = format!("_{}Bytes", NamingConvention::field_name(&param_name));
        let encode_expr = emit::emit_write_expr(&encode_ops, &binding_name);
        Some(CSharpWireWriter {
            binding_name,
            bytes_binding_name,
            param_name,
            size_expr: emit::emit_size_expr(&encode_ops.size),
            encode_expr,
        })
    }

    fn abi_call_for_function(&self, function: &FunctionDef) -> Option<&AbiCall> {
        self.abi.calls.iter().find(|call| match &call.id {
            CallId::Function(id) => id == &function.id,
            _ => false,
        })
    }
}
