use crate::validation::{ValidationCheck, ValidationRule};
use std::io::Read;

pub struct OOXMLValid;

const REQUIRED_PARTS: &[&str] = &["[Content_Types].xml"];

const DOCX_REQUIRED: &[&str] = &["word/document.xml"];
const PPTX_REQUIRED: &[&str] = &["ppt/presentation.xml"];
const XLSX_REQUIRED: &[&str] = &["xl/workbook.xml"];

#[async_trait::async_trait]
impl ValidationRule for OOXMLValid {
    fn name(&self) -> &'static str {
        "ooxml_valid"
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

        if !is_office_format(&ext) {
            return Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: true,
                details: Some(format!(
                    "OOXML check skipped for non-OOXML format: .{}",
                    ext
                )),
                fix_tool: None,
                fix_args: None,
            });
        }

        if !check_zip_magic(path) {
            return Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: false,
                details: Some("Not a valid ZIP archive (bad magic bytes)".into()),
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
                    details: Some(format!("Invalid ZIP archive: {}", e)),
                    fix_tool: None,
                    fix_args: None,
                });
            }
        };

        let mut missing_parts: Vec<String> = Vec::new();
        let mut malformed_xml: Vec<String> = Vec::new();
        let mut empty_entries: Vec<String> = Vec::new();

        for required in REQUIRED_PARTS {
            if archive.by_name(required).is_err() {
                missing_parts.push(required.to_string());
            }
        }

        let format_required: &[&str] = match ext.as_str() {
            "docx" | "doc" => DOCX_REQUIRED,
            "pptx" | "ppt" => PPTX_REQUIRED,
            "xlsx" | "xls" => XLSX_REQUIRED,
            _ => &[],
        };

        for required in format_required {
            if archive.by_name(required).is_err() {
                missing_parts.push(required.to_string());
            }
        }

        for i in 0..archive.len() {
            let mut entry = match archive.by_index(i) {
                Ok(e) => e,
                _ => continue,
            };

            if entry.size() == 0 && entry.name().ends_with(".xml") {
                empty_entries.push(entry.name().to_string());
                continue;
            }

            if entry.name().ends_with(".xml") {
                let mut content = Vec::new();
                if entry.read_to_end(&mut content).is_ok() {
                    if let Ok(text) = String::from_utf8(content) {
                        let trimmed = text.trim_start();
                        if !trimmed.starts_with("<?xml") && !trimmed.starts_with('<') {
                            malformed_xml.push(entry.name().to_string());
                        }
                    } else {
                        malformed_xml.push(entry.name().to_string());
                    }
                }
            }
        }

        let mut details_parts = Vec::new();

        if !missing_parts.is_empty() {
            details_parts.push(format!("Missing parts: {}", missing_parts.join(", ")));
        }
        if !malformed_xml.is_empty() {
            details_parts.push(format!(
                "Malformed XML: {}",
                malformed_xml.join(", ")
            ));
        }
        if !empty_entries.is_empty() {
            details_parts.push(format!("Empty entries: {}", empty_entries.join(", ")));
        }

        let total_entries = archive.len();

        if missing_parts.is_empty() && malformed_xml.is_empty() && empty_entries.is_empty() {
            Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: true,
                details: Some(format!(
                    "Valid OOXML archive with {} entries",
                    total_entries
                )),
                fix_tool: None,
                fix_args: None,
            })
        } else {
            Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: false,
                details: Some(details_parts.join("; ")),
                fix_tool: None,
                fix_args: None,
            })
        }
    }
}

fn check_zip_magic(path: &std::path::Path) -> bool {
    use std::io::Read;
    if let Ok(mut f) = std::fs::File::open(path) {
        let mut magic = [0u8; 4];
        if f.read_exact(&mut magic).is_ok() {
            return magic.starts_with(&[0x50, 0x4B, 0x03, 0x04])
                || magic.starts_with(&[0x50, 0x4B, 0x05, 0x06])
                || magic.starts_with(&[0x50, 0x4B, 0x07, 0x08]);
        }
    }
    false
}

fn is_office_format(ext: &str) -> bool {
    matches!(ext, "docx" | "xlsx" | "pptx" | "doc" | "xls" | "ppt")
}
