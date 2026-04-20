use askama::Template;

use super::plan::*;

pub fn ts_doc_block(doc: &Option<String>, indent: &str) -> String {
    match doc {
        Some(text) => {
            let mut result = format!("{indent}/**\n");
            text.lines().for_each(|line| {
                if line.is_empty() {
                    result.push_str(&format!("{indent} *\n"));
                } else {
                    result.push_str(&format!("{indent} * {line}\n"));
                }
            });
            result.push_str(&format!("{indent} */\n"));
            result
        }
        None => String::new(),
    }
}

#[derive(Template)]
#[template(path = "render_typescript/preamble_header.txt", escape = "none")]
pub struct PreambleHeaderTemplate {
    pub abi_version: u32,
    pub wasm_bindgen_glue: Option<String>,
}

#[derive(Template)]
#[template(path = "render_typescript/preamble_tail.txt", escape = "none")]
pub struct PreambleTailTemplate {
    pub wasm_bindgen_glue: Option<String>,
}

#[derive(Template)]
#[template(path = "render_typescript/preamble_node.txt", escape = "none")]
pub struct NodePreambleTemplate {
    pub abi_version: u32,
    pub module_name: String,
}

#[derive(Template)]
#[template(path = "render_typescript/footer_node.txt", escape = "none")]
pub struct NodeFooterTemplate;

#[derive(Template)]
#[template(path = "render_typescript/record.txt", escape = "none")]
pub struct RecordTemplate<'a> {
    pub name: &'a str,
    pub fields: &'a [TsField],
    pub is_blittable: bool,
    pub wire_size: Option<usize>,
    pub tail_padding: usize,
    pub size_expr: String,
    pub doc: &'a Option<String>,
}

impl<'a> RecordTemplate<'a> {
    pub fn from_record(record: &'a TsRecord) -> Self {
        let size_expr = if let Some(size) = record.wire_size {
            size.to_string()
        } else {
            record
                .fields
                .iter()
                .map(|f| f.wire_size_expr("v"))
                .collect::<Vec<_>>()
                .join(" + ")
        };
        Self {
            name: &record.name,
            fields: &record.fields,
            is_blittable: record.is_blittable,
            wire_size: record.wire_size,
            tail_padding: record.tail_padding,
            size_expr,
            doc: &record.doc,
        }
    }
}

#[derive(Template)]
#[template(path = "render_typescript/value_type_companion.txt", escape = "none")]
pub struct ValueTypeCompanionTemplate<'a> {
    pub name: &'a str,
    pub constructors: &'a [TsValueTypeConstructor],
    pub methods: &'a [TsValueTypeMethod],
}

#[derive(Template)]
#[template(path = "render_typescript/enum_c_style.txt", escape = "none")]
pub struct EnumCStyleTemplate<'a> {
    pub name: &'a str,
    pub variants: &'a [TsVariant],
    pub doc: &'a Option<String>,
}

#[derive(Template)]
#[template(path = "render_typescript/enum_namespace.txt", escape = "none")]
pub struct EnumNamespaceTemplate<'a> {
    pub name: &'a str,
    pub constructors: &'a [TsValueTypeConstructor],
    pub methods: &'a [TsValueTypeMethod],
}

#[derive(Template)]
#[template(path = "render_typescript/enum_data.txt", escape = "none")]
pub struct EnumDataTemplate<'a> {
    pub name: &'a str,
    pub variants: &'a [TsVariant],
    pub doc: &'a Option<String>,
}

#[derive(Template)]
#[template(path = "render_typescript/error_exception.txt", escape = "none")]
pub struct ErrorExceptionTemplate<'a> {
    pub type_name: &'a str,
    pub class_name: &'a str,
    pub is_c_style_enum: bool,
}

#[derive(Template)]
#[template(path = "render_typescript/function.txt", escape = "none")]
pub struct FunctionTemplate<'a> {
    pub name: &'a str,
    pub params: &'a [TsParam],
    pub return_type_str: &'a str,
    pub return_route: &'a TsOutputRoute,
    pub return_callback: &'a Option<TsCallbackHandleReturn>,
    pub ffi_name: &'a str,
    pub call_args: &'a str,
    pub call_args_with_out: &'a str,
    pub wrapper_code: &'a str,
    pub cleanup_code: &'a str,
    pub doc: &'a Option<String>,
}

#[derive(Template)]
#[template(path = "render_typescript/class.txt", escape = "none")]
pub struct ClassTemplate<'a> {
    pub cls: &'a TsClass,
}

#[derive(Template)]
#[template(path = "render_typescript/callback.txt", escape = "none")]
pub struct CallbackTemplate<'a> {
    pub callback: &'a TsCallback,
}

#[derive(Template)]
#[template(path = "render_typescript/async_function.txt", escape = "none")]
pub struct AsyncFunctionTemplate<'a> {
    pub name: &'a str,
    pub params: &'a [TsParam],
    pub return_type_str: &'a str,
    pub entry_ffi_name: &'a str,
    pub poll_sync_ffi_name: &'a str,
    pub complete_ffi_name: &'a str,
    pub panic_message_ffi_name: &'a str,
    pub free_ffi_name: &'a str,
    pub call_args: &'a str,
    pub wrapper_code: &'a str,
    pub cleanup_code: &'a str,
    pub return_route: &'a TsOutputRoute,
    pub return_handle: &'a Option<TsHandleReturn>,
    pub return_callback: &'a Option<TsCallbackHandleReturn>,
    pub doc: &'a Option<String>,
}

#[derive(Template)]
#[template(path = "render_typescript/wasm_exports.txt", escape = "none")]
pub struct WasmExportsTemplate<'a> {
    pub wasm_imports: &'a [TsWasmImportView<'a>],
}

pub struct TsWasmImportView<'a> {
    pub ffi_name: &'a str,
    pub params: &'a [TsWasmParam],
    pub return_wasm_type_str: &'a str,
}

pub struct TypeScriptEmitter;

