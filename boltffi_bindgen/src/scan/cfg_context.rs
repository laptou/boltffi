//! cfg evaluation for source scanning: filter items/variants/fields per target features.

use std::collections::HashSet;

use cfg_expr::{Expression, Predicate, TargetPredicate};
use syn::Attribute;

/// active cfg predicates for the scan target. when `scan_all` is true, cfg is ignored (legacy behavior).
#[derive(Debug, Clone)]
pub struct CfgContext {
    pub features: HashSet<String>,
    pub target_arch: Option<String>,
    /// when true, do not filter by cfg (scan everything)
    pub scan_all: bool,
}

impl Default for CfgContext {
    fn default() -> Self {
        Self {
            features: HashSet::new(),
            target_arch: None,
            scan_all: true,
        }
    }
}

impl CfgContext {
    /// true if this item/variant/field should be included for bindgen.
    /// items with no `#[cfg]` are always included.
    pub fn is_active(&self, attrs: &[Attribute]) -> bool {
        if self.scan_all {
            return true;
        }
        let cfg_attrs: Vec<&Attribute> = attrs.iter().filter(|a| a.path().is_ident("cfg")).collect();
        if cfg_attrs.is_empty() {
            return true;
        }
        cfg_attrs.iter().all(|attr| self.eval_cfg_attr(attr))
    }

    fn eval_cfg_attr(&self, attr: &Attribute) -> bool {
        let syn::Meta::List(list) = &attr.meta else {
            return true;
        };
        let expr_str = list.tokens.to_string();
        let Ok(expr) = Expression::parse(&expr_str) else {
            return true;
        };
        expr.eval(|pred| self.eval_predicate(pred))
    }

    fn eval_predicate(&self, pred: &Predicate<'_>) -> bool {
        match pred {
            Predicate::Feature(name) => self.features.contains(*name),
            Predicate::Target(TargetPredicate::Arch(arch)) => self
                .target_arch
                .as_ref()
                .map_or(false, |a| a == arch.as_str()),
            Predicate::Target(TargetPredicate::PointerWidth(w)) => {
                let _ = w;
                true
            }
            Predicate::Target(_) => true,
            Predicate::Test => false,
            Predicate::DebugAssertions => false,
            Predicate::ProcMacro => false,
            Predicate::TargetFeature(_) => false,
            Predicate::Flag(_) => true,
            Predicate::KeyValue { key, val } if *key == "feature" => self.features.contains(*val),
            Predicate::KeyValue { key, val } if *key == "target_arch" => {
                self.target_arch.as_ref().map_or(false, |a| a == val)
            }
            Predicate::KeyValue { .. } => true,
        }
    }
}
