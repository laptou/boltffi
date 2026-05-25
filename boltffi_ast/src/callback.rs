use serde::{Deserialize, Serialize};

use crate::{
    CallbackId, CanonicalName, DeprecationInfo, DocComment, MethodDef, Source, SourceSpan, UserAttr,
};

/// A callback trait exported through BoltFFI.
///
/// This represents a Rust trait whose methods can be implemented outside Rust.
/// Inline closure parameters use [`TypeExpr::Closure`](crate::TypeExpr::Closure)
/// instead of pretending to be trait declarations.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct CallbackTraitDef {
    /// Stable callback trait identity derived from the canonical Rust path.
    pub id: CallbackId,
    /// Canonical trait name.
    pub name: CanonicalName,
    /// Methods that the callback implementer must provide.
    pub methods: Vec<MethodDef>,
    /// User attributes preserved from the callback declaration.
    pub user_attrs: Vec<UserAttr>,
    /// Documentation attached to the callback.
    pub doc: Option<DocComment>,
    /// Deprecation metadata attached to the callback.
    pub deprecated: Option<DeprecationInfo>,
    /// Visibility and source location for diagnostics.
    pub source: Source,
    /// Span available during macro expansion.
    #[serde(default, skip_serializing, skip_deserializing)]
    pub source_span: Option<SourceSpan>,
}

impl CallbackTraitDef {
    /// Builds an empty callback trait definition.
    ///
    /// The `id` parameter is the stable callback ID. The `name` parameter is the
    /// canonical callback trait name.
    ///
    /// Returns a callback definition with no methods or attributes.
    pub fn new(id: CallbackId, name: CanonicalName) -> Self {
        Self {
            id,
            name,
            methods: Vec::new(),
            user_attrs: Vec::new(),
            doc: None,
            deprecated: None,
            source: Source::exported(),
            source_span: None,
        }
    }
}
