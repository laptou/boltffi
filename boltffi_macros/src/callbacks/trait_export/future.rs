//! detect `Box<dyn Future<Output = T> + Send>` / `Pin<Box<...>>` return types for dyn-safe async traits.

use syn::{ReturnType, Type};

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

/// sync method whose return is a boxed future (not `async fn`)
pub fn trait_method_returns_boxed_future(method: &syn::TraitItemFn) -> bool {
    if method.sig.asyncness.is_some() {
        return false;
    }
    match &method.sig.output {
        ReturnType::Default => false,
        ReturnType::Type(_, ty) => parse_boxed_future_output(ty).is_some(),
    }
}

/// for `fn -> Box<dyn Future<Output = T>>`, the `T` used for ffi lowering (same as `async fn -> T`)
pub fn future_method_inner_output(method: &syn::TraitItemFn) -> Option<&syn::Type> {
    match &method.sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => parse_boxed_future_output(ty),
    }
}

/// `Future<Output = ()>` — use the same completion path as void `async fn`.
pub fn is_unit_future_output(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(t) if t.elems.is_empty())
}
