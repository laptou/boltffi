use quote::quote;

/// wasm32 export ABI: panics unwind across the ffi boundary when EH is enabled.
pub(crate) fn wasm_export_abi() -> proc_macro2::TokenStream {
    quote! { "C-unwind" }
}

/// native host export ABI (swift, jni, etc.).
pub(crate) fn native_export_abi() -> proc_macro2::TokenStream {
    quote! { "C" }
}
