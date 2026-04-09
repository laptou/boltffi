//! Callback handle and ownership contracts used by generated foreign bindings.
//!
//! Rust callback traits are exported through generated foreign wrapper types.
//! Those wrappers eventually cross the boundary as a raw [`CallbackHandle`].
//!
//! This module defines the small set of contracts the generated code relies on:
//! a raw callback handle, the traits that recover owned Rust values from that
//! handle, and the foreign-type association used to move between the Rust-facing
//! callback type and its generated wrapper.

mod arc_dyn_passable;
mod foreign;
mod handle;
mod ownership;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use arc_dyn_passable::ArcDynCallbackPassable;
pub use foreign::CallbackForeignType;
pub use handle::CallbackHandle;
pub use ownership::{ArcFromCallbackHandle, BoxFromCallbackHandle};
#[cfg(target_arch = "wasm32")]
pub use wasm::{WasmCallbackOwner, boltffi_create_callback_handle};
