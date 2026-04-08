use proc_macro2::Span;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub(crate) mod callback_traits;
pub(crate) mod custom_types;
pub(crate) mod data_types;
pub(crate) mod exported_classes;
mod path_resolver;
mod source_tree;

pub(crate) use path_resolver::PathResolver;
pub(crate) use source_tree::{ModulePath, SourceModule, SourceTree};

#[derive(Clone)]
pub(crate) struct CrateIndex {
    custom_types: custom_types::CustomTypeRegistry,
    data_types: data_types::DataTypeRegistry,
    exported_classes: exported_classes::ExportedClassRegistry,
    callback_traits: callback_traits::CallbackTraitRegistry,
    path_resolver: PathResolver,
}

static CRATE_INDEX_CACHE: OnceLock<Mutex<HashMap<PathBuf, CrateIndex>>> = OnceLock::new();

impl CrateIndex {
    pub(crate) fn for_current_crate() -> syn::Result<Self> {
        let source_tree = SourceTree::for_current_crate()?;
        let manifest_dir = source_tree.manifest_dir().to_path_buf();

        let cache = CRATE_INDEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        if let Some(crate_index) = cache
            .lock()
            .map_err(|_| syn::Error::new(Span::call_site(), "crate index lock poisoned"))?
            .get(&manifest_dir)
            .cloned()
        {
            return Ok(crate_index);
        }

        let source_modules = source_tree.modules()?;
        let crate_index = Self {
            custom_types: custom_types::build_custom_type_registry(&source_modules)?,
            data_types: data_types::build_data_type_registry(&source_modules)?,
            exported_classes: exported_classes::build_exported_class_registry(&source_modules)?,
            callback_traits: callback_traits::build_callback_trait_registry(&source_modules)?,
            path_resolver: PathResolver::build(&source_modules),
        };

        cache
            .lock()
            .map_err(|_| syn::Error::new(Span::call_site(), "crate index lock poisoned"))?
            .insert(manifest_dir, crate_index.clone());

        Ok(crate_index)
    }

    pub(crate) fn custom_types(&self) -> &custom_types::CustomTypeRegistry {
        &self.custom_types
    }

    pub(crate) fn data_types(&self) -> &data_types::DataTypeRegistry {
        &self.data_types
    }

    pub(crate) fn exported_classes(&self) -> &exported_classes::ExportedClassRegistry {
        &self.exported_classes
    }

    pub(crate) fn callback_traits(&self) -> &callback_traits::CallbackTraitRegistry {
        &self.callback_traits
    }

    pub(crate) fn path_resolver(&self) -> &PathResolver {
        &self.path_resolver
    }
}
