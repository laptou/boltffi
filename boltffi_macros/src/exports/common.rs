use quote::quote;
use syn::{FnArg, Ident, ReturnType, Type};

pub(crate) fn is_factory_constructor(method: &syn::ImplItemFn, type_name: &syn::Ident) -> bool {
    FactoryMethodDescriptor::from_method(method, type_name).is_constructor()
}

pub(crate) fn is_result_of_self_type_path(path: &syn::Path, type_name: &syn::Ident) -> bool {
    FactoryReturnShape::from_path(path, type_name).is_result_of_self()
}

pub(crate) fn exported_methods(
    item_impl: &syn::ItemImpl,
) -> impl Iterator<Item = &syn::ImplItemFn> + '_ {
    item_impl
        .items
        .iter()
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method) => Some(method),
            _ => None,
        })
        .filter(|method| matches!(method.vis, syn::Visibility::Public(_)))
        .filter(|method| {
            !method
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("skip"))
        })
}

pub(crate) fn impl_type_name(item_impl: &syn::ItemImpl) -> Option<syn::Ident> {
    match item_impl.self_ty.as_ref() {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.clone()),
        _ => None,
    }
}

enum FactoryMethodDescriptor {
    Constructor,
    NonConstructor,
}

impl FactoryMethodDescriptor {
    fn from_method(method: &syn::ImplItemFn, type_name: &syn::Ident) -> Self {
        let has_receiver = method
            .sig
            .inputs
            .first()
            .is_some_and(|arg| matches!(arg, FnArg::Receiver(_)));

        if has_receiver {
            return Self::NonConstructor;
        }

        if FactoryReturnShape::from_output(&method.sig.output, type_name).is_factory_return() {
            Self::Constructor
        } else {
            Self::NonConstructor
        }
    }

    fn is_constructor(&self) -> bool {
        matches!(self, Self::Constructor)
    }
}

enum FactoryReturnShape {
    SelfValue,
    ResultOfSelf,
    Other,
}

impl FactoryReturnShape {
    fn from_output(output: &ReturnType, type_name: &syn::Ident) -> Self {
        match output {
            ReturnType::Default => Self::Other,
            ReturnType::Type(_, rust_type) => Self::from_type(rust_type, type_name),
        }
    }

    fn from_type(rust_type: &Type, type_name: &syn::Ident) -> Self {
        match rust_type {
            Type::Path(type_path) => Self::from_path(&type_path.path, type_name),
            _ => Self::Other,
        }
    }

    fn from_path(path: &syn::Path, type_name: &syn::Ident) -> Self {
        if Self::is_self_type_path(path, type_name) {
            return Self::SelfValue;
        }

        let Some(result_segment) = path.segments.last() else {
            return Self::Other;
        };
        if result_segment.ident != "Result" {
            return Self::Other;
        }
        let syn::PathArguments::AngleBracketed(arguments) = &result_segment.arguments else {
            return Self::Other;
        };
        let Some(syn::GenericArgument::Type(Type::Path(ok_type_path))) = arguments.args.first()
        else {
            return Self::Other;
        };

        if Self::is_self_type_path(&ok_type_path.path, type_name) {
            Self::ResultOfSelf
        } else {
            Self::Other
        }
    }

    fn is_self_type_path(path: &syn::Path, type_name: &syn::Ident) -> bool {
        path.segments
            .last()
            .is_some_and(|segment| segment.ident == "Self" || segment.ident == *type_name)
    }

    fn is_factory_return(&self) -> bool {
        matches!(self, Self::SelfValue | Self::ResultOfSelf)
    }

    fn is_result_of_self(&self) -> bool {
        matches!(self, Self::ResultOfSelf)
    }
}

/// `Result<OkT, ErrT>` where `OkT` is a handle: write packed ok to `out_ok`, wire-encoded err to `err_out_*`, return `FfiStatus`.
pub(crate) fn fallible_direct_ok_export_body(
    call_expr: proc_macro2::TokenStream,
    conversions: &[proc_macro2::TokenStream],
    has_conversions: bool,
    full_result_ty: &Type,
    result_ident: &Ident,
) -> proc_macro2::TokenStream {
    let bind = if has_conversions {
        quote! {
            #(#conversions)*
            let #result_ident: #full_result_ty = #call_expr;
        }
    } else {
        quote! {
            let #result_ident: #full_result_ty = #call_expr;
        }
    };
    quote! {
        #bind
        match #result_ident {
            Ok(value) => {
                if !out_ok.is_null() {
                    unsafe {
                        *out_ok = ::boltffi::__private::Passable::pack(value);
                    }
                }
                if !err_out_ptr.is_null() {
                    unsafe {
                        *err_out_ptr = ::core::ptr::null_mut();
                    }
                }
                if !err_out_len.is_null() {
                    unsafe {
                        *err_out_len = 0;
                    }
                }
                ::boltffi::__private::FfiStatus::OK
            }
            Err(err) => {
                if !out_ok.is_null() {
                    unsafe {
                        *out_ok = ::core::default::Default::default();
                    }
                }
                let buf = ::boltffi::__private::FfiBuf::wire_encode(&err);
                let bytes = unsafe { buf.as_byte_slice() };
                if !err_out_ptr.is_null() && !err_out_len.is_null() {
                    if bytes.is_empty() {
                        unsafe {
                            *err_out_ptr = ::core::ptr::null_mut();
                            *err_out_len = 0;
                        }
                    } else {
                        unsafe extern "C" {
                            fn malloc(size: usize) -> *mut ::core::ffi::c_void;
                        }
                        let copied = unsafe { malloc(bytes.len()) as *mut u8 };
                        if copied.is_null() {
                            return ::boltffi::__private::FfiStatus::INTERNAL_ERROR;
                        }
                        unsafe {
                            ::core::ptr::copy_nonoverlapping(bytes.as_ptr(), copied, bytes.len());
                            *err_out_ptr = copied;
                            *err_out_len = bytes.len();
                        }
                    }
                }
                ::boltffi::__private::FfiStatus { code: 1 }
            }
        }
    }
}

/// wire-encode `err` into `err_out_ptr` / `err_out_len` (async `complete` has no `out_ok`; ok is the return value).
pub(crate) fn wire_encode_err_to_err_out_buffers(err_ref: &Ident) -> proc_macro2::TokenStream {
    quote! {
        let buf = ::boltffi::__private::FfiBuf::wire_encode(&#err_ref);
        let bytes = unsafe { buf.as_byte_slice() };
        if !err_out_ptr.is_null() && !err_out_len.is_null() {
            if bytes.is_empty() {
                unsafe {
                    *err_out_ptr = ::core::ptr::null_mut();
                    *err_out_len = 0;
                }
            } else {
                unsafe extern "C" {
                    fn malloc(size: usize) -> *mut ::core::ffi::c_void;
                }
                let copied = unsafe { malloc(bytes.len()) as *mut u8 };
                if copied.is_null() {
                    return ::core::default::Default::default();
                }
                unsafe {
                    ::core::ptr::copy_nonoverlapping(bytes.as_ptr(), copied, bytes.len());
                    *err_out_ptr = copied;
                    *err_out_len = bytes.len();
                }
            }
        }
    }
}
