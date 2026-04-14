//! mirror `boltffi_macros::callbacks::trait_export::future` — bindgen cannot depend on the proc-macro crate.

use syn::{ReturnType, TraitItemFn, Type};

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

fn parse_boxed_future_output(ty: &Type) -> Option<&Type> {
    let inner = strip_box_or_pin_box(ty)?;
    future_output_from_trait_object(inner)
}

pub fn boxed_future_inner_output_ty(output: &ReturnType) -> Option<Type> {
    match output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => parse_boxed_future_output(ty).map(|t| t.clone()),
    }
}

pub fn trait_method_returns_boxed_future(method: &TraitItemFn) -> bool {
    if method.sig.asyncness.is_some() {
        return false;
    }
    boxed_future_inner_output_ty(&method.sig.output).is_some()
}
