use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: SkillCategory,
    pub format: String,
    pub author: String,
    #[serde(default)]
    pub icon: String,
    pub inputs: Vec<SkillInput>,
    #[serde(default)]
    pub outputs: Vec<SkillOutput>,
    pub templates: SkillTemplates,
    #[serde(default)]
    pub placeholders: Vec<SkillPlaceholder>,
    #[serde(default)]
    pub formatting: SkillFormatting,
    pub validation: SkillValidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillCategory {
    #[serde(rename = "excel")]
    Excel,
    #[serde(rename = "word")]
    Word,
    #[serde(rename = "ppt")]
    Ppt,
}

impl SkillCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillCategory::Excel => "excel",
            SkillCategory::Word => "word",
            SkillCategory::Ppt => "ppt",
        }
    }
    #[allow(dead_code)]
    pub fn format(&self) -> &'static str {
        match self {
            SkillCategory::Excel => "xlsx",
            SkillCategory::Word => "docx",
            SkillCategory::Ppt => "pptx",
        }
    }

    #[allow(dead_code)]
    pub fn icon(&self) -> &'static str {
        match self {
            SkillCategory::Excel => "📊",
            SkillCategory::Word => "📝",
            SkillCategory::Ppt => "📽️",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    pub name: String,
    #[serde(rename = "type")]
    pub input_type: SkillInputType,
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillInputType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
}

impl SkillInputType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillInputType::String => "string",
            SkillInputType::Number => "number",
            SkillInputType::Integer => "integer",
            SkillInputType::Boolean => "boolean",
            SkillInputType::Array => "array",
            SkillInputType::Object => "object",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    pub name: String,
    #[serde(rename = "type")]
    pub output_type: SkillOutputType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillOutputType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
}

impl SkillOutputType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillOutputType::String => "string",
            SkillOutputType::Integer => "integer",
            SkillOutputType::Number => "number",
            SkillOutputType::Array => "array",
            SkillOutputType::Object => "object",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTemplates {
    pub primary: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPlaceholder {
    pub name: String,
    #[serde(rename = "type")]
    pub placeholder_type: SkillPlaceholderType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillPlaceholderType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "table")]
    Table,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "markup")]
    Markup,
    #[serde(rename = "style")]
    Style,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillFormatting {
    #[serde(default)]
    pub font: FontSettings,
    #[serde(default)]
    pub header: HeaderSettings,
    #[serde(default)]
    pub body: BodySettings,
    #[serde(default)]
    pub margins: MarginSettings,
    #[serde(default)]
    pub borders: BorderSettings,
    #[serde(default)]
    pub column_width: String,
    #[serde(default)]
    pub wrap_text: bool,
    #[serde(default)]
    pub heading1: Option<HeadingSettings>,
    #[serde(default)]
    pub heading2: Option<HeadingSettings>,
    #[serde(default)]
    pub heading3: Option<HeadingSettings>,
    #[serde(default)]
    pub cover: Option<CoverSettings>,
    #[serde(default)]
    pub title_slide: Option<SlideThemeSettings>,
    #[serde(default)]
    pub content_slide: Option<SlideThemeSettings>,
    #[serde(default)]
    pub section_slide: Option<SlideThemeSettings>,
    #[serde(default)]
    pub thank_you_slide: Option<SlideThemeSettings>,
    #[serde(default)]
    pub light_theme: Option<ThemeColors>,
    #[serde(default)]
    pub dark_theme: Option<ThemeColors>,
    #[serde(default)]
    pub code: Option<CodeSettings>,
    #[serde(default)]
    pub blockquote: Option<BlockquoteSettings>,
    #[serde(default)]
    pub list: Option<ListSettings>,
    #[serde(default)]
    pub line_items: Option<LineItemsSettings>,
    #[serde(default)]
    pub totals: Option<TotalsSettings>,
    #[serde(default)]
    pub total_due: Option<TotalDueSettings>,
    #[serde(default)]
    pub even_row: Option<RowSettings>,
    #[serde(default)]
    pub odd_row: Option<RowSettings>,
    #[serde(default)]
    pub pivot_header: Option<PivotHeaderSettings>,
    #[serde(default)]
    pub chart: Option<ChartSettings>,
    #[serde(default)]
    pub bullet: Option<BulletSettings>,
    #[serde(default)]
    pub company_info: Option<CompanyInfoSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FontSettings {
    #[serde(default = "default_font_family")]
    pub family: String,
    #[serde(default = "default_font_size")]
    pub size: u32,
}

fn default_font_family() -> String { "Calibri".to_string() }
fn default_font_size() -> u32 { 11 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeaderSettings {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub fill: String,
    #[serde(default)]
    pub font_color: String,
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BodySettings {
    #[serde(default)]
    pub font_size: u32,
    #[serde(default)]
    pub line_spacing: f64,
    #[serde(default)]
    pub space_after: u32,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarginSettings {
    #[serde(default)]
    pub top: u32,
    #[serde(default)]
    pub bottom: u32,
    #[serde(default)]
    pub left: u32,
    #[serde(default)]
    pub right: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BorderSettings {
    #[serde(default)]
    pub style: String,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingSettings {
    pub font_size: u32,
    pub bold: bool,
    pub color: String,
    #[serde(default)]
    pub space_before: u32,
    #[serde(default)]
    pub space_after: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverSettings {
    pub title_font_size: u32,
    pub title_color: String,
    pub subtitle_font_size: u32,
    pub subtitle_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideThemeSettings {
    pub title_font_size: u32,
    pub title_color: String,
    pub background: String,
    #[serde(default)]
    pub subtitle_font_size: Option<u32>,
    #[serde(default)]
    pub subtitle_color: Option<String>,
    #[serde(default)]
    pub body_font_size: Option<u32>,
    #[serde(default)]
    pub body_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub background: String,
    pub title_color: String,
    pub body_color: String,
    pub accent_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSettings {
    pub font_family: String,
    pub font_size: u32,
    pub fill: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockquoteSettings {
    pub italic: bool,
    pub color: String,
    pub left_indent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSettings {
    pub left_indent: u32,
    pub hanging_indent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItemsSettings {
    pub header_fill: String,
    pub header_bold: bool,
    pub even_fill: String,
    pub odd_fill: String,
    pub borders: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalsSettings {
    pub font_size: u32,
    pub bold: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalDueSettings {
    pub font_size: u32,
    pub bold: bool,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowSettings {
    pub fill: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotHeaderSettings {
    pub bold: bool,
    pub fill: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSettings {
    pub width: u32,
    pub height: u32,
    pub style: u32,
    #[serde(default)]
    pub colors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletSettings {
    pub font_size: u32,
    #[serde(default)]
    pub indent_levels: Vec<IndentLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndentLevel {
    pub left_margin: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyInfoSettings {
    pub font_size: u32,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillValidation {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SkillRunRequest {
    pub skill_name: String,
    pub params: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRunResult {
    pub skill_name: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub file_paths: Vec<String>,
    pub warnings: Vec<String>,
}
