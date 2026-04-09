use quote::quote;

use crate::index::custom_types;
use crate::lowering::returns::lower::encoded_return_buffer_expression;
use crate::lowering::returns::model::{
    EncodedReturnStrategy, ErrorReturnStrategy, ResolvedReturn, ValueReturnStrategy,
};

pub(crate) struct AsyncExportNames {
    entry: syn::Ident,
    poll: syn::Ident,
    poll_sync: syn::Ident,
    complete: syn::Ident,
    panic_message: syn::Ident,
    cancel: syn::Ident,
    free: syn::Ident,
}

pub(crate) struct AsyncRuntimeExports<'a> {
    pub(crate) visibility: &'a syn::Visibility,
    pub(crate) names: &'a AsyncExportNames,
    pub(crate) rust_return_type: proc_macro2::TokenStream,
    pub(crate) ffi_return_type: proc_macro2::TokenStream,
    pub(crate) complete_conversion: proc_macro2::TokenStream,
    pub(crate) default_value: proc_macro2::TokenStream,
}

pub(crate) struct AsyncWasmCompleteExport {
    pub(crate) params: proc_macro2::TokenStream,
    pub(crate) return_type: proc_macro2::TokenStream,
    pub(crate) body: proc_macro2::TokenStream,
}

/// wasm `complete` export for async futures: shared by class exports and free-function exports.
/// keeps wasm/native completion behavior aligned across instance, static, and `fn` entry points.
pub(crate) fn wasm_complete_export_for_async(
    return_abi: &ResolvedReturn,
    rust_return_type: &proc_macro2::TokenStream,
) -> AsyncWasmCompleteExport {
    if return_abi.is_primitive_scalar() {
        let rust_type = return_abi.rust_type();
        AsyncWasmCompleteExport {
            params: quote! { handle: ::boltffi::__private::RustFutureHandle },
            return_type: quote! { -> #rust_type },
            body: quote! {
                match ::boltffi::__private::rustfuture::rust_future_complete::<#rust_return_type>(handle) {
                    Some(result) => result,
                    None => Default::default(),
                }
            },
        }
    } else if return_abi.is_unit() {
        AsyncWasmCompleteExport {
            params: quote! { handle: ::boltffi::__private::RustFutureHandle },
            return_type: quote! {},
            body: quote! {
                let _ = ::boltffi::__private::rustfuture::rust_future_complete::<#rust_return_type>(handle);
            },
        }
    } else if matches!(
        return_abi.value_return_strategy(),
        ValueReturnStrategy::CompositeValue
    ) {
        AsyncWasmCompleteExport {
            params: quote! {
                out: *mut ::boltffi::__private::FfiBuf,
                handle: ::boltffi::__private::RustFutureHandle,
                _out_status: *mut ::boltffi::__private::FfiStatus
            },
            return_type: quote! {},
            body: quote! {
                if out.is_null() {
                    return;
                }
                let buf = match ::boltffi::__private::rustfuture::rust_future_complete::<#rust_return_type>(handle) {
                    Some(result) => ::boltffi::__private::FfiBuf::from_vec(vec![result]),
                    None => ::boltffi::__private::FfiBuf::empty(),
                };
                out.write(buf);
            },
        }
    } else if return_abi.is_passable_value() {
        let rust_type = return_abi.rust_type();
        AsyncWasmCompleteExport {
            params: quote! { handle: ::boltffi::__private::RustFutureHandle },
            return_type: quote! { -> <#rust_type as ::boltffi::__private::Passable>::Out },
            body: quote! {
                match ::boltffi::__private::rustfuture::rust_future_complete::<#rust_return_type>(handle) {
                    Some(result) => ::boltffi::__private::Passable::pack(result),
                    None => Default::default(),
                }
            },
        }
    } else if matches!(
        return_abi.value_return_strategy(),
        ValueReturnStrategy::ObjectHandle | ValueReturnStrategy::CallbackHandle
    ) && return_abi.error_strategy() == ErrorReturnStrategy::Encoded
    {
        let ok_ty = return_abi
            .fallible_ok_type()
            .expect("fallible handle async return must parse Result ok type");
        AsyncWasmCompleteExport {
            params: quote! { handle: ::boltffi::__private::RustFutureHandle },
            return_type: quote! { -> <#ok_ty as ::boltffi::__private::Passable>::Out },
            body: quote! {
                match ::boltffi::__private::rustfuture::rust_future_complete::<#rust_return_type>(handle) {
                    Some(Ok(value)) => ::boltffi::__private::Passable::pack(value),
                    Some(Err(err)) => {
                        ::boltffi::__private::set_last_error(format!("{err:?}"));
                        ::core::default::Default::default()
                    }
                    None => ::core::default::Default::default(),
                }
            },
        }
    } else if let Some(strategy) = return_abi.encoded_return_strategy() {
        let rust_type = return_abi.rust_type();
        let registry = custom_types::registry_for_current_crate().ok();
        let result_ident = syn::Ident::new("result", proc_macro2::Span::call_site());
        let encode_expression = if matches!(strategy, EncodedReturnStrategy::Utf8String) {
            quote! { ::boltffi::__private::FfiBuf::wire_encode(&#result_ident) }
        } else {
            encoded_return_buffer_expression(rust_type, strategy, &result_ident, registry.as_ref())
        };
        AsyncWasmCompleteExport {
            params: quote! {
                out: *mut ::boltffi::__private::FfiBuf,
                handle: ::boltffi::__private::RustFutureHandle,
                _out_status: *mut ::boltffi::__private::FfiStatus
            },
            return_type: quote! {},
            body: quote! {
                if out.is_null() {
                    return;
                }
                let buf = match ::boltffi::__private::rustfuture::rust_future_complete::<#rust_return_type>(handle) {
                    Some(#result_ident) => { #encode_expression },
                    None => ::boltffi::__private::FfiBuf::empty(),
                };
                out.write(buf);
            },
        }
    } else if matches!(
        return_abi.value_return_strategy(),
        ValueReturnStrategy::ObjectHandle | ValueReturnStrategy::CallbackHandle
    ) {
        let rust_type = return_abi.rust_type();
        AsyncWasmCompleteExport {
            params: quote! { handle: ::boltffi::__private::RustFutureHandle },
            return_type: quote! { -> <#rust_type as ::boltffi::__private::Passable>::Out },
            body: quote! {
                match ::boltffi::__private::rustfuture::rust_future_complete::<#rust_return_type>(handle) {
                    Some(result) => ::boltffi::__private::Passable::pack(result),
                    None => ::core::default::Default::default(),
                }
            },
        }
    } else {
        AsyncWasmCompleteExport {
            params: quote! {
                out: *mut ::boltffi::__private::FfiBuf,
                handle: ::boltffi::__private::RustFutureHandle,
                _out_status: *mut ::boltffi::__private::FfiStatus
            },
            return_type: quote! {},
            body: quote! {
                if out.is_null() {
                    return;
                }
                let buf = match ::boltffi::__private::rustfuture::rust_future_complete::<#rust_return_type>(handle) {
                    Some(result) => ::boltffi::__private::FfiBuf::wire_encode(&result),
                    None => ::boltffi::__private::FfiBuf::empty(),
                };
                out.write(buf);
            },
        }
    }
}

