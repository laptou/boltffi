//! Per-surface lowering decisions.
//!
//! The IR's [`Surface`] trait defines what each target's
//! divergent shapes *are* — `BufferShape`, `HandleCarrier`,
//! `AsyncProtocol`, `CallbackProtocol`. It does not say which value to
//! pick at a given call site. The lowering pass needs that pick: a
//! string parameter on Wasm32 must cross as `Slice`, a record returned
//! by value on Wasm32 must cross as `Packed`, and so on. Those rules
//! used to live in `boltffi_ffi_rules`; with that crate retired, the
//! rules live here.
//!
//! [`SurfaceLower`] adds those decisions to a [`Surface`] without
//! touching the IR. It is sealed: only [`Native`] and [`Wasm32`] can
//! implement it. The public lowering function carries the
//! `S: SurfaceLower` bound so external callers cannot supply a surface
//! the lowering pass has not been taught about.
//!
//! [`Surface`]: crate::Surface
//! [`Native`]: crate::Native
//! [`Wasm32`]: crate::Wasm32

use crate::{Native, Surface, Wasm32, native, wasm32};

mod sealed {
    /// Seals [`super::SurfaceLower`].
    ///
    /// Only the two surfaces shipped with this crate implement it; an
    /// external crate cannot teach the lowering pass to handle a new
    /// surface without changes here.
    pub trait Sealed {}

    impl Sealed for crate::Native {}
    impl Sealed for crate::Wasm32 {}
}

/// A [`Surface`] paired with the lowering-pass decisions that pick its
/// concrete shape values.
///
/// Each method names a fixed call-site role and returns the shape the
/// pass must use there. The choices follow the boltffi convention
/// shared with the foreign-side bindings.
pub trait SurfaceLower: Surface + sealed::Sealed {
    /// Buffer shape used for an encoded parameter crossing.
    ///
    /// Encoded params (strings, vecs, encoded records, ...) cross as
    /// pointer-plus-count on every supported surface today.
    #[doc(hidden)]
    fn encoded_param_shape() -> Self::BufferShape;

    /// Buffer shape used for an encoded return crossing.
    ///
    /// Native returns occupy a single descriptor slot
    /// ([`native::BufferShape::Buffer`]). Wasm32 returns occupy one
    /// 64-bit slot folded into [`wasm32::BufferShape::Packed`].
    #[doc(hidden)]
    fn encoded_return_shape() -> Self::BufferShape;

    /// Handle carrier used for an inline closure crossing.
    ///
    /// On native, a closure crosses through the runtime's
    /// [`native::HandleCarrier::CallbackHandle`] struct so the inner
    /// vtable pointer travels with the handle. On wasm32, the closure
    /// crosses as a 32-bit handle.
    #[doc(hidden)]
    fn closure_handle_carrier() -> Self::HandleCarrier;

    /// Handle carrier used for a class instance crossing.
    ///
    /// Native classes cross as a 64-bit token
    /// ([`native::HandleCarrier::U64`]). Wasm32 classes cross as a
    /// 32-bit token ([`wasm32::HandleCarrier::U32`]).
    #[doc(hidden)]
    fn class_handle_carrier() -> Self::HandleCarrier;
}

impl SurfaceLower for Native {
    fn encoded_param_shape() -> Self::BufferShape {
        native::BufferShape::Slice
    }

    fn encoded_return_shape() -> Self::BufferShape {
        native::BufferShape::Buffer
    }

    fn closure_handle_carrier() -> Self::HandleCarrier {
        native::HandleCarrier::CallbackHandle
    }

    fn class_handle_carrier() -> Self::HandleCarrier {
        native::HandleCarrier::U64
    }
}

impl SurfaceLower for Wasm32 {
    fn encoded_param_shape() -> Self::BufferShape {
        wasm32::BufferShape::Slice
    }

    fn encoded_return_shape() -> Self::BufferShape {
        wasm32::BufferShape::Packed
    }

    fn closure_handle_carrier() -> Self::HandleCarrier {
        wasm32::HandleCarrier::U32
    }

    fn class_handle_carrier() -> Self::HandleCarrier {
        wasm32::HandleCarrier::U32
    }
}
