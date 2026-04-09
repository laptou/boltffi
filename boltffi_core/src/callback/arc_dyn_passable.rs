//! Bridge for `Arc<dyn exported_callback_trait>` as [`crate::passable::Passable`].
//!
//! `Arc` is not a fundamental type, so user crates cannot implement [`crate::passable::Passable`]
//! for `Arc<dyn LocalTrait>` (orphan rules). [`ArcDynCallbackPassable`] is implemented in the
//! macro expansion for each exported callback trait object; [`crate::passable::Passable`] for
//! `Arc<T>` is provided in `passable/value.rs` as a blanket over this trait.

use std::sync::Arc;

use super::CallbackHandle;

/// Implemented by generated code for each exported `dyn` callback trait so `Arc<dyn Trait>` can
/// use the callback-handle ABI without violating orphan rules.
pub unsafe trait ArcDynCallbackPassable {
    /// Rebuilds `Arc<dyn Trait>` from a handle returned across the boundary.
    ///
    /// # Safety
    ///
    /// Same as [`super::ArcFromCallbackHandle::arc_from_callback_handle`].
    unsafe fn unpack_from_handle(handle: CallbackHandle) -> Arc<Self>;

    /// Packs shared callback ownership into a handle for the opposite direction.
    fn pack_to_handle(self_arc: Arc<Self>) -> CallbackHandle;
}
