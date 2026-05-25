use boltffi_ffi_rules::classification::PassableCategory;
use std::collections::HashMap;
use syn::{Item, ItemEnum, ItemStruct, Type};

use crate::data::analysis::{EnumDataShape, StructDataShape};
use crate::index::{CrateIndex, SourceModule};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataTypeCategory {
    Scalar,
    Blittable,
    WireEncoded,
}

impl DataTypeCategory {
    pub fn supports_direct_vec(self) -> bool {
        matches!(self, Self::Blittable)
    }
}

#[derive(Default, Clone)]
pub struct DataTypeRegistry {
    categories_by_path: HashMap<Vec<String>, DataTypeCategory>,
    unique_name_categories: HashMap<String, DataTypeCategory>,
}

struct TypePathKey {
    segments: Vec<String>,
}

impl DataTypeRegistry {
    fn insert(&mut self, qualified_path: Vec<String>, category: DataTypeCategory) {
        self.categories_by_path.insert(qualified_path, category);
    }

    fn finalize_unique_names(&mut self) {
        let name_counts = self.categories_by_path.keys().fold(
            HashMap::<String, usize>::new(),
            |mut counts, qualified_path| {
                if let Some(name) = qualified_path.last() {
                    *counts.entry(name.clone()).or_insert(0) += 1;
                }
                counts
            },
        );

        self.unique_name_categories = self.categories_by_path.iter().fold(
            HashMap::<String, DataTypeCategory>::new(),
            |mut unique, (qualified_path, category)| {
                if let Some(name) = qualified_path.last()
                    && name_counts.get(name).copied() == Some(1)
                {
                    unique.insert(name.clone(), *category);
                }
                unique
            },
        );
    }

    pub fn category_for(&self, ty: &Type) -> Option<DataTypeCategory> {
        let type_path_key = TypePathKey::from_type(ty)?;
        if type_path_key.segments.len() == 1 {
            return type_path_key
                .segments
                .first()
                .and_then(|name| self.unique_name_categories.get(name).copied());
        }

        if let Some(category) = self
            .categories_by_path
            .get(&type_path_key.segments)
            .copied()
        {
            return Some(category);
        }

        let mut matches = self
            .categories_by_path
            .iter()
            .filter(|(registered_path, _)| type_path_key.has_suffix(registered_path))
            .map(|(_, category)| *category);
        let first = matches.next()?;
        matches.all(|next| next == first).then_some(first)
    }
}

#[cfg(test)]
impl DataTypeRegistry {
    pub(crate) fn with_entries(entries: &[(&str, DataTypeCategory)]) -> Self {
        let mut registry = DataTypeRegistry::default();
        for (name, category) in entries {
            registry.insert(vec![name.to_string()], *category);
        }
        registry.finalize_unique_names();
        registry
    }
}

pub fn registry_for_current_crate() -> syn::Result<DataTypeRegistry> {
    Ok(CrateIndex::for_current_crate()?.data_types().clone())
}

pub(super) fn build_data_type_registry(
    source_modules: &[SourceModule],
) -> syn::Result<DataTypeRegistry> {
    let mut registry = DataTypeRegistry::default();
    collect_root_types(source_modules, &mut registry)?;

    registry.finalize_unique_names();
    Ok(registry)
}

fn collect_root_types(
    source_modules: &[SourceModule],
    registry: &mut DataTypeRegistry,
) -> syn::Result<()> {
    source_modules.iter().try_for_each(|source_module| {
        let mut collector = DataTypeCollector {
            module_path: source_module.module_path().clone().into_strings(),
            registry,
        };
        source_module
            .syntax()
            .items
            .iter()
            .try_for_each(|item| collector.collect_item(item))
    })
}

struct DataTypeCollector<'a> {
    module_path: Vec<String>,
    registry: &'a mut DataTypeRegistry,
}

impl<'a> DataTypeCollector<'a> {
    fn collect_item(&mut self, item: &Item) -> syn::Result<()> {
        match item {
            Item::Struct(item_struct) => {
                self.collect_struct(item_struct);
                Ok(())
            }
            Item::Enum(item_enum) => {
                self.collect_enum(item_enum);
                Ok(())
            }
            Item::Mod(item_mod) => {
                let Some((_, items)) = &item_mod.content else {
                    return Ok(());
                };
                self.module_path.push(item_mod.ident.to_string());
                let collect_result = items
                    .iter()
                    .try_for_each(|nested| self.collect_item(nested));
                self.module_path.pop();
                collect_result
            }
            _ => Ok(()),
        }
    }

    fn collect_struct(&mut self, item_struct: &ItemStruct) {
        if !StructDataShape::new(item_struct).is_boltffi_data() {
            return;
        }
        let category = classify_struct_category(item_struct);
        let mut qualified_path = self.module_path.clone();
        qualified_path.push(item_struct.ident.to_string());
        self.registry.insert(qualified_path, category);
    }

    fn collect_enum(&mut self, item_enum: &ItemEnum) {
        if !EnumDataShape::new(item_enum).is_boltffi_data() {
            return;
        }
        let category = classify_enum_category(item_enum);
        let mut qualified_path = self.module_path.clone();
        qualified_path.push(item_enum.ident.to_string());
        self.registry.insert(qualified_path, category);
    }
}

impl TypePathKey {
    fn from_type(ty: &Type) -> Option<Self> {
        match ty {
            Type::Path(type_path) if type_path.qself.is_none() => Some(Self {
                segments: type_path
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect(),
            }),
            Type::Group(group) => Self::from_type(group.elem.as_ref()),
            Type::Paren(paren) => Self::from_type(paren.elem.as_ref()),
            _ => None,
        }
    }

    fn has_suffix(&self, suffix: &[String]) -> bool {
        self.segments.len() >= suffix.len()
            && self.segments[self.segments.len() - suffix.len()..] == *suffix
    }
}

fn classify_struct_category(item_struct: &ItemStruct) -> DataTypeCategory {
    match StructDataShape::new(item_struct).passable_category() {
        PassableCategory::Blittable => DataTypeCategory::Blittable,
        PassableCategory::Scalar | PassableCategory::WireEncoded => DataTypeCategory::WireEncoded,
    }
}

fn classify_enum_category(item_enum: &ItemEnum) -> DataTypeCategory {
    match EnumDataShape::new(item_enum).passable_category() {
        PassableCategory::Scalar => DataTypeCategory::Scalar,
        PassableCategory::Blittable | PassableCategory::WireEncoded => {
            DataTypeCategory::WireEncoded
        }
    }
}
