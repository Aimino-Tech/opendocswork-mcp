pub mod rules;

use rules::RuleRegistry;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub passed: bool,
    pub summary: String,
    pub checks: Vec<ValidationCheck>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub rule: String,
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_args: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ValidateRequest {
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ValidationFixRequest {
    pub tool_name: String,
    pub args: serde_json::Value,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationFix {
    pub file_path: String,
    pub tool_name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillValidationConfig {
    pub rules: Vec<SkillValidationRule>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SkillValidationRule {
    Simple(String),
    WithConfig { name: String, params: serde_json::Value },
}

impl<'de> Deserialize<'de> for SkillValidationRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            String(String),
            Map(std::collections::HashMap<String, serde_json::Value>),
        }

        let raw = Raw::deserialize(deserializer)?;
        match raw {
            Raw::String(s) => Ok(SkillValidationRule::Simple(s)),
            Raw::Map(mut map) => {
                if let Some(params) = map.remove("params") {
                    let name = map
                        .remove("name")
                        .and_then(|v| v.as_str().map(String::from))
                        .ok_or_else(|| de::Error::missing_field("name"))?;
                    Ok(SkillValidationRule::WithConfig { name, params })
                } else if map.len() == 1 {
                    let (name, params) = map.into_iter().next()
                        .expect("map.len() == 1 checked above");
                    Ok(SkillValidationRule::WithConfig { name, params })
                } else {
                    Err(de::Error::custom(
                        "expected a single key-value pair or {name, params}",
                    ))
                }
            }
        }
    }
}

#[async_trait::async_trait]
pub trait ValidationRule: Send + Sync {
    fn name(&self) -> &'static str;
    async fn validate(&self, file_path: &str, config: Option<&serde_json::Value>) -> Result<ValidationCheck, anyhow::Error>;
}

pub struct ValidationEngine {
    registry: RuleRegistry,
    skill_config: Option<SkillValidationConfig>,
}

impl std::fmt::Debug for ValidationEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationEngine").finish()
    }
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationEngine {
    pub fn new() -> Self {
        let mut registry = RuleRegistry::new();
        registry.register(Box::new(rules::NoEmptyPlaceholders));
        registry.register(Box::new(rules::BrandColorsMatch));
        registry.register(Box::new(rules::MaxPages));
        registry.register(Box::new(rules::StylesExist));
        registry.register(Box::new(rules::CrossReferencesValid));
        registry.register(Box::new(rules::OOXMLValid));
        Self { registry, skill_config: None }
    }

    pub fn register(&mut self, rule: Box<dyn ValidationRule>) {
        self.registry.register(rule);
    }

    pub fn from_skill_config(config: &SkillValidationConfig) -> Self {
        let mut engine = Self::new();
        let config_names: Vec<String> = config
            .rules
            .iter()
            .map(|r| match r {
                SkillValidationRule::Simple(name) => name.clone(),
                SkillValidationRule::WithConfig { name, .. } => name.clone(),
            })
            .collect();
        engine.registry.set_active_filter(Some(config_names));
        engine.skill_config = Some(config.clone());
        engine
    }

    pub async fn validate(
        &self,
        file_path: &str,
        check_names: Option<&[String]>,
        rules_config: Option<&serde_json::Value>,
    ) -> ValidationReport {
        let start = Instant::now();
        let mut checks = Vec::new();

        let applicable: Vec<Arc<dyn ValidationRule>> = self.registry.resolve(check_names);

        let merged_config = self.merge_config(rules_config);

        for rule in &applicable {
            let rule_config = merged_config.as_ref().and_then(|c| c.get(rule.name()));
            match rule.validate(file_path, rule_config).await {
                Ok(check) => checks.push(check),
                Err(e) => checks.push(ValidationCheck {
                    rule: rule.name().to_string(),
                    passed: false,
                    details: Some(format!("Validation error: {}", e)),
                    fix_tool: None,
                    fix_args: None,
                }),
            }
        }

        let passed_count = checks.iter().filter(|c| c.passed).count();
        let duration_ms = start.elapsed().as_millis() as u64;

        ValidationReport {
            passed: passed_count == checks.len(),
            summary: format!("{}/{} checks passed", passed_count, checks.len()),
            checks,
            duration_ms,
        }
    }

    fn merge_config(&self, runtime_config: Option<&serde_json::Value>) -> Option<serde_json::Value> {
        let skill_params = self.skill_config.as_ref().and_then(|sc| {
            let params: serde_json::Value = sc.rules.iter().filter_map(|r| {
                match r {
                    SkillValidationRule::WithConfig { name, params } => {
                        Some((name.clone(), params.clone()))
                    }
                    _ => None,
                }
            }).collect::<serde_json::Map<_, _>>().into();
            if params.as_object().map(|m| m.is_empty()).unwrap_or(true) {
                None
            } else {
                Some(params)
            }
        });

        match (skill_params, runtime_config) {
            (Some(mut skill), Some(runtime)) => {
                if let Some(skill_map) = skill.as_object_mut() {
                    if let Some(runtime_map) = runtime.as_object() {
                        for (k, v) in runtime_map {
                            skill_map.insert(k.clone(), v.clone());
                        }
                    }
                }
                Some(skill)
            }
            (Some(skill), None) => Some(skill),
            (None, Some(runtime)) => Some(runtime.clone()),
            (None, None) => None,
        }
    }
}

