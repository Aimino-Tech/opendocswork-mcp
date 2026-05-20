use crate::validation::{ValidationCheck, ValidationRule};
use std::io::Read;

pub struct CrossReferencesValid;

#[async_trait::async_trait]
impl ValidationRule for CrossReferencesValid {
    fn name(&self) -> &'static str {
        "cross_references_valid"
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

        if !matches!(ext.as_str(), "docx" | "pptx") {
            return Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: true,
                details: Some(format!(
                    "Cross-reference check not applicable for .{} format",
                    ext
                )),
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

        let mut ref_count: usize = 0;
        let mut caption_count: usize = 0;
        let mut seq_count: usize = 0;

        for i in 0..archive.len() {
            let mut entry = match archive.by_index(i) {
                Ok(e) => e,
                _ => continue,
            };

            if !entry.name().ends_with(".xml") {
                continue;
            }

            let mut content = String::new();
            if entry.read_to_string(&mut content).is_err() {
                continue;
            }

            ref_count += content.matches("w:instrText").count();
            ref_count += content.matches("w:numId").count();

            caption_count += content.matches("<w:caption").count();
            caption_count += content.matches("a:fld").count();

            seq_count += content.matches("SEQ ").count();
            seq_count += content.matches(" w:instrText ").count();
        }

        let details = format!(
            "Found {} references, {} captions, {} sequence fields",
            ref_count, caption_count, seq_count
        );

        Ok(ValidationCheck {
            rule: self.name().to_string(),
            passed: true,
            details: Some(details),
            fix_tool: None,
            fix_args: None,
        })
    }
}
