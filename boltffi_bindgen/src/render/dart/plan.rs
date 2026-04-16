use crate::ir::{AbiCall, AbiParam, AbiType, ErrorTransport, ParamRole, PrimitiveType, Transport};

#[derive(Clone, Debug)]
pub enum DartNativeType {
    Void,
    Primitive(PrimitiveType),
    Function {
        params: Vec<DartNativeType>,
        return_ty: Box<DartNativeType>,
    },
    Pointer(Box<DartNativeType>),
    Custom(String),
}

impl DartNativeType {
    pub fn from_abi_type(abi_type: &AbiType) -> Self {
        match abi_type {
            AbiType::Void => DartNativeType::Void,
            AbiType::Bool => DartNativeType::Primitive(PrimitiveType::Bool),
            AbiType::I8 => DartNativeType::Primitive(PrimitiveType::I8),
            AbiType::U8 => DartNativeType::Primitive(PrimitiveType::U8),
            AbiType::I16 => DartNativeType::Primitive(PrimitiveType::I16),
            AbiType::U16 => DartNativeType::Primitive(PrimitiveType::U16),
            AbiType::I32 => DartNativeType::Primitive(PrimitiveType::I32),
            AbiType::U32 => DartNativeType::Primitive(PrimitiveType::U32),
            AbiType::I64 => DartNativeType::Primitive(PrimitiveType::I64),
            AbiType::U64 => DartNativeType::Primitive(PrimitiveType::U64),
            AbiType::ISize => DartNativeType::Primitive(PrimitiveType::ISize),
            AbiType::USize => DartNativeType::Primitive(PrimitiveType::USize),
            AbiType::F32 => DartNativeType::Primitive(PrimitiveType::F32),
            AbiType::F64 => DartNativeType::Primitive(PrimitiveType::F64),
            AbiType::Pointer(primitive) => {
                DartNativeType::Pointer(Box::new(DartNativeType::Primitive(*primitive)))
            }
            AbiType::OwnedBuffer => DartNativeType::Custom("FfiBuf_u8".to_string()),
            AbiType::InlineCallbackFn {
                params,
                return_type,
            } => DartNativeType::Function {
                params: params.iter().map(Self::from_abi_type).collect(),
                return_ty: Box::new(Self::from_abi_type(return_type)),
            },
            AbiType::Handle(_) => DartNativeType::Pointer(Box::new(DartNativeType::Void)),
            AbiType::CallbackHandle => DartNativeType::Custom("BoltFFICallbackHandle".to_string()),
            AbiType::Struct(record_id) => {
                DartNativeType::Custom(format!("___{}", record_id.as_str()))
            }
        }
    }
    pub fn native_type(&self) -> String {
        match self {
            DartNativeType::Void => "$$ffi.Void".to_string(),
            DartNativeType::Primitive(primitive) => {
                super::emit::primitive_native_type(*primitive).to_string()
            }
            DartNativeType::Function { params, return_ty } => format!(
                "$$ffi.Pointer<$$ffi.NativeFunction<{} Function({})>>",
                return_ty.native_type(),
                params.iter().fold(
                    DartNativeType::Pointer(Box::new(DartNativeType::Void)).native_type(),
                    |acc, ty| acc + ", " + ty.native_type().as_str()
                )
            ),
            DartNativeType::Pointer(inner) => format!("$$ffi.Pointer<{}>", inner.native_type()),
            DartNativeType::Custom(ty) => ty.clone(),
        }
    }

    pub fn dart_sub_type(&self) -> String {
        match self {
            DartNativeType::Void => "void".to_string(),
            DartNativeType::Primitive(primitive) => super::emit::primitive_dart_type(*primitive),
            o @ (DartNativeType::Function { .. }
            | DartNativeType::Pointer(..)
            | DartNativeType::Custom(..)) => o.native_type(),
            // DartNativeType::Pointer(inner) => format!("$$ffi.Pointer<{}>", inner.native_type()),
            // DartNativeType::Custom(ty) => ty.clone(),
        }
    }

