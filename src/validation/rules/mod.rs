mod no_empty_placeholders;
mod brand_colors_match;
mod max_pages;
mod styles_exist;
mod cross_references_valid;
mod ooxml_valid;

pub use no_empty_placeholders::NoEmptyPlaceholders;
pub use brand_colors_match::BrandColorsMatch;
pub use max_pages::MaxPages;
pub use styles_exist::StylesExist;
pub use cross_references_valid::CrossReferencesValid;
pub use ooxml_valid::OOXMLValid;

use crate::validation::ValidationRule;
use std::collections::HashMap;
use std::sync::Arc;

pub struct RuleRegistry {
    rules: HashMap<&'static str, Arc<dyn ValidationRule>>,
    active_filter: Option<Vec<String>>,
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            active_filter: None,
        }
    }

    pub fn register(&mut self, rule: Box<dyn ValidationRule>) {
        let name: &'static str = rule.name();
        self.rules.insert(name, Arc::from(rule));
    }

    pub fn set_active_filter(&mut self, filter: Option<Vec<String>>) {
        self.active_filter = filter;
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn ValidationRule>> {
        self.rules.get(name).cloned()
    }

    pub fn all(&self) -> Vec<Arc<dyn ValidationRule>> {
        self.rules.values().cloned().collect()
    }

    pub fn resolve(&self, check_names: Option<&[String]>) -> Vec<Arc<dyn ValidationRule>> {
        let names: Vec<&str> = match check_names {
            Some(names) => names.iter().map(|s| s.as_str()).collect(),
            None => match &self.active_filter {
                Some(filter) => filter.iter().map(|s| s.as_str()).collect(),
                None => return self.all(),
            },
        };
        names
            .into_iter()
            .filter_map(|n| self.rules.get(n))
            .cloned()
            .collect()
    }
}