impl AsyncExportNames {
    pub(crate) fn new(base_name: &str, span: proc_macro2::Span) -> Self {
        Self {
            entry: syn::Ident::new(base_name, span),
            poll: syn::Ident::new(&format!("{}_poll", base_name), span),
            poll_sync: syn::Ident::new(&format!("{}_poll_sync", base_name), span),
            complete: syn::Ident::new(&format!("{}_complete", base_name), span),
            panic_message: syn::Ident::new(&format!("{}_panic_message", base_name), span),
            cancel: syn::Ident::new(&format!("{}_cancel", base_name), span),
            free: syn::Ident::new(&format!("{}_free", base_name), span),
        }
    }

    pub(crate) fn entry(&self) -> &syn::Ident {
        &self.entry
    }
}

impl<'a> AsyncRuntimeExports<'a> {
    pub(crate) fn render(
        &self,
        wasm_complete: AsyncWasmCompleteExport,
    ) -> proc_macro2::TokenStream {
        let native_complete = self.render_native_complete();
        let wasm_complete = self.render_wasm_complete(wasm_complete);
        let native_poll = self.render_native_poll();
        let wasm_poll = self.render_wasm_poll();
        let wasm_panic_message = self.render_wasm_panic_message();
        let cancel = self.render_cancel();
        let free = self.render_free();

        quote! {
            #native_poll
            #wasm_poll
            #wasm_panic_message
            #native_complete
            #wasm_complete
            #cancel
            #free
        }
    }

