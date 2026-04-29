//! shared parsing for `Pin<Box<dyn Future<Output = T> + Send>>` / `Box<dyn Future<...>>`
//! return types on sync trait methods (dyn-safe async traits).
//!
//! keep bindgen (`boltffi_bindgen::scan`) and macros (`boltffi_macros::callbacks::trait_export`)
//! agreeing by sourcing this module — drift broke indexeddb-style callbacks before.

use syn::{ReturnType, TraitItemFn, Type};

/// if `ty` is `Box<Inner>` or `Pin<Box<Inner>>`, return `Inner`.
fn strip_box_or_pin_box(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    match last.ident.to_string().as_str() {
        "Box" => {
            let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                return None;
            };
            match args.args.first()? {
                syn::GenericArgument::Type(inner) => Some(inner),
                _ => None,
            }
        }
        "Pin" => {
            let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
                return None;
            };
            let inner = match args.args.first()? {
                syn::GenericArgument::Type(t) => t,
                _ => return None,
            };
            strip_box_or_pin_box(inner)
        }
        _ => None,
    }
}

/// `dyn Future<Output = T> + Send + ...` → `T`
fn future_output_from_trait_object(ty: &Type) -> Option<&Type> {
    let Type::TraitObject(obj) = ty else {
        return None;
    };
    for bound in &obj.bounds {
        let syn::TypeParamBound::Trait(trait_bound) = bound else {
            continue;
        };
        let Some(seg) = trait_bound.path.segments.last() else {
            continue;
        };
        if seg.ident != "Future" {
            continue;
        }
        let syn::PathArguments::AngleBracketed(ab) = &seg.arguments else {
            continue;
        };
        for arg in &ab.args {
            if let syn::GenericArgument::AssocType(assoc) = arg {
                if assoc.ident == "Output" {
                    return Some(&assoc.ty);
                }
            }
        }
    }
    None
}

/// `Box<dyn Future<Output = T> + Send>` / `Pin<Box<dyn Future<Output = T> + Send>>` → `T`
pub fn parse_boxed_future_output(ty: &Type) -> Option<&Type> {
    let inner = strip_box_or_pin_box(ty)?;
    future_output_from_trait_object(inner)
}

pub fn boxed_future_inner_output_ty(output: &ReturnType) -> Option<Type> {
    match output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => parse_boxed_future_output(ty).map(|t| t.clone()),
    }
}

/// sync method whose return is a boxed future (not `async fn`)
pub fn trait_method_returns_boxed_future(method: &TraitItemFn) -> bool {
    if method.sig.asyncness.is_some() {
        return false;
    }
    boxed_future_inner_output_ty(&method.sig.output).is_some()
}

/// for `fn -> Box<dyn Future<Output = T>>`, the `T` used for ffi lowering (same as `async fn -> T`)
pub fn future_method_inner_output(method: &TraitItemFn) -> Option<&syn::Type> {
    match &method.sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => parse_boxed_future_output(ty),
    }
}

/// `Future<Output = ()>` — same completion path as void `async fn`.
pub fn is_unit_future_output(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(t) if t.elems.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn detects_pin_box_dyn_future_return() {
        let method: TraitItemFn = parse_quote! {
            fn fetch(&self, key: u32) -> std::pin::Pin<
                std::boxed::Box<dyn std::future::Future<Output = u64> + Send + '_>,
            >;
        };
        assert!(trait_method_returns_boxed_future(&method));
        assert!(future_method_inner_output(&method).is_some());
    }

    #[test]
    fn async_fn_is_not_classified_as_boxed_future() {
        let method: TraitItemFn = parse_quote! {
            async fn fetch(&self, key: u32) -> u64;
        };
        assert!(!trait_method_returns_boxed_future(&method));
    }
}