pub async fn validate_after_skill(
    skill_config: &SkillValidationConfig,
    output_path: &str,
    runtime_config: Option<&serde_json::Value>,
) -> ValidationReport {
    let engine = ValidationEngine::from_skill_config(skill_config);
    engine.validate(output_path, None, runtime_config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_runs_all_rules() {
        let engine = ValidationEngine::new();
        let report = engine.validate("/nonexistent/file.docx", None, None).await;
        assert!(!report.passed);
        assert!(!report.checks.is_empty());
        assert!(report.duration_ms < 1000);
    }

    #[tokio::test]
    async fn test_engine_runs_selected_checks() {
        let engine = ValidationEngine::new();
        let check_names = vec!["ooxml_valid".to_string()];
        let report = engine
            .validate("/nonexistent/file.docx", Some(&check_names), None)
            .await;
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].rule, "ooxml_valid");
    }

    #[test]
    fn test_report_serialization() {
        let report = ValidationReport {
            passed: false,
            summary: "1/2 checks passed".into(),
            checks: vec![
                ValidationCheck {
                    rule: "no_empty_placeholders".into(),
                    passed: false,
                    details: Some("Placeholder {name} still empty".into()),
                    fix_tool: Some("office_replace_text".into()),
                    fix_args: Some(serde_json::json!({
                        "file_path": "/path/doc.docx",
                        "search": "{name}",
                        "replace": "Acme Corp"
                    })),
                },
                ValidationCheck {
                    rule: "max_pages".into(),
                    passed: true,
                    details: None,
                    fix_tool: None,
                    fix_args: None,
                },
            ],
            duration_ms: 15,
        };
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert!(json.contains("\"passed\": false"));
        assert!(json.contains("\"no_empty_placeholders\""));
        assert!(json.contains("\"office_replace_text\""));
    }

    #[test]
    fn test_validation_fix_serialization() {
        let fix = ValidationFix {
            file_path: "/path/doc.docx".into(),
            tool_name: "office_replace_text".into(),
            args: serde_json::json!({"search": "{name}", "replace": "Acme Corp"}),
        };
        let json = serde_json::to_string(&fix).unwrap();
        let parsed: ValidationFix = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tool_name, "office_replace_text");
        assert_eq!(parsed.file_path, "/path/doc.docx");
    }

    #[test]
    fn test_skill_rule_shorthand_format() {
        let with_shorthand = r##"{
            "rules": [
                "no_empty_placeholders",
                {"brand_colors_match": {"primary": "#1a1a2e"}},
                {"max_pages": 10},
                {"styles_exist": ["Heading 1", "Normal"]}
            ]
        }"##;
        let config: SkillValidationConfig = serde_json::from_str(with_shorthand).unwrap();
        assert_eq!(config.rules.len(), 4);
        match &config.rules[1] {
            SkillValidationRule::WithConfig { name, params } => {
                assert_eq!(name, "brand_colors_match");
                assert_eq!(params["primary"], "#1a1a2e");
            }
            _ => panic!("Expected WithConfig for shorthand"),
        }
        match &config.rules[2] {
            SkillValidationRule::WithConfig { name, params } => {
                assert_eq!(name, "max_pages");
                assert_eq!(params.as_u64(), Some(10));
            }
            _ => panic!("Expected WithConfig for shorthand number"),
        }
    }

    #[test]
    fn test_skill_validation_config() {
        let json_str = r##"{
            "rules": [
                "no_empty_placeholders",
                {"name": "brand_colors_match", "params": {"primary": "#1a1a2e", "accent": "#6c5ce7"}},
                "max_pages"
            ]
        }"##;
        let config: SkillValidationConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.rules.len(), 3);
        match &config.rules[1] {
            SkillValidationRule::WithConfig { name, params } => {
                assert_eq!(name, "brand_colors_match");
                assert_eq!(params["primary"], "#1a1a2e");
            }
            _ => panic!("Expected WithConfig variant"),
        }
    }
}
