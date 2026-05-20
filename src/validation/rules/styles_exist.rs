use crate::validation::{ValidationCheck, ValidationRule};
use std::io::Read;

pub struct StylesExist;

#[async_trait::async_trait]
impl ValidationRule for StylesExist {
    fn name(&self) -> &'static str {
        "styles_exist"
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

        let styles_paths = match ext.as_str() {
            "docx" => Some(vec!["word/styles.xml"]),
            "pptx" => Some(vec!["ppt/styles.xml", "ppt/theme/theme1.xml"]),
            "xlsx" => Some(vec!["xl/styles.xml"]),
            _ => None,
        };

        let styles_paths = match styles_paths {
            Some(s) => s,
            None => {
                return Ok(ValidationCheck {
                    rule: self.name().to_string(),
                    passed: true,
                    details: Some(format!(
                        "Styles check not applicable for .{} format",
                        ext
                    )),
                    fix_tool: None,
                    fix_args: None,
                });
            }
        };

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

        let mut found_any = false;
        let mut missing_paths = Vec::new();

        for sp in &styles_paths {
            if archive.by_name(sp).is_ok() {
                found_any = true;
            } else {
                missing_paths.push(sp.to_string());
            }
        }

        if let Some(cfg) = config {
            if let Some(required_styles) = cfg.as_array() {
                let mut missing_styles = Vec::new();
                for entry in required_styles {
                    if let Some(style_id) = entry.as_str() {
                        let mut found_style = false;
                        for sp in &styles_paths {
                            if let Ok(mut entry) = archive.by_name(sp) {
                                let mut content = String::new();
                                if entry.read_to_string(&mut content).is_ok()
                                    && (content.contains(&format!("w:styleId=\"{}\"", style_id))
                                        || content.contains(&format!("w:val=\"{}\"", style_id)))
                                {
                                    found_style = true;
                                    break;
                                }
                            }
                        }
                        if !found_style {
                            missing_styles.push(style_id.to_string());
                        }
                    }
                }
                if missing_styles.is_empty() {
                    return Ok(ValidationCheck {
                        rule: self.name().to_string(),
                        passed: true,
                        details: Some("All required styles found".into()),
                        fix_tool: None,
                        fix_args: None,
                    });
                } else {
                    return Ok(ValidationCheck {
                        rule: self.name().to_string(),
                        passed: false,
                        details: Some(format!("Required styles not found: {}", missing_styles.join(", "))),
                        fix_tool: None,
                        fix_args: None,
                    });
                }
            }
        }

        if found_any || missing_paths.is_empty() {
            let details = if missing_paths.is_empty() {
                "Required styles files found".to_string()
            } else {
                format!(
                    "Some optional styles present, missing: {}",
                    missing_paths.join(", ")
                )
            };
            Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: true,
                details: Some(details),
                fix_tool: None,
                fix_args: None,
            })
        } else {
            Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: false,
                details: Some(format!(
                    "Required styles not found: {}",
                    missing_paths.join(", ")
                )),
                fix_tool: None,
                fix_args: None,
            })
        }
    }
}
