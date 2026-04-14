use boltffi_ffi_rules::naming;
use syn::{Item, ItemMod, ItemTrait};

use crate::callbacks::trait_export::future;
use crate::index::{ModulePath, SourceModule};

#[derive(Clone, Default)]
pub struct CallbackTraitRegistry {
    entries: Vec<CallbackTraitEntry>,
}

#[derive(Clone)]
pub struct CallbackTraitResolution {
    pub foreign_path: syn::Path,
    pub is_object_safe: bool,
    pub supports_local_handle: bool,
    pub local_handle_path: syn::Path,
}

#[derive(Clone)]
struct CallbackTraitEntry {
    module_path: Vec<String>,
    trait_name: String,
    is_object_safe: bool,
    has_async_methods: bool,
}

struct CallbackTraitDescriptor<'a> {
    item_trait: &'a ItemTrait,
}

impl CallbackTraitRegistry {
    pub fn resolve(&self, trait_path: &syn::Path) -> Option<CallbackTraitResolution> {
        let module_path = ModulePath::from_syn_path(trait_path);
        let (trait_name, module_path) = module_path.as_strings().split_last()?;
        let matches = self
            .entries
            .iter()
            .filter(|entry| entry.trait_name == *trait_name)
            .filter(|entry| module_path.is_empty() || entry.module_path == module_path)
            .collect::<Vec<_>>();

        match matches.as_slice() {
            [entry] => Some(CallbackTraitResolution {
                foreign_path: entry.foreign_path(),
                is_object_safe: entry.is_object_safe,
                supports_local_handle: entry.supports_local_handle(),
                local_handle_path: entry.local_handle_path(),
            }),
            _ => None,
        }
    }
}

pub(super) fn build_callback_trait_registry(
    source_modules: &[SourceModule],
) -> syn::Result<CallbackTraitRegistry> {
    let mut entries = Vec::new();

    source_modules.iter().try_for_each(|source_module| {
        let mut collector = CallbackTraitCollector {
            module_path: source_module.module_path().clone().into_strings(),
            entries: &mut entries,
        };
        source_module
            .syntax()
            .items
            .iter()
            .try_for_each(|item| collector.collect_item(item))
    })?;

    Ok(CallbackTraitRegistry { entries })
}

struct CallbackTraitCollector<'a> {
    module_path: Vec<String>,
    entries: &'a mut Vec<CallbackTraitEntry>,
}

impl<'a> CallbackTraitCollector<'a> {
    fn collect_item(&mut self, item: &Item) -> syn::Result<()> {
        match item {
            Item::Trait(item_trait) => self.collect_trait(item_trait),
            Item::Mod(item_mod) => self.collect_mod(item_mod),
            _ => Ok(()),
        }
    }

    fn collect_trait(&mut self, item_trait: &ItemTrait) -> syn::Result<()> {
        let trait_descriptor = CallbackTraitDescriptor::new(item_trait);
        if !trait_descriptor.is_exported_callback() {
            return Ok(());
        }

        let entry = CallbackTraitEntry {
            module_path: self.module_path.clone(),
            trait_name: item_trait.ident.to_string(),
            is_object_safe: trait_descriptor.is_object_safe(),
            has_async_methods: trait_descriptor.needs_async_runtime(),
        };
        self.entries.push(entry);
        Ok(())
    }

    fn collect_mod(&mut self, item_mod: &ItemMod) -> syn::Result<()> {
        let Some((_, items)) = &item_mod.content else {
            return Ok(());
        };
        let mut next_path = self.module_path.clone();
        next_path.push(item_mod.ident.to_string());
        let mut nested = CallbackTraitCollector {
            module_path: next_path,
            entries: self.entries,
        };
        items.iter().try_for_each(|item| nested.collect_item(item))
    }
}

impl<'a> CallbackTraitDescriptor<'a> {
    fn new(item_trait: &'a ItemTrait) -> Self {
        Self { item_trait }
    }

    fn is_exported_callback(&self) -> bool {
        self.item_trait.attrs.iter().any(|attr| {
            attr.path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "export" || segment.ident == "ffi_trait")
        })
    }

    /// `async fn` methods only (used for dyn-safety; boxed-future returns are still object-safe).
    fn has_async_fn_methods(&self) -> bool {
        self.item_trait.items.iter().any(|item| {
            matches!(
                item,
                syn::TraitItem::Fn(method) if method.sig.asyncness.is_some()
            )
        })
    }

    fn has_future_return_methods(&self) -> bool {
        self.item_trait.items.iter().any(|item| {
            matches!(
                item,
                syn::TraitItem::Fn(method) if future::trait_method_returns_boxed_future(method)
            )
        })
    }

    /// async fn or boxed-future return — needs async completion path; disables local handle.
    fn needs_async_runtime(&self) -> bool {
        self.has_async_fn_methods() || self.has_future_return_methods()
    }

    fn has_async_trait_attr(&self) -> bool {
        self.item_trait.attrs.iter().any(|attr| {
            attr.path()
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "async_trait")
        })
    }

    fn is_object_safe(&self) -> bool {
        !self.has_async_fn_methods() || self.has_async_trait_attr()
    }
}

impl CallbackTraitEntry {
    fn callback_handle_helper_name(&self) -> String {
        format!(
            "__boltffi_local_{}_handle",
            naming::to_snake_case(&self.trait_name)
        )
    }

    fn foreign_path(&self) -> syn::Path {
        let mut segments = Vec::with_capacity(self.module_path.len() + 2);
        segments.push("crate".to_string());
        segments.extend(self.module_path.iter().cloned());
        segments.push(format!("Foreign{}", self.trait_name));
        syn::parse_str(&segments.join("::")).unwrap_or_else(|_| syn::Path {
            leading_colon: None,
            segments: syn::punctuated::Punctuated::new(),
        })
    }

    fn local_handle_path(&self) -> syn::Path {
        let mut segments = Vec::with_capacity(self.module_path.len() + 2);
        segments.push("crate".to_string());
        segments.extend(self.module_path.iter().cloned());
        segments.push(self.callback_handle_helper_name());
        syn::parse_str(&segments.join("::")).unwrap_or_else(|_| syn::Path {
            leading_colon: None,
            segments: syn::punctuated::Punctuated::new(),
        })
    }

    fn supports_local_handle(&self) -> bool {
        self.is_object_safe && !self.has_async_methods
    }
}
