use crate::validation::{ValidationCheck, ValidationRule};
use std::io::Read;

pub struct BrandColorsMatch;

#[async_trait::async_trait]
impl ValidationRule for BrandColorsMatch {
    fn name(&self) -> &'static str {
        "brand_colors_match"
    }

    async fn validate(&self, file_path: &str, config: Option<&serde_json::Value>) -> Result<ValidationCheck, anyhow::Error> {
        let path = std::path::Path::new(file_path);
        if !path.exists() {
            return Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: false,
                details: Some(format!("File not found: {}", file_path)),
                fix_tool: None,
                fix_args: None,
            });
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !matches!(ext.as_str(), "docx" | "pptx" | "xlsx") {
            return Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: true,
                details: Some("Brand color check skipped: not an OOXML format".into()),
                fix_tool: None,
                fix_args: None,
            });
        }

        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                return Ok(ValidationCheck {
                    rule: self.name().to_string(),
                    passed: false,
                    details: Some(format!("Cannot open file: {}", e)),
                    fix_tool: None,
                    fix_args: None,
                });
            }
        };

        let mut archive = match zip::ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                return Ok(ValidationCheck {
                    rule: self.name().to_string(),
                    passed: false,
                    details: Some(format!("Invalid OOXML archive: {}", e)),
                    fix_tool: None,
                    fix_args: None,
                });
            }
        };

        let theme_names = [
            "theme/theme1.xml",
            "ppt/theme/theme1.xml",
            "xl/theme/theme1.xml",
        ];

        let mut found_theme = false;
        let mut theme_content = String::new();

        for theme_path in &theme_names {
            if let Ok(mut entry) = archive.by_name(theme_path) {
                found_theme = true;
                if entry.read_to_string(&mut theme_content).is_ok() {
                    break;
                }
            }
        }

        if !found_theme {
            return Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: false,
                details: Some("No theme file found in OOXML archive".into()),
                fix_tool: None,
                fix_args: None,
            });
        }

        if let Some(cfg) = config {
            let mut mismatches = Vec::new();
            if let Some(expected_primary) = cfg.get("primary").and_then(|v| v.as_str()) {
                let normalized = expected_primary.trim_start_matches('#');
                if !theme_content.contains(normalized) {
                    mismatches.push(format!("primary color {} not found in theme", expected_primary));
                }
            }
            if let Some(expected_accent) = cfg.get("accent").and_then(|v| v.as_str()) {
                let normalized = expected_accent.trim_start_matches('#');
                if !theme_content.contains(normalized) {
                    mismatches.push(format!("accent color {} not found in theme", expected_accent));
                }
            }

            if mismatches.is_empty() {
                return Ok(ValidationCheck {
                    rule: self.name().to_string(),
                    passed: true,
                    details: Some("Theme colors match expected values".into()),
                    fix_tool: None,
                    fix_args: None,
                });
            } else {
                return Ok(ValidationCheck {
                    rule: self.name().to_string(),
                    passed: false,
                    details: Some(mismatches.join("; ")),
                    fix_tool: None,
                    fix_args: None,
                });
            }
        }

        let has_valid_elements = theme_content.contains("<a:theme")
            || theme_content.contains("<a:themeElement");
        if has_valid_elements {
            Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: true,
                details: Some("Theme file found with valid theme elements".into()),
                fix_tool: None,
                fix_args: None,
            })
        } else {
            Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: false,
                details: Some("Theme file found but no valid theme elements".into()),
                fix_tool: None,
                fix_args: None,
            })
        }
    }
}
