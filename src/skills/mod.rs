pub mod registry;
pub mod runner;
pub mod validator;

use serde::{Deserialize, Serialize};

// ── Category ──────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SkillCategory {
    #[default]
    Excel,
    Word,
    Ppt,
    Multi,
}

impl SkillCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillCategory::Excel => "excel",
            SkillCategory::Word => "word",
            SkillCategory::Ppt => "ppt",
            SkillCategory::Multi => "multi",
        }
    }

    pub fn format(&self) -> &'static str {
        match self {
            SkillCategory::Excel => "xlsx",
            SkillCategory::Word => "docx",
            SkillCategory::Ppt => "pptx",
            SkillCategory::Multi => "json",
        }
    }
}

// ── Input / Output ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    pub name: String,
    #[serde(rename = "type")]
    pub input_type: String,
    pub description: Option<String>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub name: String,
    #[serde(rename = "type")]
    pub output_type: String,
    pub description: Option<String>,
}

// ── Placeholder ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPlaceholder {
    pub name: String,
    #[serde(rename = "type")]
    pub placeholder_type: String,
    pub description: Option<String>,
}

// ── Templates ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTemplates {
    pub primary: String,
    pub description: String,
}

// ── Formatting ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillFormatting {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margins: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub borders: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_width: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap_text: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading1: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading2: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading3: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_slide: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_slide: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_slide: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thank_you_slide: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light_theme: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dark_theme: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blockquote: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub even_row: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub odd_row: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chart: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bullet: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_items: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub totals: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_due: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pivot_header: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_info: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

// ── Validation ───────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillValidation {
    #[serde(default)]
    pub rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub name: String,
    #[serde(rename = "type")]
    pub rule_type: ValidationRuleType,
    pub field: String,
    pub message: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub values: Option<Vec<String>>,
    #[serde(default)]
    pub depends_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRuleType {
    #[serde(rename = "assert")]
    Assert,
    #[serde(rename = "regex")]
    Regex,
    #[serde(rename = "enum")]
    Enum,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub skill_name: String,
    pub passed: bool,
    pub rule_results: Vec<RuleResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
    pub rule_name: String,
    pub passed: bool,
    pub message: String,
}

// ── Main Definition ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub category: SkillCategory,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub inputs: Vec<SkillInput>,
    #[serde(default)]
    pub outputs: Vec<SkillOutput>,
    #[serde(default)]
    pub templates: Option<SkillTemplates>,
    #[serde(default)]
    pub placeholders: Vec<SkillPlaceholder>,
    #[serde(default)]
    pub formatting: SkillFormatting,
    #[serde(default)]
    pub validation: SkillValidation,
    // Additional fields for JSON serialization
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl SkillDefinition {
    pub fn id(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }

    pub fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: self.name.clone(),
            category: self.category.clone(),
            format: self.format.clone(),
            version: self.version.clone(),
            description: self.description.clone().unwrap_or_default(),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            formatting: self.formatting.clone(),
            validation: self.validation.clone(),
        }
    }
}

// ── Metadata (what gets returned from list_skills) ──────

#[derive(Debug, Clone, Serialize)]
pub struct SkillMetadata {
    pub name: String,
    pub category: SkillCategory,
    pub format: String,
    pub version: String,
    pub description: String,
    pub inputs: Vec<SkillInput>,
    pub outputs: Vec<SkillOutput>,
    pub formatting: SkillFormatting,
    pub validation: SkillValidation,
}

// ── Run Result ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRunResult {
    pub skill_name: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub file_paths: Vec<String>,
    pub warnings: Vec<String>,
}
