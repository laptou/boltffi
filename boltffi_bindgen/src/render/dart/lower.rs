use crate::{
    ir::{
        AbiCall, AbiContract, AbiParam, AbiRecord, AbiType, CallId, FfiContract, FieldDef,
        FieldName, FieldReadOp, FunctionId, OffsetExpr, ReadOp, ReadSeq, RecordDef, RecordId,
        WriteOp, WriteSeq,
    },
    render::dart::{
        DartBlittableField, DartBlittableLayout, DartLibrary, DartNative, DartNativeFunction,
        DartNativeFunctionParam, DartNativeType, DartRecord, DartRecordField, NamingConvention,
    },
};

pub struct DartLowerer<'a> {
    ffi: &'a FfiContract,
    abi: &'a AbiContract,
    package_name: &'a str,
}

impl<'a> DartLowerer<'a> {
    pub fn new(ffi: &'a FfiContract, abi: &'a AbiContract, package_name: &'a str) -> Self {
        Self {
            ffi,
            abi,
            package_name,
        }
    }

    fn lower_native_function_param(&self, abi_param: &AbiParam) -> DartNativeFunctionParam {
        DartNativeFunctionParam {
            name: abi_param.name.to_string(),
            native_type: DartNativeType::from_abi_param(abi_param),
        }
    }

    fn lower_native_function(&self, abi_call: &AbiCall) -> DartNativeFunction {
        let symbol = abi_call.symbol.to_string();

        let params = abi_call
            .params
            .iter()
            .map(|p| self.lower_native_function_param(p))
            .collect();

        let is_not_leaf = abi_call.params.iter().any(|p| {
            matches!(
                p.abi_type,
                AbiType::InlineCallbackFn { .. } | AbiType::CallbackHandle
            )
        });

        DartNativeFunction {
            symbol,
            params,
            return_type: DartNativeType::abi_call_return_type(abi_call),
            is_leaf: !is_not_leaf,
        }
    }

    pub fn abi_call_for_function(&self, function: &FunctionId) -> &AbiCall {
        self.abi
            .calls
            .iter()
            .find(|c| match &c.id {
                CallId::Function(id) => id == function,
                _ => false,
            })
            .unwrap()
    }

    fn abi_record_for(&self, record_id: &RecordId) -> Option<&AbiRecord> {
        self.abi
            .records
            .iter()
            .find(|record| record.id == *record_id)
    }

    fn record_field_read_seq(
        &self,
        abi_record: &AbiRecord,
        field_name: &FieldName,
    ) -> Option<ReadSeq> {
        match abi_record.decode_ops.ops.first() {
            Some(ReadOp::Record { fields, .. }) => fields
                .iter()
                .find(|field| field.name == *field_name)
                .map(|field| field.seq.clone()),
            _ => None,
        }
    }

    fn record_field_write_seq(
        &self,
        abi_record: &AbiRecord,
        field_name: &FieldName,
    ) -> Option<WriteSeq> {
        match abi_record.encode_ops.ops.first() {
            Some(WriteOp::Record { fields, .. }) => fields
                .iter()
                .find(|field| field.name == *field_name)
                .map(|field| field.seq.clone()),
            _ => None,
        }
    }

    fn lower_record_field(&self, field: &FieldDef, abi_record: &AbiRecord) -> DartRecordField {
        let record_field_write_seq = self
            .record_field_write_seq(abi_record, &field.name)
            .unwrap();
        let record_field_read_seq = self.record_field_read_seq(abi_record, &field.name).unwrap();

        DartRecordField {
            name: field.name.to_string(),
            offset: 0,
            dart_type: super::emit::type_expr_dart_type(&field.type_expr),
            wire_decode_expr: super::emit::emit_reader_read(&record_field_read_seq),
            wire_encode_expr: super::emit::emit_write_expr(&record_field_write_seq, "writer"),
        }
    }