impl TypeScriptEmitter {
    pub fn emit(module: &TsModule) -> String {
        let mut output = String::new();

        output.push_str(
            &PreambleHeaderTemplate {
                abi_version: module.abi_version,
                wasm_bindgen_glue: module.wasm_bindgen_glue.clone(),
            }
            .render()
            .unwrap(),
        );
        output.push('\n');

        let wasm_import_views: Vec<TsWasmImportView> = module
            .wasm_imports
            .iter()
            .map(|import| TsWasmImportView {
                ffi_name: &import.ffi_name,
                params: &import.params,
                return_wasm_type_str: import.return_wasm_type.as_deref().unwrap_or("void"),
            })
            .collect();

        output.push_str(
            &WasmExportsTemplate {
                wasm_imports: &wasm_import_views,
            }
            .render()
            .unwrap(),
        );
        output.push_str("\n\n");
        output.push_str(
            &PreambleTailTemplate {
                wasm_bindgen_glue: module.wasm_bindgen_glue.clone(),
            }
            .render()
            .unwrap(),
        );
        output.push('\n');

        for record in &module.records {
            output.push_str(&RecordTemplate::from_record(record).render().unwrap());
            if record.has_companion() {
                output.push_str("\n\n");
                output.push_str(
                    &ValueTypeCompanionTemplate {
                        name: &record.name,
                        constructors: &record.constructors,
                        methods: &record.methods,
                    }
                    .render()
                    .unwrap(),
                );
            }
            output.push_str("\n\n");
        }

        for enumeration in &module.enums {
            if enumeration.is_c_style() {
                output.push_str(
                    &EnumCStyleTemplate {
                        name: &enumeration.name,
                        variants: &enumeration.variants,
                        doc: &enumeration.doc,
                    }
                    .render()
                    .unwrap(),
                );
                if enumeration.has_companion() {
                    output.push_str("\n\n");
                    output.push_str(
                        &EnumNamespaceTemplate {
                            name: &enumeration.name,
                            constructors: &enumeration.constructors,
                            methods: &enumeration.methods,
                        }
                        .render()
                        .unwrap(),
                    );
                }
            } else {
                output.push_str(
                    &EnumDataTemplate {
                        name: &enumeration.name,
                        variants: &enumeration.variants,
                        doc: &enumeration.doc,
                    }
                    .render()
                    .unwrap(),
                );
                if enumeration.has_companion() {
                    output.push_str("\n\n");
                    output.push_str(
                        &ValueTypeCompanionTemplate {
                            name: &enumeration.name,
                            constructors: &enumeration.constructors,
                            methods: &enumeration.methods,
                        }
                        .render()
                        .unwrap(),
                    );
                }
            }
            output.push_str("\n\n");
        }

        for error_exception in &module.error_exceptions {
            output.push_str(
                &ErrorExceptionTemplate {
                    type_name: &error_exception.type_name,
                    class_name: &error_exception.class_name,
                    is_c_style_enum: error_exception.is_c_style_enum,
                }
                .render()
                .unwrap(),
            );
            output.push_str("\n\n");
        }

        for function in &module.functions {
            let call_args = function
                .params
                .iter()
                .flat_map(|p| p.ffi_args())
                .collect::<Vec<_>>()
                .join(", ");
            let call_args_with_out = if call_args.is_empty() {
                "outPtr".to_string()
            } else {
                format!("outPtr, {call_args}")
            };

            let wrapper_code = function
                .params
                .iter()
                .filter_map(|p| p.wrapper_code())
                .collect::<Vec<_>>()
                .join("\n  ");

            let cleanup_code = function
                .params
                .iter()
                .filter_map(|p| p.cleanup_code())
                .collect::<Vec<_>>()
                .join("\n  ");

            let return_type_str = function.return_type.as_deref().unwrap_or("void");

            output.push_str(
                &FunctionTemplate {
                    name: &function.name,
                    params: &function.params,
                    return_type_str,
                    return_route: &function.return_route,
                    return_callback: &function.return_callback,
                    ffi_name: &function.ffi_name,
                    call_args: &call_args,
                    call_args_with_out: &call_args_with_out,
                    wrapper_code: &wrapper_code,
                    cleanup_code: &cleanup_code,
                    doc: &function.doc,
                }
                .render()
                .unwrap(),
            );
            output.push_str("\n\n");
        }

        for async_function in &module.async_functions {
            let call_args = async_function
                .params
                .iter()
                .flat_map(|p| p.ffi_args())
                .collect::<Vec<_>>()
                .join(", ");

            let wrapper_code = async_function
                .params
                .iter()
                .filter_map(|p| p.wrapper_code())
                .collect::<Vec<_>>()
                .join("\n    ");

            let cleanup_code = async_function
                .params
                .iter()
                .filter_map(|p| p.cleanup_code())
                .collect::<Vec<_>>()
                .join("\n    ");

            let return_type_str = async_function.return_type.as_deref().unwrap_or("void");

            output.push_str(
                &AsyncFunctionTemplate {
                    name: &async_function.name,
                    params: &async_function.params,
                    return_type_str,
                    entry_ffi_name: &async_function.entry_ffi_name,
                    poll_sync_ffi_name: &async_function.poll_sync_ffi_name,
                    complete_ffi_name: &async_function.complete_ffi_name,
                    panic_message_ffi_name: &async_function.panic_message_ffi_name,
                    free_ffi_name: &async_function.free_ffi_name,
                    call_args: &call_args,
                    wrapper_code: &wrapper_code,
                    cleanup_code: &cleanup_code,
                    return_route: &async_function.return_route,
                    return_handle: &async_function.return_handle,
                    return_callback: &async_function.return_callback,
                    doc: &async_function.doc,
                }
                .render()
                .unwrap(),
            );
            output.push_str("\n\n");
        }

        for class in &module.classes {
            output.push_str(&ClassTemplate { cls: class }.render().unwrap());
            output.push_str("\n\n");
        }

        for callback in &module.callbacks {
            output.push_str(&CallbackTemplate { callback }.render().unwrap());
            output.push_str("\n\n");
        }

        output
    }

    pub fn emit_node(module: &TsModule, module_name: &str) -> String {
        let mut output = String::new();

        output.push_str(
            &NodePreambleTemplate {
                abi_version: module.abi_version,
                module_name: module_name.to_string(),
            }
            .render()
            .unwrap(),
        );
        output.push('\n');

        let wasm_import_views: Vec<TsWasmImportView> = module
            .wasm_imports
            .iter()
            .map(|import| TsWasmImportView {
                ffi_name: &import.ffi_name,
                params: &import.params,
                return_wasm_type_str: import.return_wasm_type.as_deref().unwrap_or("void"),
            })
            .collect();

        output.push_str(
            &WasmExportsTemplate {
                wasm_imports: &wasm_import_views,
            }
            .render()
            .unwrap(),
        );
        output.push_str("\n\n");

        for record in &module.records {
            output.push_str(&RecordTemplate::from_record(record).render().unwrap());
            if record.has_companion() {
                output.push_str("\n\n");
                output.push_str(
                    &ValueTypeCompanionTemplate {
                        name: &record.name,
                        constructors: &record.constructors,
                        methods: &record.methods,
                    }
                    .render()
                    .unwrap(),
                );
            }
            output.push_str("\n\n");
        }

        for enumeration in &module.enums {
            if enumeration.is_c_style() {
                output.push_str(
                    &EnumCStyleTemplate {
                        name: &enumeration.name,
                        variants: &enumeration.variants,
                        doc: &enumeration.doc,
                    }
                    .render()
                    .unwrap(),
                );
                if enumeration.has_companion() {
                    output.push_str("\n\n");
                    output.push_str(
                        &EnumNamespaceTemplate {
                            name: &enumeration.name,
                            constructors: &enumeration.constructors,
                            methods: &enumeration.methods,
                        }
                        .render()
                        .unwrap(),
                    );
                }
            } else {
                output.push_str(
                    &EnumDataTemplate {
                        name: &enumeration.name,
                        variants: &enumeration.variants,
                        doc: &enumeration.doc,
                    }
                    .render()
                    .unwrap(),
                );
                if enumeration.has_companion() {
                    output.push_str("\n\n");
                    output.push_str(
                        &ValueTypeCompanionTemplate {
                            name: &enumeration.name,
                            constructors: &enumeration.constructors,
                            methods: &enumeration.methods,
                        }
                        .render()
                        .unwrap(),
                    );
                }
            }
            output.push_str("\n\n");
        }

        for error_exception in &module.error_exceptions {
            output.push_str(
                &ErrorExceptionTemplate {
                    type_name: &error_exception.type_name,
                    class_name: &error_exception.class_name,
                    is_c_style_enum: error_exception.is_c_style_enum,
                }
                .render()
                .unwrap(),
            );
            output.push_str("\n\n");
        }

        for callback in &module.callbacks {
            output.push_str(&CallbackTemplate { callback }.render().unwrap());
            output.push_str("\n\n");
        }

        output.push_str(&NodeFooterTemplate.render().unwrap());
        output.push_str("\n\n");

        for function in &module.functions {
            let call_args = function
                .params
                .iter()
                .flat_map(|p| p.ffi_args())
                .collect::<Vec<_>>()
                .join(", ");
            let call_args_with_out = if call_args.is_empty() {
                "outPtr".to_string()
            } else {
                format!("outPtr, {call_args}")
            };

            let wrapper_code = function
                .params
                .iter()
                .filter_map(|p| p.wrapper_code())
                .collect::<Vec<_>>()
                .join("\n  ");

            let cleanup_code = function
                .params
                .iter()
                .filter_map(|p| p.cleanup_code())
                .collect::<Vec<_>>()
                .join("\n  ");

            let return_type_str = function.return_type.as_deref().unwrap_or("void");

            output.push_str(
                &FunctionTemplate {
                    name: &function.name,
                    params: &function.params,
                    return_type_str,
                    return_route: &function.return_route,
                    return_callback: &function.return_callback,
                    ffi_name: &function.ffi_name,
                    call_args: &call_args,
                    call_args_with_out: &call_args_with_out,
                    wrapper_code: &wrapper_code,
                    cleanup_code: &cleanup_code,
                    doc: &function.doc,
                }
                .render()
                .unwrap(),
            );
            output.push_str("\n\n");
        }

        for async_function in &module.async_functions {
            let call_args = async_function
                .params
                .iter()
                .flat_map(|p| p.ffi_args())
                .collect::<Vec<_>>()
                .join(", ");

            let wrapper_code = async_function
                .params
                .iter()
                .filter_map(|p| p.wrapper_code())
                .collect::<Vec<_>>()
                .join("\n  ");

            let cleanup_code = async_function
                .params
                .iter()
                .filter_map(|p| p.cleanup_code())
                .collect::<Vec<_>>()
                .join("\n  ");

            let return_type_str = async_function.return_type.as_deref().unwrap_or("void");

            output.push_str(
                &AsyncFunctionTemplate {
                    name: &async_function.name,
                    params: &async_function.params,
                    return_type_str,
                    entry_ffi_name: &async_function.entry_ffi_name,
                    poll_sync_ffi_name: &async_function.poll_sync_ffi_name,
                    complete_ffi_name: &async_function.complete_ffi_name,
                    panic_message_ffi_name: &async_function.panic_message_ffi_name,
                    free_ffi_name: &async_function.free_ffi_name,
                    call_args: &call_args,
                    wrapper_code: &wrapper_code,
                    cleanup_code: &cleanup_code,
                    return_route: &async_function.return_route,
                    return_handle: &async_function.return_handle,
                    return_callback: &async_function.return_callback,
                    doc: &async_function.doc,
                }
                .render()
                .unwrap(),
            );
            output.push_str("\n\n");
        }

        for class in &module.classes {
            output.push_str(&ClassTemplate { cls: class }.render().unwrap());
            output.push_str("\n\n");
        }

        output
    }
}