    pub fn abi_call_return_type(abi_call: &AbiCall) -> Self {
        if let Some(Transport::Handle { class_id, .. }) = &abi_call.returns.transport {
            return Self::from_abi_type(&AbiType::Handle(class_id.clone()));
        }

        if matches!(abi_call.returns.transport, Some(Transport::Callback { .. })) {
            return Self::from_abi_type(&AbiType::CallbackHandle);
        }

        if matches!(abi_call.error, ErrorTransport::Encoded { .. }) {
            return Self::from_abi_type(&AbiType::OwnedBuffer);
        }

        match &abi_call.returns.transport {
            None => {
                if matches!(abi_call.error, ErrorTransport::StatusCode) {
                    Self::Custom("FfiStatus".to_string())
                } else {
                    Self::from_abi_type(&AbiType::Void)
                }
            }
            Some(Transport::Scalar(origin)) => Self::Primitive(origin.primitive()),
            Some(Transport::Composite(layout)) => {
                Self::from_abi_type(&AbiType::Struct(layout.record_id.clone()))
            }
            Some(Transport::Span(_)) => Self::from_abi_type(&AbiType::OwnedBuffer),
            Some(Transport::Handle { .. } | Transport::Callback { .. }) => unreachable!(),
        }
    }

    pub fn from_abi_param(abi_param: &AbiParam) -> Self {
        if let ParamRole::CallbackContext { .. } = &abi_param.role {
            return Self::Pointer(Box::new(Self::Void));
        }

        if let ParamRole::StatusOut = &abi_param.role {
            return Self::Pointer(Box::new(Self::Custom("FfiStatus".to_string())));
        }

        let native_type = Self::from_abi_type(&abi_param.abi_type);

        match &abi_param.role {
            ParamRole::OutDirect | ParamRole::OutLen { .. } => Self::Pointer(Box::new(native_type)),
            _ => native_type,
        }
    }

    pub fn field_annot(&self) -> String {
        match self {
            DartNativeType::Void
            | DartNativeType::Function { .. }
            | DartNativeType::Pointer(_)
            | DartNativeType::Custom(_) => String::new(),
            primitive @ DartNativeType::Primitive(_) => format!("@{}()", primitive.native_type()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DartNativeFunctionParam {
    pub name: String,
    pub native_type: DartNativeType,
}

#[derive(Debug, Clone)]
pub struct DartNativeFunction {
    pub symbol: String,
    pub params: Vec<DartNativeFunctionParam>,
    pub return_type: DartNativeType,
    pub is_leaf: bool,
}

#[derive(Debug, Clone)]
pub struct DartNative {
    pub functions: Vec<DartNativeFunction>,
}

#[derive(Debug, Clone)]
pub struct DartRecordField {
    pub name: String,
    pub offset: usize,
    pub dart_type: String,
    pub wire_decode_expr: String,
    pub wire_encode_expr: String,
}

#[derive(Debug, Clone)]
pub struct DartBlittableLayout {
    pub struct_size: usize,
    pub fields: Vec<DartBlittableField>,
}

#[derive(Debug, Clone)]
pub struct DartBlittableField {
    pub name: String,
    pub primitive: PrimitiveType,
    pub native_type: DartNativeType,
    pub offset_const_name: String,
    pub offset: usize,
}

impl DartBlittableField {
    pub fn decode_expr(&self, bytes_name: &str) -> String {
        super::emit_read_blittable_value(&self.offset_const_name, self.primitive, bytes_name)
    }

    pub fn encode_expr(&self, bytes_name: &str) -> String {
        super::emit_write_blittable_value(
            &self.offset_const_name,
            self.primitive,
            &self.name,
            bytes_name,
        )
    }
}

#[derive(Debug, Clone)]
pub struct DartRecord {
    pub name: String,
    pub fields: Vec<DartRecordField>,
    pub blittable_layout: Option<DartBlittableLayout>,
}
#[derive(Debug, Clone)]
pub struct DartLibrary {
    pub native: DartNative,
    pub records: Vec<DartRecord>,
}