    fn render_native_complete(&self) -> proc_macro2::TokenStream {
        let visibility = self.visibility;
        let complete_ident = &self.names.complete;
        let rust_return_type = &self.rust_return_type;
        let ffi_return_type = &self.ffi_return_type;
        let complete_conversion = &self.complete_conversion;
        let default_value = &self.default_value;

        quote! {
            #[cfg(not(target_arch = "wasm32"))]
            #[unsafe(no_mangle)]
            #visibility unsafe extern "C" fn #complete_ident(
                handle: ::boltffi::__private::RustFutureHandle,
                out_status: *mut ::boltffi::__private::FfiStatus,
            ) -> #ffi_return_type {
                match ::boltffi::__private::rustfuture::rust_future_complete::<#rust_return_type>(handle) {
                    Some(result) => { #complete_conversion }
                    None => {
                        if !out_status.is_null() { *out_status = ::boltffi::__private::FfiStatus::CANCELLED; }
                        #default_value
                    }
                }
            }
        }
    }

    fn render_wasm_complete(&self, export: AsyncWasmCompleteExport) -> proc_macro2::TokenStream {
        let visibility = self.visibility;
        let complete_ident = &self.names.complete;
        let params = export.params;
        let return_type = export.return_type;
        let body = export.body;

        quote! {
            #[cfg(target_arch = "wasm32")]
            #[unsafe(no_mangle)]
            #visibility unsafe extern "C" fn #complete_ident(
                #params
            ) #return_type {
                #body
            }
        }
    }

    fn render_native_poll(&self) -> proc_macro2::TokenStream {
        let visibility = self.visibility;
        let poll_ident = &self.names.poll;
        let rust_return_type = &self.rust_return_type;

        quote! {
            #[cfg(not(target_arch = "wasm32"))]
            #[unsafe(no_mangle)]
            #visibility unsafe extern "C" fn #poll_ident(
                handle: ::boltffi::__private::RustFutureHandle,
                callback_data: u64,
                callback: ::boltffi::__private::RustFutureContinuationCallback,
            ) {
                ::boltffi::__private::rustfuture::rust_future_poll::<#rust_return_type>(handle, callback, callback_data)
            }
        }
    }

    fn render_wasm_poll(&self) -> proc_macro2::TokenStream {
        let visibility = self.visibility;
        let poll_sync_ident = &self.names.poll_sync;
        let rust_return_type = &self.rust_return_type;

        quote! {
            #[cfg(target_arch = "wasm32")]
            #[unsafe(no_mangle)]
            #visibility unsafe extern "C" fn #poll_sync_ident(
                handle: ::boltffi::__private::RustFutureHandle,
            ) -> i32 {
                ::boltffi::__private::rust_future_poll_sync::<#rust_return_type>(handle)
            }
        }
    }

    fn render_wasm_panic_message(&self) -> proc_macro2::TokenStream {
        let visibility = self.visibility;
        let panic_message_ident = &self.names.panic_message;
        let rust_return_type = &self.rust_return_type;

        quote! {
            #[cfg(target_arch = "wasm32")]
            #[unsafe(no_mangle)]
            #visibility unsafe extern "C" fn #panic_message_ident(
                handle: ::boltffi::__private::RustFutureHandle,
            ) -> ::boltffi::__private::FfiBuf {
                match ::boltffi::__private::rust_future_panic_message::<#rust_return_type>(handle) {
                    Some(message) => ::boltffi::__private::FfiBuf::wire_encode(&message),
                    None => ::boltffi::__private::FfiBuf::empty(),
                }
            }
        }
    }

    fn render_cancel(&self) -> proc_macro2::TokenStream {
        let visibility = self.visibility;
        let cancel_ident = &self.names.cancel;
        let rust_return_type = &self.rust_return_type;

        quote! {
            #[unsafe(no_mangle)]
            #visibility unsafe extern "C" fn #cancel_ident(handle: ::boltffi::__private::RustFutureHandle) {
                ::boltffi::__private::rustfuture::rust_future_cancel::<#rust_return_type>(handle)
            }
        }
    }

    fn render_free(&self) -> proc_macro2::TokenStream {
        let visibility = self.visibility;
        let free_ident = &self.names.free;
        let rust_return_type = &self.rust_return_type;

        quote! {
            #[unsafe(no_mangle)]
            #visibility unsafe extern "C" fn #free_ident(handle: ::boltffi::__private::RustFutureHandle) {
                ::boltffi::__private::rustfuture::rust_future_free::<#rust_return_type>(handle)
            }
        }
    }
}