#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;
    use crate::ir::ids::FieldName;
    use crate::ir::ops::{
        OffsetExpr, ReadOp, ReadSeq, SizeExpr, ValueExpr, WireShape, WriteOp, WriteSeq,
    };
    use crate::ir::types::PrimitiveType;

    fn primitive_size(p: PrimitiveType) -> usize {
        match p {
            PrimitiveType::Bool | PrimitiveType::I8 | PrimitiveType::U8 => 1,
            PrimitiveType::I16 | PrimitiveType::U16 => 2,
            PrimitiveType::I32 | PrimitiveType::U32 | PrimitiveType::F32 => 4,
            PrimitiveType::I64
            | PrimitiveType::U64
            | PrimitiveType::F64
            | PrimitiveType::ISize
            | PrimitiveType::USize => 8,
        }
    }

    fn primitive_read(primitive: PrimitiveType) -> ReadSeq {
        ReadSeq {
            size: SizeExpr::Fixed(primitive_size(primitive)),
            ops: vec![ReadOp::Primitive {
                primitive,
                offset: OffsetExpr::Base,
            }],
            shape: WireShape::Value,
        }
    }

    fn test_proxy_abi_i32_return_shape() -> crate::ir::abi::ReturnShape {
        use crate::ir::plan::{ScalarOrigin, Transport};
        use boltffi_ffi_rules::transport::{
            ReturnContract, ScalarReturnStrategy, ValueReturnStrategy,
        };
        crate::ir::abi::ReturnShape {
            contract: ReturnContract::infallible(ValueReturnStrategy::Scalar(
                ScalarReturnStrategy::PrimitiveValue,
            )),
            transport: Some(Transport::Scalar(ScalarOrigin::Primitive(PrimitiveType::I32))),
            decode_ops: None,
            encode_ops: None,
        }
    }

    fn primitive_write(primitive: PrimitiveType, field: &str) -> WriteSeq {
        WriteSeq {
            size: SizeExpr::Fixed(primitive_size(primitive)),
            ops: vec![WriteOp::Primitive {
                primitive,
                value: ValueExpr::Field(
                    Box::new(ValueExpr::Var("value".to_string())),
                    FieldName::new(field),
                ),
            }],
            shape: WireShape::Value,
        }
    }

    fn string_read() -> ReadSeq {
        ReadSeq {
            size: SizeExpr::Runtime,
            ops: vec![ReadOp::String {
                offset: OffsetExpr::Base,
            }],
            shape: WireShape::Value,
        }
    }

    fn string_write(field: &str) -> WriteSeq {
        WriteSeq {
            size: SizeExpr::StringLen(ValueExpr::Field(
                Box::new(ValueExpr::Var("value".to_string())),
                FieldName::new(field),
            )),
            ops: vec![WriteOp::String {
                value: ValueExpr::Field(
                    Box::new(ValueExpr::Var("value".to_string())),
                    FieldName::new(field),
                ),
            }],
            shape: WireShape::Value,
        }
    }

    #[test]
    fn snapshot_preamble() {
        let mut output = PreambleHeaderTemplate {
            abi_version: 1,
            wasm_bindgen_glue: None,
        }
        .render()
        .unwrap();
        output.push_str("\n");
        output.push_str(
            &PreambleTailTemplate {
                wasm_bindgen_glue: None,
            }
            .render()
            .unwrap(),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_preamble_wasm_bindgen_glue() {
        let glue = "demo_wbg.js".to_string();
        let mut output = PreambleHeaderTemplate {
            abi_version: 1,
            wasm_bindgen_glue: Some(glue.clone()),
        }
        .render()
        .unwrap();
        output.push_str("\n");
        output.push_str(
            &PreambleTailTemplate {
                wasm_bindgen_glue: Some(glue),
            }
            .render()
            .unwrap(),
        );
        insta::assert_snapshot!(output);
    }

    #[test]
    fn snapshot_record_with_primitive_fields() {
        let record = TsRecord {
            name: "Point".to_string(),
            fields: vec![
                TsField {
                    name: "x".to_string(),
                    ts_type: "number".to_string(),
                    decode: primitive_read(PrimitiveType::F64),
                    encode: primitive_write(PrimitiveType::F64, "x"),
                    doc: None,
                },
                TsField {
                    name: "y".to_string(),
                    ts_type: "number".to_string(),
                    decode: primitive_read(PrimitiveType::F64),
                    encode: primitive_write(PrimitiveType::F64, "y"),
                    doc: None,
                },
            ],
            constructors: vec![],
            methods: vec![],
            is_blittable: true,
            wire_size: Some(16),
            tail_padding: 0,
            doc: None,
        };

        let template = RecordTemplate::from_record(&record);
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_record_with_string_field() {
        let record = TsRecord {
            name: "User".to_string(),
            fields: vec![
                TsField {
                    name: "id".to_string(),
                    ts_type: "number".to_string(),
                    decode: primitive_read(PrimitiveType::I32),
                    encode: primitive_write(PrimitiveType::I32, "id"),
                    doc: None,
                },
                TsField {
                    name: "name".to_string(),
                    ts_type: "string".to_string(),
                    decode: string_read(),
                    encode: string_write("name"),
                    doc: Some("The user's display name".to_string()),
                },
            ],
            constructors: vec![],
            methods: vec![],
            is_blittable: false,
            wire_size: None,
            tail_padding: 0,
            doc: Some("A user record".to_string()),
        };

        let template = RecordTemplate::from_record(&record);
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_enum_c_style() {
        let doc = Some("A color enum".to_string());
        let variants = vec![
            TsVariant {
                name: "Red".to_string(),
                discriminant: 0,
                fields: vec![],
                doc: None,
            },
            TsVariant {
                name: "Green".to_string(),
                discriminant: 1,
                fields: vec![],
                doc: None,
            },
            TsVariant {
                name: "Blue".to_string(),
                discriminant: 2,
                fields: vec![],
                doc: Some("The blue channel".to_string()),
            },
        ];
        let template = EnumCStyleTemplate {
            name: "Color",
            variants: &variants,
            doc: &doc,
        };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_enum_c_style_u8_tag() {
        let doc = Some("Byte-sized enum".to_string());
        let variants = vec![
            TsVariant {
                name: "None".to_string(),
                discriminant: 0,
                fields: vec![],
                doc: None,
            },
            TsVariant {
                name: "Some".to_string(),
                discriminant: 255,
                fields: vec![],
                doc: None,
            },
        ];
        let template = EnumCStyleTemplate {
            name: "ByteState",
            variants: &variants,
            doc: &doc,
        };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_enum_data() {
        let doc: Option<String> = None;
        let variants = vec![
            TsVariant {
                name: "Circle".to_string(),
                discriminant: 0,
                fields: vec![TsVariantField {
                    name: "radius".to_string(),
                    ts_type: "number".to_string(),
                    decode: primitive_read(PrimitiveType::F64),
                    encode: primitive_write(PrimitiveType::F64, "radius"),
                }],
                doc: None,
            },
            TsVariant {
                name: "Rectangle".to_string(),
                discriminant: 1,
                fields: vec![
                    TsVariantField {
                        name: "width".to_string(),
                        ts_type: "number".to_string(),
                        decode: primitive_read(PrimitiveType::F64),
                        encode: primitive_write(PrimitiveType::F64, "width"),
                    },
                    TsVariantField {
                        name: "height".to_string(),
                        ts_type: "number".to_string(),
                        decode: primitive_read(PrimitiveType::F64),
                        encode: primitive_write(PrimitiveType::F64, "height"),
                    },
                ],
                doc: None,
            },
            TsVariant {
                name: "Nothing".to_string(),
                discriminant: 2,
                fields: vec![],
                doc: Some("An empty shape".to_string()),
            },
        ];
        let template = EnumDataTemplate {
            name: "Shape",
            variants: &variants,
            doc: &doc,
        };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_function_void() {
        let doc: Option<String> = None;
        let template = FunctionTemplate {
            name: "reset",
            params: &[],
            return_type_str: "void",
            return_route: &TsOutputRoute::void(),
            return_callback: &None,
            ffi_name: "boltffi_reset",
            call_args: "",
            call_args_with_out: "outPtr",
            wrapper_code: "",
            cleanup_code: "",
            doc: &doc,
        };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_function_direct_return() {
        let doc = Some("Adds two numbers".to_string());
        let params = vec![
            TsParam {
                name: "a".to_string(),
                ts_type: "number".to_string(),
                input_route: TsInputRoute::Direct,
            },
            TsParam {
                name: "b".to_string(),
                ts_type: "number".to_string(),
                input_route: TsInputRoute::Direct,
            },
        ];
        let template = FunctionTemplate {
            name: "add",
            params: &params,
            return_type_str: "number",
            return_route: &TsOutputRoute::direct(String::new()),
            return_callback: &None,
            ffi_name: "boltffi_add",
            call_args: "a, b",
            call_args_with_out: "outPtr, a, b",
            wrapper_code: "",
            cleanup_code: "",
            doc: &doc,
        };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_function_wire_encoded_return() {
        let doc: Option<String> = None;
        let template = FunctionTemplate {
            name: "getUsers",
            params: &[],
            return_type_str: "User[]",
            return_route: &TsOutputRoute::packed(
                "reader.readArray(() => decodeUser(reader))".to_string(),
            ),
            return_callback: &None,
            ffi_name: "boltffi_get_users",
            call_args: "",
            call_args_with_out: "",
            wrapper_code: "",
            cleanup_code: "",
            doc: &doc,
        };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn async_function_param_cleanup_runs_after_await() {
        let doc: Option<String> = None;
        let params = vec![TsParam {
            name: "message".to_string(),
            ts_type: "Message".to_string(),
            input_route: TsInputRoute::CodecEncoded {
                codec_name: "MessageCodec".to_string(),
            },
        }];
        let rendered = AsyncFunctionTemplate {
            name: "sendMessage",
            params: &params,
            return_type_str: "Response",
            entry_ffi_name: "boltffi_send_message",
            poll_sync_ffi_name: "boltffi_send_message_poll_sync",
            complete_ffi_name: "boltffi_send_message_complete",
            panic_message_ffi_name: "boltffi_send_message_panic_message",
            free_ffi_name: "boltffi_send_message_free",
            call_args: "message_writer.ptr, message_writer.len",
            wrapper_code: "const message_writer = _module.allocWriter(MessageCodec.size(message));\n  MessageCodec.encode(message_writer, message);",
            cleanup_code: "_module.freeWriter(message_writer);",
            return_route: &TsOutputRoute::packed("ResponseCodec.decode(reader)".to_string()),
            return_handle: &None,
            return_callback: &None,
            doc: &doc,
        }
        .render()
        .unwrap();

        let cleanup_index = rendered
            .find("_module.freeWriter(message_writer);")
            .unwrap();
        let await_index = rendered
            .find("const awaitedHandle = await _module.asyncManager.pollAsync(")
            .unwrap();
        assert!(cleanup_index > await_index);
    }

    fn sync_callback_fixture() -> TsCallback {
        TsCallback {
            interface_name: "ValueHandler".to_string(),
            trait_name_snake: "value_handler".to_string(),
            create_handle_fn: "boltffi_create_value_handler_handle".to_string(),
            local_free_fn: "__boltffi_local_value_handler_free".to_string(),
            wrap_handle_fn: "wrapValueHandler".to_string(),
            proxy_class_name: "ValueHandlerProxy".to_string(),
            is_returned: true,
            methods: vec![TsCallbackMethod {
                ts_name: "onValue".to_string(),
                import_name: "__boltffi_callback_value_handler_on_value".to_string(),
                proxy_export_name: "__boltffi_local_value_handler_on_value".to_string(),
                params: vec![TsCallbackParam {
                    name: "value".to_string(),
                    ts_type: "number".to_string(),
                    kind: TsCallbackParamKind::Primitive {
                        import_ts_type: "number".to_string(),
                        call_expr: "value".to_string(),
                    },
                }],
                proxy_params: vec![TsParam {
                    name: "value".to_string(),
                    ts_type: "number".to_string(),
                    input_route: TsInputRoute::Direct,
                }],
                return_type: Some("number".to_string()),
                import_return: TsCallbackImportReturn::Direct {
                    wasm_type: "number".to_string(),
                    outbound_wrap: None,
                },
                proxy_return_route: TsOutputRoute::direct(String::new()),
                proxy_return_handle: None,
                proxy_return_callback: None,
                proxy_abi_returns: test_proxy_abi_i32_return_shape(),
                doc: None,
            }],
            async_methods: vec![],
            closure_fn_type: None,
            doc: None,
        }
    }

    fn async_callback_fixture() -> TsCallback {
        TsCallback {
            interface_name: "AsyncFetcher".to_string(),
            trait_name_snake: "async_fetcher".to_string(),
            create_handle_fn: "boltffi_create_async_fetcher_handle".to_string(),
            local_free_fn: "__boltffi_local_async_fetcher_free".to_string(),
            wrap_handle_fn: "wrapAsyncFetcher".to_string(),
            proxy_class_name: "AsyncFetcherProxy".to_string(),
            is_returned: true,
            methods: vec![],
            async_methods: vec![TsAsyncCallbackMethod {
                ts_name: "fetch".to_string(),
                start_import_name: "__boltffi_callback_async_fetcher_fetch_start".to_string(),
                complete_export_name: "boltffi_callback_async_fetcher_fetch_complete".to_string(),
                proxy_export_name: "__boltffi_local_async_fetcher_fetch".to_string(),
                proxy_params: vec![],
                poll_sync_ffi_name: "__boltffi_local_async_fetcher_fetch_poll_sync".to_string(),
                complete_ffi_name: "__boltffi_local_async_fetcher_fetch_complete".to_string(),
                panic_message_ffi_name: "__boltffi_local_async_fetcher_fetch_panic_message".to_string(),
                cancel_ffi_name: "__boltffi_local_async_fetcher_fetch_cancel".to_string(),
                free_ffi_name: "__boltffi_local_async_fetcher_fetch_free".to_string(),
                proxy_return_route: TsOutputRoute::packed("reader.readI32()".to_string()),
                return_handle: None,
                return_callback: None,
                params: vec![TsCallbackParam {
                    name: "key".to_string(),
                    ts_type: "number".to_string(),
                    kind: TsCallbackParamKind::Primitive {
                        import_ts_type: "number".to_string(),
                        call_expr: "key".to_string(),
                    },
                }],
                return_type: Some("number".to_string()),
                encode_expr: None,
                size_expr: None,
                direct_write_method: Some("writeI32".to_string()),
                direct_write_value_expr: Some("result".to_string()),
                direct_size: Some(4),
                proxy_wasm_imports: vec![],
                doc: None,
            }],
            closure_fn_type: None,
            doc: None,
        }
    }

    #[test]
    fn callback_registry_emits_refcounted_lifecycle_contract() {
        let callback = sync_callback_fixture();
        let rendered = CallbackTemplate {
            callback: &callback,
        }
        .render()
        .unwrap();

        assert!(rendered.contains("const _value_handler_ref_counts = new Map<number, number>();"));
        assert!(
            rendered.contains("let _value_handler_next_id = _callback_handle_js_namespace_start;")
        );
        assert!(rendered.contains("const handle_key = _callback_handle_key(handle);"));
        assert!(rendered.contains("_value_handler_ref_counts.set(id, 1);"));
        assert!(rendered.contains("return _value_handler_retain(handle);"));
        assert!(rendered.contains("_value_handler_release(handle);"));
        assert!(rendered.contains("const impl = _value_handler_lookup(handle);"));
    }

    #[test]
    fn callback_registry_emits_invalid_handle_and_no_resurrection_guards() {
        let callback = sync_callback_fixture();
        let rendered = CallbackTemplate {
            callback: &callback,
        }
        .render()
        .unwrap();

        assert!(rendered.contains(
            "Cannot clone unknown callback handle ${handle_key} in ValueHandler registry"
        ));
        assert!(rendered.contains(
            "Cannot free unknown callback handle ${handle_key} in ValueHandler registry"
        ));
        assert!(
            rendered.contains("Callback handle ${handle_key} not found in ValueHandler registry")
        );
        assert!(rendered.contains("if (currentCount === 1) {"));
        assert!(rendered.contains("_value_handler_ref_counts.delete(handle_key);"));
        assert!(rendered.contains("_value_handler_registry.delete(handle_key);"));
        assert!(rendered.contains("return handle_key;"));
    }

    #[test]
    fn async_callback_invalid_handle_is_reported_through_completion() {
        let callback = async_callback_fixture();
        let rendered = CallbackTemplate {
            callback: &callback,
        }
        .render()
        .unwrap();

        assert!(rendered.contains("let impl: AsyncFetcher;"));
        assert!(rendered.contains("impl = _async_fetcher_lookup(handle);"));
        assert!(rendered.contains("completeError(err);"));
        assert!(rendered.contains("return;"));
    }

    #[test]
    fn snapshot_class_with_constructor_and_methods() {
        let class = TsClass {
            class_name: "Counter".to_string(),
            ffi_free: "boltffi_counter_free".to_string(),
            constructors: vec![TsClassConstructor {
                ts_name: "new".to_string(),
                ffi_name: "boltffi_counter_new".to_string(),
                is_default: true,
                params: vec![],
                returns_nullable_handle: false,
                return_type: None,
                return_handle: None,
                return_callback: None,
                mode: TsClassConstructorMode::Sync(TsClassSyncConstructor {}),
                doc: Some("Creates a counter".to_string()),
            }],
            methods: vec![
                TsClassMethod {
                    ts_name: "increment".to_string(),
                    ffi_name: "boltffi_counter_increment".to_string(),
                    is_static: false,
                    params: vec![TsParam {
                        name: "delta".to_string(),
                        ts_type: "number".to_string(),
                        input_route: TsInputRoute::Direct,
                    }],
                    return_type: Some("number".to_string()),
                    return_handle: None,
                    return_callback: None,
                    mode: TsClassMethodMode::Sync(TsClassSyncMethod {
                        return_route: TsOutputRoute::direct(String::new()),
                    }),
                    doc: None,
                },
                TsClassMethod {
                    ts_name: "nextValue".to_string(),
                    ffi_name: "boltffi_counter_next_value".to_string(),
                    is_static: false,
                    params: vec![],
                    return_type: Some("number".to_string()),
                    return_handle: None,
                    return_callback: None,
                    mode: TsClassMethodMode::Async(TsClassAsyncMethod {
                        poll_sync_ffi_name: "boltffi_counter_next_value_poll_sync".to_string(),
                        complete_ffi_name: "boltffi_counter_next_value_complete".to_string(),
                        panic_message_ffi_name: "boltffi_counter_next_value_panic_message"
                            .to_string(),
                        cancel_ffi_name: "boltffi_counter_next_value_cancel".to_string(),
                        free_ffi_name: "boltffi_counter_next_value_free".to_string(),
                        return_route: TsOutputRoute::packed("reader.readI32()".to_string()),
                    }),
                    doc: None,
                },
            ],
            doc: Some("A counter class".to_string()),
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn class_nullable_constructor_preserves_null_contract() {
        let class = TsClass {
            class_name: "Session".to_string(),
            ffi_free: "boltffi_session_free".to_string(),
            constructors: vec![TsClassConstructor {
                ts_name: "open".to_string(),
                ffi_name: "boltffi_session_open".to_string(),
                is_default: false,
                params: vec![TsParam {
                    name: "path".to_string(),
                    ts_type: "string".to_string(),
                    input_route: TsInputRoute::String,
                }],
                returns_nullable_handle: true,
                return_type: None,
                return_handle: None,
                return_callback: None,
                mode: TsClassConstructorMode::Sync(TsClassSyncConstructor {}),
                doc: None,
            }],
            methods: vec![],
            doc: None,
        };

        let rendered = ClassTemplate { cls: &class }.render().unwrap();
        assert!(rendered.contains("static open(path: string): Session | null {"));
        assert!(rendered.contains("if (handle === 0) {\n        return null;\n      }"));
    }

    #[test]
    fn class_async_return_frees_handles_on_decode_failures() {
        let class = TsClass {
            class_name: "Counter".to_string(),
            ffi_free: "boltffi_counter_free".to_string(),
            constructors: vec![],
            methods: vec![TsClassMethod {
                ts_name: "nextValue".to_string(),
                ffi_name: "boltffi_counter_next_value".to_string(),
                is_static: false,
                params: vec![],
                return_type: Some("number".to_string()),
                return_handle: None,
                return_callback: None,
                mode: TsClassMethodMode::Async(TsClassAsyncMethod {
                    poll_sync_ffi_name: "boltffi_counter_next_value_poll_sync".to_string(),
                    complete_ffi_name: "boltffi_counter_next_value_complete".to_string(),
                    panic_message_ffi_name: "boltffi_counter_next_value_panic_message".to_string(),
                    cancel_ffi_name: "boltffi_counter_next_value_cancel".to_string(),
                    free_ffi_name: "boltffi_counter_next_value_free".to_string(),
                    return_route: TsOutputRoute::packed("reader.readI32()".to_string()),
                }),
                doc: None,
            }],
            doc: None,
        };

        let rendered = ClassTemplate { cls: &class }.render().unwrap();
        assert!(rendered.contains("let completeCompleted = false;"));
        assert!(rendered.contains("_module.freeBuf(outPtr);"));
        assert!(rendered.contains("_module.freeBufDescriptor(outPtr);"));
        assert!(rendered.contains(
            "_exports.boltffi_counter_next_value_free(awaitedHandle);"
        ));
    }

    #[test]
    fn class_async_param_cleanup_runs_after_await() {
        let class = TsClass {
            class_name: "Database".to_string(),
            ffi_free: "boltffi_database_free".to_string(),
            constructors: vec![],
            methods: vec![TsClassMethod {
                ts_name: "query".to_string(),
                ffi_name: "boltffi_database_query".to_string(),
                is_static: false,
                params: vec![TsParam {
                    name: "sql".to_string(),
                    ts_type: "string".to_string(),
                    input_route: TsInputRoute::String,
                }],
                return_type: Some("QueryResult".to_string()),
                return_handle: None,
                return_callback: None,
                mode: TsClassMethodMode::Async(TsClassAsyncMethod {
                    poll_sync_ffi_name: "boltffi_database_query_poll_sync".to_string(),
                    complete_ffi_name: "boltffi_database_query_complete".to_string(),
                    panic_message_ffi_name: "boltffi_database_query_panic_message".to_string(),
                    cancel_ffi_name: "boltffi_database_query_cancel".to_string(),
                    free_ffi_name: "boltffi_database_query_free".to_string(),
                    return_route: TsOutputRoute::packed(
                        "QueryResultCodec.decode(reader)".to_string(),
                    ),
                }),
                doc: None,
            }],
            doc: None,
        };

        let rendered = ClassTemplate { cls: &class }.render().unwrap();
        let cleanup_index = rendered.find("_module.freeAlloc(sql_alloc);").unwrap();
        let await_index = rendered
            .find("const awaitedHandle = await _module.asyncManager.pollAsync(")
            .unwrap();
        assert!(cleanup_index > await_index);
    }

    #[test]
    fn snapshot_wasm_exports() {
        let params = vec![
            TsWasmParam {
                name: "a".to_string(),
                wasm_type: "number".to_string(),
            },
            TsWasmParam {
                name: "b".to_string(),
                wasm_type: "number".to_string(),
            },
        ];
        let imports = vec![TsWasmImportView {
            ffi_name: "boltffi_add",
            params: &params,
            return_wasm_type_str: "number",
        }];
        let template = WasmExportsTemplate {
            wasm_imports: &imports,
        };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn wasm_exports_renders_encoded_return_with_out_param() {
        let params = vec![
            TsWasmParam {
                name: "out".to_string(),
                wasm_type: "number".to_string(),
            },
            TsWasmParam {
                name: "payload".to_string(),
                wasm_type: "number".to_string(),
            },
        ];
        let imports = vec![TsWasmImportView {
            ffi_name: "boltffi_echo_payload",
            params: &params,
            return_wasm_type_str: "void",
        }];
        let template = WasmExportsTemplate {
            wasm_imports: &imports,
        };
        let rendered = template.render().unwrap();
        assert!(rendered.contains("boltffi_echo_payload(out: number, payload: number): void;"));
    }

    #[test]
    fn snapshot_class_with_pass_handle_constructor() {
        let class = TsClass {
            class_name: "Receiver".to_string(),
            ffi_free: "boltffi_receiver_free".to_string(),
            constructors: vec![TsClassConstructor {
                ts_name: "new".to_string(),
                ffi_name: "boltffi_receiver_new".to_string(),
                is_default: true,
                params: vec![TsParam {
                    name: "endpoint".to_string(),
                    ts_type: "Endpoint".to_string(),
                    input_route: TsInputRoute::Handle { nullable: false },
                }],
                returns_nullable_handle: false,
                return_type: None,
                return_handle: None,
                return_callback: None,
                mode: TsClassConstructorMode::Sync(TsClassSyncConstructor {}),
                doc: None,
            }],
            methods: vec![],
            doc: None,
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_class_with_static_method() {
        let class = TsClass {
            class_name: "MathUtils".to_string(),
            ffi_free: "boltffi_math_utils_free".to_string(),
            constructors: vec![],
            methods: vec![TsClassMethod {
                ts_name: "add".to_string(),
                ffi_name: "boltffi_math_utils_add".to_string(),
                is_static: true,
                params: vec![
                    TsParam {
                        name: "a".to_string(),
                        ts_type: "number".to_string(),
                        input_route: TsInputRoute::Direct,
                    },
                    TsParam {
                        name: "b".to_string(),
                        ts_type: "number".to_string(),
                        input_route: TsInputRoute::Direct,
                    },
                ],
                return_type: Some("number".to_string()),
                return_handle: None,
                return_callback: None,
                mode: TsClassMethodMode::Sync(TsClassSyncMethod {
                    return_route: TsOutputRoute::direct(String::new()),
                }),
                doc: None,
            }],
            doc: None,
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_class_with_void_method() {
        let class = TsClass {
            class_name: "Logger".to_string(),
            ffi_free: "boltffi_logger_free".to_string(),
            constructors: vec![TsClassConstructor {
                ts_name: "new".to_string(),
                ffi_name: "boltffi_logger_new".to_string(),
                is_default: true,
                params: vec![],
                returns_nullable_handle: false,
                return_type: None,
                return_handle: None,
                return_callback: None,
                mode: TsClassConstructorMode::Sync(TsClassSyncConstructor {}),
                doc: None,
            }],
            methods: vec![TsClassMethod {
                ts_name: "log".to_string(),
                ffi_name: "boltffi_logger_log".to_string(),
                is_static: false,
                params: vec![TsParam {
                    name: "message".to_string(),
                    ts_type: "string".to_string(),
                    input_route: TsInputRoute::String,
                }],
                return_type: None,
                return_handle: None,
                return_callback: None,
                mode: TsClassMethodMode::Sync(TsClassSyncMethod {
                    return_route: TsOutputRoute::void(),
                }),
                doc: None,
            }],
            doc: None,
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_class_with_handle_return() {
        let class = TsClass {
            class_name: "Factory".to_string(),
            ffi_free: "boltffi_factory_free".to_string(),
            constructors: vec![],
            methods: vec![TsClassMethod {
                ts_name: "createChild".to_string(),
                ffi_name: "boltffi_factory_create_child".to_string(),
                is_static: false,
                params: vec![],
                return_type: Some("Child".to_string()),
                return_handle: Some(TsHandleReturn {
                    class_name: "Child".to_string(),
                    nullable: false,
                }),
                return_callback: None,
                mode: TsClassMethodMode::Sync(TsClassSyncMethod {
                    return_route: TsOutputRoute::direct(String::new()),
                }),
                doc: None,
            }],
            doc: None,
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_class_with_nullable_handle_return() {
        let class = TsClass {
            class_name: "Cache".to_string(),
            ffi_free: "boltffi_cache_free".to_string(),
            constructors: vec![],
            methods: vec![TsClassMethod {
                ts_name: "get".to_string(),
                ffi_name: "boltffi_cache_get".to_string(),
                is_static: false,
                params: vec![TsParam {
                    name: "key".to_string(),
                    ts_type: "string".to_string(),
                    input_route: TsInputRoute::String,
                }],
                return_type: Some("Entry | null".to_string()),
                return_handle: Some(TsHandleReturn {
                    class_name: "Entry".to_string(),
                    nullable: true,
                }),
                return_callback: None,
                mode: TsClassMethodMode::Sync(TsClassSyncMethod {
                    return_route: TsOutputRoute::direct(String::new()),
                }),
                doc: None,
            }],
            doc: None,
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_class_with_encoded_param() {
        let class = TsClass {
            class_name: "Renderer".to_string(),
            ffi_free: "boltffi_renderer_free".to_string(),
            constructors: vec![],
            methods: vec![TsClassMethod {
                ts_name: "draw".to_string(),
                ffi_name: "boltffi_renderer_draw".to_string(),
                is_static: false,
                params: vec![TsParam {
                    name: "point".to_string(),
                    ts_type: "Point".to_string(),
                    input_route: TsInputRoute::CodecEncoded {
                        codec_name: "Point".to_string(),
                    },
                }],
                return_type: None,
                return_handle: None,
                return_callback: None,
                mode: TsClassMethodMode::Sync(TsClassSyncMethod {
                    return_route: TsOutputRoute::void(),
                }),
                doc: None,
            }],
            doc: None,
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_class_async_with_encoded_return() {
        let class = TsClass {
            class_name: "Database".to_string(),
            ffi_free: "boltffi_database_free".to_string(),
            constructors: vec![],
            methods: vec![TsClassMethod {
                ts_name: "query".to_string(),
                ffi_name: "boltffi_database_query".to_string(),
                is_static: false,
                params: vec![TsParam {
                    name: "sql".to_string(),
                    ts_type: "string".to_string(),
                    input_route: TsInputRoute::String,
                }],
                return_type: Some("QueryResult".to_string()),
                return_handle: None,
                return_callback: None,
                mode: TsClassMethodMode::Async(TsClassAsyncMethod {
                    poll_sync_ffi_name: "boltffi_database_query_poll_sync".to_string(),
                    complete_ffi_name: "boltffi_database_query_complete".to_string(),
                    panic_message_ffi_name: "boltffi_database_query_panic_message".to_string(),
                    cancel_ffi_name: "boltffi_database_query_cancel".to_string(),
                    free_ffi_name: "boltffi_database_query_free".to_string(),
                    return_route: TsOutputRoute::packed(
                        "QueryResultCodec.decode(reader)".to_string(),
                    ),
                }),
                doc: None,
            }],
            doc: None,
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_class_with_async_fallible_constructor() {
        let class = TsClass {
            class_name: "Endpoint".to_string(),
            ffi_free: "boltffi_endpoint_free".to_string(),
            constructors: vec![TsClassConstructor {
                ts_name: "new".to_string(),
                ffi_name: "boltffi_endpoint_new".to_string(),
                is_default: true,
                params: vec![TsParam {
                    name: "cfg".to_string(),
                    ts_type: "string".to_string(),
                    input_route: TsInputRoute::String,
                }],
                returns_nullable_handle: false,
                return_type: Some("Endpoint".to_string()),
                return_handle: Some(TsHandleReturn {
                    class_name: "Endpoint".to_string(),
                    nullable: false,
                }),
                return_callback: None,
                mode: TsClassConstructorMode::Async(TsClassAsyncMethod {
                    poll_sync_ffi_name: "boltffi_endpoint_new_poll_sync".to_string(),
                    complete_ffi_name: "boltffi_endpoint_new_complete".to_string(),
                    panic_message_ffi_name: "boltffi_endpoint_new_panic_message".to_string(),
                    cancel_ffi_name: "boltffi_endpoint_new_cancel".to_string(),
                    free_ffi_name: "boltffi_endpoint_new_free".to_string(),
                    return_route: TsOutputRoute::async_fallible_handle_carrier(
                        "Endpoint".to_string(),
                        "new TransferErrorException(TransferErrorCodec.decode(reader))".to_string(),
                        false,
                    ),
                }),
                doc: None,
            }],
            methods: vec![],
            doc: None,
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_class_sync_method_with_result_and_handle_ok() {
        let class = TsClass {
            class_name: "Receiver".to_string(),
            ffi_free: "boltffi_receiver_free".to_string(),
            constructors: vec![],
            methods: vec![TsClassMethod {
                ts_name: "receive".to_string(),
                ffi_name: "boltffi_receiver_receive".to_string(),
                is_static: false,
                params: vec![],
                return_type: Some("Transfer".to_string()),
                return_handle: Some(TsHandleReturn {
                    class_name: "Transfer".to_string(),
                    nullable: false,
                }),
                return_callback: None,
                mode: TsClassMethodMode::Sync(TsClassSyncMethod {
                    return_route: TsOutputRoute::sync_direct_ok_carrier_ok(
                        "Transfer".to_string(),
                        "new TransferErrorException(TransferErrorCodec.decode(reader))".to_string(),
                        String::new(),
                        false,
                    ),
                }),
                doc: None,
            }],
            doc: None,
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_callback_with_handle_param() {
        let callback = TsCallback {
            interface_name: "SenderCallbacks".to_string(),
            trait_name_snake: "sender_callbacks".to_string(),
            create_handle_fn: "boltffi_create_sender_callbacks_handle".to_string(),
            local_free_fn: "__boltffi_local_sender_callbacks_free".to_string(),
            wrap_handle_fn: "wrapSenderCallbacks".to_string(),
            proxy_class_name: "SenderCallbacksProxy".to_string(),
            is_returned: true,
            methods: vec![TsCallbackMethod {
                ts_name: "onTransferStarted".to_string(),
                import_name: "__boltffi_callback_sender_callbacks_on_transfer_started".to_string(),
                proxy_export_name: "__boltffi_local_sender_callbacks_on_transfer_started".to_string(),
                params: vec![TsCallbackParam {
                    name: "transfer".to_string(),
                    ts_type: "Transfer".to_string(),
                    kind: TsCallbackParamKind::InboundHandle {
                        class_name: "Transfer".to_string(),
                    },
                }],
                proxy_params: vec![TsParam {
                    name: "transfer".to_string(),
                    ts_type: "Transfer".to_string(),
                    input_route: TsInputRoute::Handle { nullable: false },
                }],
                return_type: None,
                import_return: TsCallbackImportReturn::Void,
                proxy_return_route: TsOutputRoute::void(),
                proxy_return_handle: None,
                proxy_return_callback: None,
                proxy_abi_returns: crate::ir::abi::ReturnShape::void(),
                doc: None,
            }],
            async_methods: vec![],
            closure_fn_type: None,
            doc: None,
        };
        let template = CallbackTemplate { callback: &callback };
        insta::assert_snapshot!(template.render().unwrap());
    }

    /// when the trait is never returned from rust, omit proxy class and `wrap*` (mirrors swift).
    #[test]
    fn snapshot_callback_not_returned_omits_proxy() {
        let mut callback = sync_callback_fixture();
        callback.is_returned = false;
        let template = CallbackTemplate { callback: &callback };
        insta::assert_snapshot!(template.render().unwrap());
    }

    /// rust→js import returns a nested callback handle; user implementation is registered to a u32.
    #[test]
    fn snapshot_callback_inbound_register_callback_return() {
        let callback = TsCallback {
            interface_name: "ParentCb".to_string(),
            trait_name_snake: "parent_cb".to_string(),
            create_handle_fn: "boltffi_create_parent_cb_handle".to_string(),
            local_free_fn: "__boltffi_local_parent_cb_free".to_string(),
            wrap_handle_fn: "wrapParentCb".to_string(),
            proxy_class_name: "ParentCbProxy".to_string(),
            is_returned: true,
            methods: vec![TsCallbackMethod {
                ts_name: "getChild".to_string(),
                import_name: "__boltffi_callback_parent_cb_get_child".to_string(),
                proxy_export_name: "__boltffi_local_parent_cb_get_child".to_string(),
                params: vec![],
                proxy_params: vec![],
                return_type: Some("ChildCb".to_string()),
                import_return: TsCallbackImportReturn::Direct {
                    wasm_type: "number".to_string(),
                    outbound_wrap: Some(TsCallbackImportOutboundWrap::RegisterCallback {
                        register_fn: "registerChildCb".to_string(),
                        nullable: false,
                    }),
                },
                proxy_return_route: TsOutputRoute::direct(String::new()),
                proxy_return_handle: None,
                proxy_return_callback: Some(TsCallbackHandleReturn {
                    interface_name: "ChildCb".to_string(),
                    wrap_fn: "wrapChildCb".to_string(),
                    nullable: false,
                }),
                proxy_abi_returns: test_proxy_abi_i32_return_shape(),
                doc: None,
            }],
            async_methods: vec![],
            closure_fn_type: None,
            doc: None,
        };
        let template = CallbackTemplate { callback: &callback };
        insta::assert_snapshot!(template.render().unwrap());
    }

    fn test_proxy_abi_u64_return_shape() -> crate::ir::abi::ReturnShape {
        use crate::ir::plan::{ScalarOrigin, Transport};
        use boltffi_ffi_rules::transport::{
            ReturnContract, ScalarReturnStrategy, ValueReturnStrategy,
        };
        crate::ir::abi::ReturnShape {
            contract: ReturnContract::infallible(ValueReturnStrategy::Scalar(
                ScalarReturnStrategy::PrimitiveValue,
            )),
            transport: Some(Transport::Scalar(ScalarOrigin::Primitive(PrimitiveType::U64))),
            decode_ops: None,
            encode_ops: None,
        }
    }

    /// u64 return from `__boltffi_local_*` maps to `bigint` in `WasmExports` typing.
    #[test]
    fn snapshot_callback_proxy_u64_return_uses_bigint() {
        let callback = TsCallback {
            interface_name: "StreamSource".to_string(),
            trait_name_snake: "stream_source".to_string(),
            create_handle_fn: "boltffi_create_stream_source_handle".to_string(),
            local_free_fn: "__boltffi_local_stream_source_free".to_string(),
            wrap_handle_fn: "wrapStreamSource".to_string(),
            proxy_class_name: "StreamSourceProxy".to_string(),
            is_returned: true,
            methods: vec![TsCallbackMethod {
                ts_name: "count".to_string(),
                import_name: "__boltffi_callback_stream_source_count".to_string(),
                proxy_export_name: "__boltffi_local_stream_source_count".to_string(),
                params: vec![],
                proxy_params: vec![],
                return_type: Some("bigint".to_string()),
                import_return: TsCallbackImportReturn::Direct {
                    wasm_type: "bigint".to_string(),
                    outbound_wrap: None,
                },
                proxy_return_route: TsOutputRoute::direct(String::new()),
                proxy_return_handle: None,
                proxy_return_callback: None,
                proxy_abi_returns: test_proxy_abi_u64_return_shape(),
                doc: None,
            }],
            async_methods: vec![],
            closure_fn_type: None,
            doc: None,
        };
        let template = CallbackTemplate { callback: &callback };
        insta::assert_snapshot!(template.render().unwrap());
    }

    /// wasm `complete` returns a u32 handle directly (not out-buf); mirrors `ReceivedTransfer::stream`.
    #[test]
    fn snapshot_class_async_method_returns_handle() {
        let class = TsClass {
            class_name: "ReceivedTransfer".to_string(),
            ffi_free: "boltffi_received_transfer_free".to_string(),
            constructors: vec![],
            methods: vec![TsClassMethod {
                ts_name: "stream".to_string(),
                ffi_name: "boltffi_received_transfer_stream".to_string(),
                is_static: false,
                params: vec![TsParam {
                    name: "fileIndex".to_string(),
                    ts_type: "bigint".to_string(),
                    input_route: TsInputRoute::Direct,
                }],
                return_type: Some("DataStreamHandle".to_string()),
                return_handle: Some(TsHandleReturn {
                    class_name: "DataStreamHandle".to_string(),
                    nullable: false,
                }),
                return_callback: None,
                mode: TsClassMethodMode::Async(TsClassAsyncMethod {
                    poll_sync_ffi_name: "boltffi_received_transfer_stream_poll_sync".to_string(),
                    complete_ffi_name: "boltffi_received_transfer_stream_complete".to_string(),
                    panic_message_ffi_name: "boltffi_received_transfer_stream_panic_message".to_string(),
                    cancel_ffi_name: "boltffi_received_transfer_stream_cancel".to_string(),
                    free_ffi_name: "boltffi_received_transfer_stream_free".to_string(),
                    return_route: TsOutputRoute::async_scalar(String::new()),
                }),
                doc: None,
            }],
            doc: None,
        };
        let template = ClassTemplate { cls: &class };
        insta::assert_snapshot!(template.render().unwrap());
    }

    #[test]
    fn snapshot_async_function_returns_callback_handle() {
        let output = AsyncFunctionTemplate {
            name: "makeCallback",
            params: &[],
            return_type_str: "MyCallback",
            entry_ffi_name: "boltffi_make_callback",
            poll_sync_ffi_name: "boltffi_make_callback_poll_sync",
            complete_ffi_name: "boltffi_make_callback_complete",
            panic_message_ffi_name: "boltffi_make_callback_panic_message",
            free_ffi_name: "boltffi_make_callback_free",
            call_args: "",
            wrapper_code: "",
            cleanup_code: "",
            return_route: &TsOutputRoute::async_scalar(String::new()),
            return_handle: &None,
            return_callback: &Some(TsCallbackHandleReturn {
                interface_name: "MyCallback".to_string(),
                wrap_fn: "wrapMyCallback".to_string(),
                nullable: false,
            }),
            doc: &None,
        }
        .render()
        .unwrap();
        insta::assert_snapshot!(output);
    }
}
