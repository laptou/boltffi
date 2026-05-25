use boltffi_ast::{
    DefaultValue as SourceDefaultValue, DeprecationInfo as SourceDeprecationInfo,
    DocComment as SourceDocComment,
};

use crate::{DeclMeta, DefaultValue, DeprecationInfo, DocComment, ElementMeta, IntegerValue};

use super::{LowerError, error::UnsupportedType};

pub(super) fn decl_meta(
    doc: Option<&SourceDocComment>,
    deprecated: Option<&SourceDeprecationInfo>,
) -> DeclMeta {
    DeclMeta::new(doc.map(DocComment::from), deprecated.map(Into::into))
}

pub(super) fn element_meta(
    doc: Option<&SourceDocComment>,
    deprecated: Option<&SourceDeprecationInfo>,
    default: Option<&SourceDefaultValue>,
) -> Result<ElementMeta, LowerError> {
    Ok(ElementMeta::new(
        doc.map(DocComment::from),
        deprecated.map(Into::into),
        default.map(DefaultValue::try_from).transpose()?,
    ))
}

impl From<&SourceDocComment> for DocComment {
    fn from(doc: &SourceDocComment) -> Self {
        Self::new(doc.as_str())
    }
}

impl From<&SourceDeprecationInfo> for DeprecationInfo {
    fn from(deprecated: &SourceDeprecationInfo) -> Self {
        Self::new(deprecated.note.clone(), deprecated.since.clone())
    }
}

impl TryFrom<&SourceDefaultValue> for DefaultValue {
    type Error = LowerError;

    fn try_from(default: &SourceDefaultValue) -> Result<Self, Self::Error> {
        match default {
            SourceDefaultValue::Bool(value) => Ok(DefaultValue::Bool(*value)),
            SourceDefaultValue::Integer(value) => {
                Ok(DefaultValue::Integer(IntegerValue::new(value.value)))
            }
            SourceDefaultValue::String(value) => Ok(DefaultValue::String(value.clone())),
            SourceDefaultValue::None => Ok(DefaultValue::Null),
            SourceDefaultValue::Float(_)
            | SourceDefaultValue::Bytes(_)
            | SourceDefaultValue::Path(_) => {
                Err(LowerError::unsupported_type(UnsupportedType::DefaultValue))
            }
        }
    }
}