    fn lower_record_blittable_field(&self, field: &FieldReadOp) -> DartBlittableField {
        let (primitive, offset) = match field.seq.ops.first() {
            Some(ReadOp::Primitive { primitive, offset }) => (*primitive, offset),
            _ => unreachable!(),
        };
        let offset = match offset {
            OffsetExpr::Base => 0,
            OffsetExpr::BasePlus(offset) => *offset,
            _ => unreachable!(),
        };
        let name = NamingConvention::property_name(field.name.as_str());
        let offset_const_name =
            NamingConvention::property_name(format!("offset_{}", field.name.as_str()).as_str());

        DartBlittableField {
            name,
            offset,
            native_type: DartNativeType::Primitive(primitive),
            primitive,
            offset_const_name,
        }
    }

    fn lower_record_blittable_layout(&self, abi_record: &AbiRecord) -> DartBlittableLayout {
        let fields = match abi_record.decode_ops.ops.first() {
            Some(ReadOp::Record { fields, .. }) => fields
                .iter()
                .map(|f| self.lower_record_blittable_field(f))
                .collect(),
            _ => unreachable!(),
        };

        DartBlittableLayout {
            fields,
            struct_size: abi_record
                .size
                .expect("record.is_blittable <=> size != None"),
        }
    }

    fn lower_record(&self, record: &RecordDef) -> DartRecord {
        let name = NamingConvention::class_name(record.id.as_str());

        let abi_record = self.abi_record_for(&record.id).unwrap();

        let fields = record
            .fields
            .iter()
            .map(|f| self.lower_record_field(f, abi_record))
            .collect();

        let blittable_layout = abi_record
            .is_blittable
            .then(|| self.lower_record_blittable_layout(abi_record));

        DartRecord {
            name,
            fields,
            blittable_layout,
        }
    }

    pub fn library(&self) -> DartLibrary {
        let records = self
            .ffi
            .catalog
            .all_records()
            .map(|r| self.lower_record(r))
            .collect();

        let native_functions = self
            .ffi
            .functions
            .iter()
            .map(|f| {
                let abi_call = self.abi_call_for_function(&f.id);
                self.lower_native_function(abi_call)
            })
            .collect();

        DartLibrary {
            native: DartNative {
                functions: native_functions,
            },
            records,
        }
    }
}

#[cfg(test)]
mod test {
    use boltffi_ffi_rules::callable::ExecutionKind;

    use crate::{
        ir::{
            self, CallbackId, CallbackKind, CallbackTraitDef, FunctionDef, PackageInfo, ParamDef,
            ParamName, ParamPassing, PrimitiveType, ReturnDef, TypeExpr,
        },
        render::dart::DartEmitter,
    };

    use super::*;

    fn empty_contract() -> FfiContract {
        FfiContract {
            package: PackageInfo {
                name: "test".to_string(),
                version: None,
            },
            functions: vec![],
            catalog: Default::default(),
        }
    }

    fn lower(ffi: &FfiContract) -> DartLibrary {
        let abi = ir::Lowerer::new(ffi).to_abi_contract();

        DartLowerer::new(ffi, &abi, "test").library()
    }

    #[test]
    pub fn native_function_primitive_in() {
        let mut ffi = empty_contract();
        ffi.functions.insert(
            0,
            FunctionDef {
                id: FunctionId::new("echo_u64"),
                params: vec![ParamDef {
                    name: ParamName::new("v"),
                    type_expr: TypeExpr::Primitive(PrimitiveType::U64),
                    passing: ParamPassing::Value,
                    doc: None,
                }],
                returns: ReturnDef::Void,
                execution_kind: ExecutionKind::Sync,
                doc: None,
                deprecated: None,
            },
        );

        let library = lower(&ffi);

        assert!(matches!(
            library.native.functions[0].params[0].native_type,
            DartNativeType::Primitive(PrimitiveType::U64)
        ));

        assert_eq!(
            library.native.functions[0].params[0]
                .native_type
                .dart_sub_type(),
            "int".to_string()
        );
    }

