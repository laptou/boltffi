use serde::{Deserialize, Serialize};

use crate::{
    CanonicalName, CustomTypeId, DeprecationInfo, DocComment, Path, Source, SourceSpan, TypeExpr,
    UserAttr,
};

/// A user-declared custom type.
///
/// Custom types describe a Rust type that should be exposed through a different
/// representation type, together with the Rust functions that convert between
/// the two.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct CustomTypeDef {
    /// Stable custom type identity derived from the canonical Rust path.
    pub id: CustomTypeId,
    /// Canonical custom type name.
    pub name: CanonicalName,
    /// Remote Rust type being represented.
    pub remote: TypeExpr,
    /// Source representation type used at the FFI surface.
    pub repr: TypeExpr,
    /// Converter functions supplied by the source declaration.
    pub converters: CustomTypeConverters,
    /// User attributes preserved from the custom type declaration.
    pub user_attrs: Vec<UserAttr>,
    /// Documentation attached to the custom type.
    pub doc: Option<DocComment>,
    /// Deprecation metadata attached to the custom type.
    pub deprecated: Option<DeprecationInfo>,
    /// Visibility and source location for diagnostics.
    pub source: Source,
    /// Span available during macro expansion.
    #[serde(default, skip_serializing, skip_deserializing)]
    pub source_span: Option<SourceSpan>,
}

impl CustomTypeDef {
    /// Builds a custom type definition.
    ///
    /// The `id` parameter is the stable custom type ID. The `name` parameter is
    /// the canonical source name. The `remote`, `repr`, and `converters`
    /// parameters record the user-declared conversion surface.
    ///
    /// Returns a custom type with no user attributes or documentation.
    pub fn new(
        id: CustomTypeId,
        name: CanonicalName,
        remote: TypeExpr,
        repr: TypeExpr,
        converters: CustomTypeConverters,
    ) -> Self {
        Self {
            id,
            name,
            remote,
            repr,
            converters,
            user_attrs: Vec::new(),
            doc: None,
            deprecated: None,
            source: Source::exported(),
            source_span: None,
        }
    }
}

/// Converter functions attached to a custom type declaration.
///
/// The paths name the Rust functions used to cross between the remote type and
/// the representation type.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct CustomTypeConverters {
    /// Function that turns the remote Rust value into the representation type.
    pub into_ffi: Path,
    /// Function that attempts to rebuild the remote Rust value from the representation type.
    pub try_from_ffi: Path,
}

impl CustomTypeConverters {
    /// Builds a pair of custom type converter paths.
    ///
    /// The `into_ffi` parameter names the conversion into the representation.
    /// The `try_from_ffi` parameter names the fallible conversion back into the
    /// remote Rust type.
    ///
    /// Returns converter metadata for the custom type declaration.
    pub fn new(into_ffi: Path, try_from_ffi: Path) -> Self {
        Self {
            into_ffi,
            try_from_ffi,
        }
    }
}
