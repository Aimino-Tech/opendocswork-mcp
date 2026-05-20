use crate::validation::{ValidationCheck, ValidationRule};
use regex::Regex;
use std::io::Read;

pub struct NoEmptyPlaceholders;

#[async_trait::async_trait]
impl ValidationRule for NoEmptyPlaceholders {
    fn name(&self) -> &'static str {
        "no_empty_placeholders"
    }

    async fn validate(&self, file_path: &str, _config: Option<&serde_json::Value>) -> Result<ValidationCheck, anyhow::Error> {
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

        let re = Regex::new(r"\{[^}]*\}")?;
        let mut placeholders: Vec<String> = Vec::new();

        match ext.as_str() {
            "docx" | "pptx" | "xlsx" | "doc" | "ppt" | "xls" => {
                if let Ok(file) = std::fs::File::open(path) {
                    if let Ok(mut archive) = zip::ZipArchive::new(file) {
                        for i in 0..archive.len() {
                            if let Ok(mut entry) = archive.by_index(i) {
                                if entry.name().ends_with(".xml") || entry.name().ends_with(".rels")
                                {
                                    let mut content = String::new();
                                    if entry.read_to_string(&mut content).is_ok() {
                                        for cap in re.captures_iter(&content) {
                                            let ph = cap[0].to_string();
                                            if !placeholders.contains(&ph) {
                                                placeholders.push(ph);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                if let Ok(content) = std::fs::read_to_string(path) {
                    for cap in re.captures_iter(&content) {
                        let ph = cap[0].to_string();
                        if !placeholders.contains(&ph) {
                            placeholders.push(ph);
                        }
                    }
                }
            }
        }

        if placeholders.is_empty() {
            Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: true,
                details: None,
                fix_tool: None,
                fix_args: None,
            })
        } else {
            let details = format!(
                "Found {} placeholder(s): {}",
                placeholders.len(),
                placeholders.join(", ")
            );
            Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: false,
                details: Some(details),
                fix_tool: Some("office_replace_text".into()),
                fix_args: Some(serde_json::json!({
                    "file_path": file_path,
                    "placeholders": placeholders,
                })),
            })
        }
    }
}
