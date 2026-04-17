pub use boltffi_core::{
    ArcDynCallbackPassable, ArcFromCallbackHandle, BoxFromCallbackHandle, CallbackForeignType,
    CallbackHandle,
    CustomFfiConvertible, CustomTypeConversionError, Data, EventSubscription, FfiType,
    StreamProducer, UnexpectedFfiCallbackError, custom_ffi, custom_type, data, default, error,
    export, ffi_stream, name, skip,
};

#[doc(hidden)]
pub mod __private {
    pub use boltffi_core::return_buffer::{boltffi_alloc_return_buffer, boltffi_free_return_buffer};
    pub use boltffi_core::{
        ArcDynCallbackPassable, ArcFromCallbackHandle, BoxFromCallbackHandle, CallbackForeignType,
        CallbackHandle,
        EventSubscription, FfiBuf, FfiSpan, FfiStatus, Passable, RustFutureContinuationCallback,
        RustFutureHandle, StreamContinuationCallback, StreamPollResult, SubscriptionHandle,
        VecTransport, WaitResult, WirePassable, rustfuture, set_last_error, wire,
    };
    #[cfg(target_arch = "wasm32")]
    pub use boltffi_core::{
        AsyncCallbackCompletion, AsyncCallbackCompletionCode, AsyncCallbackCompletionResult,
        AsyncCallbackRegistry, AsyncCallbackRequestGuard, AsyncCallbackRequestId,
        WasmCallbackOutBuf, WasmCallbackOwner, rust_future_panic_message, rust_future_poll_sync,
        take_packed_utf8_string, write_return_slot,
    };
}
