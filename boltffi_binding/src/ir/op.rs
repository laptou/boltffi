use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::{CanonicalName, FieldKey, Primitive, TypeRef};

/// Marker for operations that yield a count of bytes.
///
/// Used as the type parameter on [`Op`] so byte counts cannot be added to
/// element counts or scalar values without a compile error.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ByteCount;

/// Marker for operations that yield a count of elements.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ElementCount;

/// Marker for operations that yield a boolean condition.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Truth;

/// Marker for operations that yield a scalar of type `T`.
///
/// `T` is one of the unit-marker types that name a scalar shape (for
/// instance `i32` or `f64`). Scalar arithmetic over the same `T` is
/// allowed; mixing two different `T`s is a compile error.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Scalar<T>(PhantomData<T>);

/// The kind of scalar produced by a typed operation.
///
/// Recorded inside the operation tree so a deserialized expression still
/// knows what arithmetic precision to use.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ScalarTy {
    /// Primitive scalar.
    Primitive(Primitive),
}

/// The identity of a value bound by a repeated operation.
///
/// A `for_each`-style operation introduces a fresh binder for the element
/// it iterates; nested loops can refer to outer binders by id. Stable
/// across serialization so a deserialized tree resolves its references
/// correctly.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BinderId(u32);

impl BinderId {
    /// Wraps a numeric binder id.
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the underlying numeric value.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Where a [`ValueRef`] starts.
///
/// `SelfValue` is the value currently being moved across the boundary;
/// `Named` and `Local` reference parameters and locals by canonical name;
/// `Binder` references the element bound by an enclosing repeated
/// operation.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ValueRoot {
    /// The value currently being encoded or decoded.
    SelfValue,
    /// A named callable parameter or local.
    Named(CanonicalName),
    /// A generated local.
    Local(CanonicalName),
    /// The element bound by an enclosing repeated operation.
    Binder(BinderId),
}

/// A path to a value available while a plan renders.
///
/// Begins at a [`ValueRoot`] and walks zero or more [`FieldKey`] accesses.
/// Storing the path as data keeps every renderer consistent about what
/// `self.name.first` or `payload.0` actually points at, even across
/// languages with different field-access syntax.
///
/// # Example
///
/// `ValueRef::self_value().field(FieldKey::Named(name))` references a
/// field of the current record while encoding it.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ValueRef {
    root: ValueRoot,
    path: Vec<FieldKey>,
}

impl ValueRef {
    /// References the value currently being moved across the boundary.
    pub fn self_value() -> Self {
        Self {
            root: ValueRoot::SelfValue,
            path: Vec::new(),
        }
    }

    /// References a named value.
    pub fn named(name: CanonicalName) -> Self {
        Self {
            root: ValueRoot::Named(name),
            path: Vec::new(),
        }
    }

    /// Appends a field or tuple-position access to the path.
    pub fn field(mut self, field: FieldKey) -> Self {
        self.path.push(field);
        self
    }

    /// Returns the root.
    pub fn root(&self) -> &ValueRoot {
        &self.root
    }

    /// Returns the field path from the root.
    pub fn path(&self) -> &[FieldKey] {
        &self.path
    }
}

/// A typed expression in the binding operation language.
///
/// The phantom marker tracks what kind of value the expression yields: a
/// byte count, an element count, a scalar of some primitive, a truth
/// value. Adding two byte counts is allowed; adding a byte count to an
/// element count is rejected at compile time.
///
/// # Example
///
/// The wire size of a length-prefixed UTF-8 string is built as
/// `Op::<ByteCount>::fixed(4).add_bytes(Op::<ByteCount>::utf8_bytes(value))`,
/// where `4` is the length prefix and `utf8_bytes` is the body length.
#[derive(Debug, Serialize, Deserialize)]
pub struct Op<T> {
    node: OpNode,
    #[serde(skip)]
    marker: PhantomData<T>,
}

impl<T> Clone for Op<T> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            marker: PhantomData,
        }
    }
}

impl<T> PartialEq for Op<T> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T> Eq for Op<T> {}

impl<T> std::hash::Hash for Op<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}

impl<T> Op<T> {
    fn new(node: OpNode) -> Self {
        Self {
            node,
            marker: PhantomData,
        }
    }

    /// Returns the underlying [`OpNode`].
    pub fn node(&self) -> &OpNode {
        &self.node
    }
}

impl Op<ByteCount> {
    /// Builds a fixed byte count.
    pub fn fixed(bytes: u64) -> Self {
        Self::new(OpNode::ByteCount(bytes))
    }

    /// Returns the sum of two byte counts.
    pub fn add_bytes(self, other: Op<ByteCount>) -> Self {
        Self::new(OpNode::Add(Box::new(self.node), Box::new(other.node)))
    }

    /// Computes the UTF-8 byte length of a value.
    pub fn utf8_bytes(value: ValueRef) -> Self {
        Self::new(OpNode::Intrinsic {
            intrinsic: IntrinsicOp::Utf8ByteCount,
            args: vec![OpNode::Value(value)],
        })
    }

    /// Computes the encoded byte size of a value.
    pub fn wire_size(value: ValueRef) -> Self {
        Self::new(OpNode::Intrinsic {
            intrinsic: IntrinsicOp::WireSize,
            args: vec![OpNode::Value(value)],
        })
    }
}

impl Op<ElementCount> {
    /// Computes the element count of a sequence.
    pub fn sequence_len(value: ValueRef) -> Self {
        Self::new(OpNode::Intrinsic {
            intrinsic: IntrinsicOp::SequenceLen,
            args: vec![OpNode::Value(value)],
        })
    }
}

impl<T> Op<Scalar<T>> {
    /// Reads a scalar value from a value reference.
    pub fn value(value: ValueRef) -> Self {
        Self::new(OpNode::Value(value))
    }
}

/// The serializable shape behind an [`Op`].
///
/// Public typed constructors live on `Op` so callers cannot build the
/// equivalent of "add an element count to a byte count" through `OpNode`
/// directly. The untyped node still has to be public for inspection and
/// serialization.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OpNode {
    /// Reference to a value in scope.
    Value(ValueRef),
    /// Fixed byte count.
    ByteCount(u64),
    /// Integer literal.
    Integer(i128),
    /// Sum within the same typed family.
    Add(Box<OpNode>, Box<OpNode>),
    /// Product within the same typed family.
    Mul(Box<OpNode>, Box<OpNode>),
    /// Equality between two scalar values.
    Eq(Box<OpNode>, Box<OpNode>),
    /// Field access on a base operation.
    Field {
        /// Base operation.
        base: Box<OpNode>,
        /// Field selected from the base.
        field: FieldKey,
    },
    /// Built-in operation whose spelling depends on the target language.
    Intrinsic {
        /// The intrinsic.
        intrinsic: IntrinsicOp,
        /// Arguments passed to it.
        args: Vec<OpNode>,
    },
    /// Type-size query.
    SizeOf(TypeRef),
}

/// A built-in operation whose spelling depends on the target language.
///
/// `Utf8ByteCount` asks for the UTF-8 byte length of a string;
/// `SequenceLen` asks for the element count of a sequence; `WireSize`
/// asks for the encoded byte count of a value. Targets render these with
/// whatever idiomatic call their language provides.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IntrinsicOp {
    /// UTF-8 byte length of a string.
    Utf8ByteCount,
    /// Element count of a sequence.
    SequenceLen,
    /// Encoded byte size of a value.
    WireSize,
}