    #[test]
    pub fn native_function_primitive_out() {
        let mut ffi = empty_contract();
        ffi.functions.insert(
            0,
            FunctionDef {
                id: FunctionId::new("echo_f32"),
                params: vec![],
                returns: ReturnDef::Value(TypeExpr::Primitive(PrimitiveType::F32)),
                execution_kind: ExecutionKind::Sync,
                doc: None,
                deprecated: None,
            },
        );
        let library = lower(&ffi);

        assert!(matches!(
            library.native.functions[0].return_type,
            DartNativeType::Primitive(PrimitiveType::F32)
        ));
        assert_eq!(
            library.native.functions[0].return_type.dart_sub_type(),
            "double".to_string()
        );
    }

    #[test]
    pub fn native_function_void_out() {
        let mut ffi = empty_contract();
        ffi.functions.insert(
            0,
            FunctionDef {
                id: FunctionId::new("noop"),
                params: vec![],
                returns: ReturnDef::Void,
                execution_kind: ExecutionKind::Sync,
                doc: None,
                deprecated: None,
            },
        );
        let library = lower(&ffi);

        assert!(matches!(
            library.native.functions[0].return_type,
            DartNativeType::Void,
        ));
        assert_eq!(
            library.native.functions[0].return_type.dart_sub_type(),
            "void".to_string()
        );
    }

    #[test]
    pub fn native_function_closure_in() {
        let mut ffi = empty_contract();
        ffi.catalog.insert_callback(CallbackTraitDef {
            id: CallbackId::new("ClosureCb"),
            methods: vec![],
            kind: CallbackKind::Closure,
            doc: None,
        });
        ffi.functions.insert(
            0,
            FunctionDef {
                id: FunctionId::new("function_with_callback"),
                params: vec![ParamDef {
                    name: ParamName::new("cb"),
                    type_expr: TypeExpr::Callback(CallbackId::new("ClosureCb")),
                    passing: ParamPassing::ImplTrait,
                    doc: None,
                }],
                returns: ReturnDef::Void,
                execution_kind: ExecutionKind::Sync,
                doc: None,
                deprecated: None,
            },
        );
        let library = lower(&ffi);

        assert!(
            library.native.functions[0].params[0]
                .native_type
                .native_type()
                .contains("$$ffi.Pointer<$$ffi.NativeFunction<")
        );
        assert!(!library.native.functions[0].is_leaf);
    }

    #[test]
    pub fn blittable_record_produces_dart_ffi_struct() {
        let mut ffi = empty_contract();
        ffi.catalog.insert_record(RecordDef {
            id: RecordId::new("Point"),
            is_repr_c: true,
            is_error: false,
            fields: vec![
                FieldDef {
                    name: FieldName::new("x"),
                    type_expr: TypeExpr::Primitive(PrimitiveType::F64),
                    doc: None,
                    default: None,
                },
                FieldDef {
                    name: FieldName::new("y"),
                    type_expr: TypeExpr::Primitive(PrimitiveType::F64),
                    doc: None,
                    default: None,
                },
            ],
            constructors: vec![],
            methods: vec![],
            doc: None,
            deprecated: None,
        });

        let library = lower(&ffi);

        let output = DartEmitter::emit(&library);

        assert!(library.records[0].blittable_layout.is_some());
        assert!(output.contains("final class ___Point extends $$ffi.Struct"));
    }

    #[test]
    pub fn non_blittable_record_does_not_produce_dart_ffi_struct() {
        let mut ffi = empty_contract();
        ffi.catalog.insert_record(RecordDef {
            id: RecordId::new("Person"),
            is_repr_c: true,
            is_error: false,
            fields: vec![
                FieldDef {
                    name: FieldName::new("age"),
                    type_expr: TypeExpr::Primitive(PrimitiveType::U64),
                    doc: None,
                    default: None,
                },
                FieldDef {
                    name: FieldName::new("name"),
                    type_expr: TypeExpr::String,
                    doc: None,
                    default: None,
                },
            ],
            constructors: vec![],
            methods: vec![],
            doc: None,
            deprecated: None,
        });

        let library = lower(&ffi);

        assert!(library.records[0].blittable_layout.is_none());
    }
}
