use serde::{Deserialize, Serialize};
use std::fmt;

/// One word inside a [`CanonicalName`].
///
/// Names in the binding contract are stored as ordered segments rather than
/// as a pre-cased string so each target language can apply its own casing
/// rule. A `NamePart` is one of those segments, normalized to the form the
/// classifier produced.
///
/// # Example
///
/// The Rust type `UserProfile` becomes two parts: `["user", "profile"]`. A
/// PascalCase target joins them as `UserProfile`; a snake_case target as
/// `user_profile`; a SCREAMING_SNAKE target as `USER_PROFILE`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NamePart(String);

impl NamePart {
    /// Stores one already-normalized segment.
    pub fn new(part: impl Into<String>) -> Self {
        Self(part.into())
    }

    /// Returns the segment.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NamePart {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<&str> for NamePart {
    fn from(part: &str) -> Self {
        Self::new(part)
    }
}

impl From<String> for NamePart {
    fn from(part: String) -> Self {
        Self::new(part)
    }
}

/// A name in the binding contract before any target language has spelled it.
///
/// Storing names as ordered segments is the only way to render the same
/// name as `Point` in Swift, `point` in Python, and `point_t` in C without
/// re-parsing the original Rust identifier in each target.
///
/// Empty names are accepted by the constructor and rejected during
/// validation, so a deserialized contract can still produce a precise
/// diagnostic for the offending declaration instead of failing to load.
///
/// # Example
///
/// `CanonicalName::single("status")` is a one-segment name. For the Rust
/// type `XmlParser`, the segments are `["xml", "parser"]`; the Swift
/// renderer produces `XmlParser`, the snake_case renderer produces
/// `xml_parser`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CanonicalName {
    parts: Vec<NamePart>,
}

impl CanonicalName {
    /// Builds a name from already-normalized parts.
    pub fn new(parts: Vec<NamePart>) -> Self {
        Self { parts }
    }

    /// Builds a single-segment name.
    pub fn single(part: impl Into<NamePart>) -> Self {
        Self {
            parts: vec![part.into()],
        }
    }

    /// Returns the segments in source order.
    pub fn parts(&self) -> &[NamePart] {
        &self.parts
    }

    /// Returns the segments joined by `::`.
    pub fn as_path_string(&self) -> String {
        self.parts
            .iter()
            .map(NamePart::as_str)
            .collect::<Vec<_>>()
            .join("::")
    }
}
