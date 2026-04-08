//! types with `#[export] impl` (ffi classes) — indexed for handle-style lowering.

use std::collections::HashMap;
use syn::{Item, ItemImpl, Type};

use crate::index::SourceModule;

#[derive(Default, Clone)]
pub struct ExportedClassRegistry {
    paths: HashMap<Vec<String>, ()>,
    unique_name_paths: HashMap<String, Vec<String>>,
}

impl ExportedClassRegistry {
    pub fn is_exported_class_type(&self, ty: &Type) -> bool {
        let Some(segments) = type_path_segments(Self::peel_reference(ty)) else {
            return false;
        };
        self.lookup_qualified(&segments)
    }

    fn peel_reference(ty: &Type) -> &Type {
        match ty {
            Type::Reference(r) => Self::peel_reference(r.elem.as_ref()),
            other => other,
        }
    }

    fn lookup_qualified(&self, segments: &[String]) -> bool {
        if segments.len() == 1 {
            return self
                .unique_name_paths
                .get(&segments[0])
                .map(|path| self.paths.contains_key(path))
                .unwrap_or(false);
        }
        if self.paths.contains_key(&segments.to_vec()) {
            return true;
        }
        self.paths
            .keys()
            .any(|registered| segments.ends_with(registered.as_slice()))
    }
}

fn type_path_segments(ty: &Type) -> Option<Vec<String>> {
    match ty {
        Type::Path(type_path) if type_path.qself.is_none() => Some(
            type_path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect(),
        ),
        Type::Group(group) => type_path_segments(group.elem.as_ref()),
        Type::Paren(paren) => type_path_segments(paren.elem.as_ref()),
        _ => None,
    }
}

pub(super) fn build_exported_class_registry(
    source_modules: &[SourceModule],
) -> syn::Result<ExportedClassRegistry> {
    let mut registry = ExportedClassRegistry::default();
    source_modules.iter().try_for_each(|source_module| {
        let mut collector = ExportedClassCollector {
            module_path: source_module.module_path().clone().into_strings(),
            registry: &mut registry,
        };
        source_module
            .syntax()
            .items
            .iter()
            .try_for_each(|item| collector.collect_item(item))
    })?;
    registry.finalize_unique_names();
    Ok(registry)
}

struct ExportedClassCollector<'a> {
    module_path: Vec<String>,
    registry: &'a mut ExportedClassRegistry,
}

impl<'a> ExportedClassCollector<'a> {
    fn collect_item(&mut self, item: &Item) -> syn::Result<()> {
        match item {
            Item::Impl(item_impl) if has_export_attr(item_impl) => {
                self.collect_export_impl(item_impl);
                Ok(())
            }
            Item::Mod(item_mod) => {
                let Some((_, items)) = &item_mod.content else {
                    return Ok(());
                };
                self.module_path.push(item_mod.ident.to_string());
                let r = items
                    .iter()
                    .try_for_each(|nested| self.collect_item(nested));
                self.module_path.pop();
                r
            }
            _ => Ok(()),
        }
    }

    fn collect_export_impl(&mut self, item_impl: &ItemImpl) {
        let Some(type_ident) = impl_self_type_ident(item_impl) else {
            return;
        };
        let mut qualified = self.module_path.clone();
        qualified.push(type_ident);
        self.registry.paths.insert(qualified, ());
    }
}

impl ExportedClassRegistry {
    fn finalize_unique_names(&mut self) {
        let name_counts = self.paths.keys().fold(
            HashMap::<String, usize>::new(),
            |mut counts, qualified_path| {
                if let Some(name) = qualified_path.last() {
                    *counts.entry(name.clone()).or_insert(0) += 1;
                }
                counts
            },
        );
        self.unique_name_paths = self.paths.keys().fold(
            HashMap::<String, Vec<String>>::new(),
            |mut unique, qualified_path| {
                if let Some(name) = qualified_path.last() {
                    if name_counts.get(name).copied() == Some(1) {
                        unique.insert(name.clone(), qualified_path.clone());
                    }
                }
                unique
            },
        );
    }
}

fn has_export_attr(item_impl: &ItemImpl) -> bool {
    item_impl.attrs.iter().any(|attr| {
        attr.path().is_ident("export")
            || attr
                .path()
                .segments
                .last()
                .is_some_and(|s| s.ident == "export")
    })
}

fn impl_self_type_ident(item_impl: &ItemImpl) -> Option<String> {
    match item_impl.self_ty.as_ref() {
        Type::Path(type_path) if type_path.qself.is_none() => type_path
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string()),
        _ => None,
    }
}
