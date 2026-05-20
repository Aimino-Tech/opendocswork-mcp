use crate::validation::{ValidationCheck, ValidationRule};
use std::io::Read;

pub struct MaxPages;

#[async_trait::async_trait]
impl ValidationRule for MaxPages {
    fn name(&self) -> &'static str {
        "max_pages"
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

        let custom_limit = config.and_then(|c| c.as_u64());

        let (page_count, default_max) = match ext.as_str() {
            "docx" => (count_docx_pages(path)?, 10),
            "pptx" => (count_pptx_slides(path)?, 15),
            "xlsx" => (count_xlsx_sheets(path)? as u64, 5),
            _ => {
                return Ok(ValidationCheck {
                    rule: self.name().to_string(),
                    passed: true,
                    details: Some(format!(
                        "Page count check not applicable for .{} format",
                        ext
                    )),
                    fix_tool: None,
                    fix_args: None,
                });
            }
        };

        let max_allowed = custom_limit.unwrap_or(default_max);

        if page_count <= max_allowed {
            Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: true,
                details: Some(format!(
                    "{} page(s) within limit of {}",
                    page_count, max_allowed
                )),
                fix_tool: None,
                fix_args: None,
            })
        } else {
            Ok(ValidationCheck {
                rule: self.name().to_string(),
                passed: false,
                details: Some(format!(
                    "{} page(s) exceeds max allowed ({})",
                    page_count, max_allowed
                )),
                fix_tool: None,
                fix_args: None,
            })
        }
    }
}

fn count_docx_pages(path: &std::path::Path) -> Result<u64, anyhow::Error> {
    if let Ok(file) = std::fs::File::open(path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            let mut total = 0u64;
            for i in 0..archive.len() {
                if let Ok(mut entry) = archive.by_index(i) {
                    let name = entry.name().to_string();
                    if name.starts_with("word/") && name.ends_with(".xml") {
                        let mut content = String::new();
                        if entry.read_to_string(&mut content).is_ok() {
                            total += content.matches("<w:sectPr").count() as u64;
                            total +=
                                content.matches("<w:br w:type=\"page\"").count() as u64;
                        }
                    }
                }
            }
            return Ok(std::cmp::max(total, 1));
        }
    }
    Ok(1)
}

fn count_pptx_slides(path: &std::path::Path) -> Result<u64, anyhow::Error> {
    if let Ok(file) = std::fs::File::open(path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            let count = (0..archive.len())
                .filter(|i| {
                    archive
                        .by_index(*i)
                        .map(|e| {
                            e.name().starts_with("ppt/slides/slide")
                                && e.name().ends_with(".xml")
                        })
                        .unwrap_or(false)
                })
                .count() as u64;
            return Ok(std::cmp::max(count, 1));
        }
    }
    Ok(1)
}

fn count_xlsx_sheets(path: &std::path::Path) -> Result<usize, anyhow::Error> {
    if let Ok(file) = std::fs::File::open(path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            let count = (0..archive.len())
                .filter(|i| {
                    archive
                        .by_index(*i)
                        .map(|e| {
                            e.name().starts_with("xl/worksheets/sheet")
                                && e.name().ends_with(".xml")
                        })
                        .unwrap_or(false)
                })
                .count();
            return Ok(std::cmp::max(count, 1));
        }
    }
    Ok(1)
}
