use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::time::Instant;

use rayon::prelude::*;
use regex::Regex;

/// Engine for DOCX template placeholder replacement (mail merge) and batch processing.
pub struct TemplateEngine;

impl TemplateEngine {
    pub fn new() -> Self {
        Self
    }

    /// Fill a DOCX template with data and save to output_path.
    ///
    /// Placeholders can be in `{name}` or `{{name}}` format.
    /// The `data` map keys should correspond to placeholder names (without braces).
    ///
    /// Returns JSON with status, output_path, fields_filled count, and list of placeholders.
    pub fn fill_template(
        input_path: &str,
        output_path: &str,
        data: &HashMap<String, String>,
    ) -> Result<String, String> {
        let path = Path::new(input_path);
        if !path.exists() {
            return Err(format!("Template file not found: {}", input_path));
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "docx" {
            return Err(format!(
                "Unsupported format: '{}'. Only .docx files are supported.",
                ext
            ));
        }

        let all_placeholders = Self::do_detect_placeholders(input_path)?;

        let mut doc = rdocx::Document::open(input_path)
            .map_err(|e| format!("Failed to open DOCX: {}", e))?;

        // Replace both {key} and {{key}} formats since users may use either
        let mut total_replacements = 0;

        for (key, value) in data {
            let single_braced = format!("{{{}}}", key);
            total_replacements += doc.replace_text(&single_braced, value);

            let double_braced = format!("{{{{{}}}}}", key);
            total_replacements += doc.replace_text(&double_braced, value);
        }

        doc.save(output_path)
            .map_err(|e| format!("Failed to save filled DOCX: {}", e))?;

        // Report placeholders that were actually found in the template
        let placeholders_in_template: Vec<String> = all_placeholders
            .iter()
            .filter(|p| data.contains_key(*p))
            .cloned()
            .collect();

        let result = serde_json::json!({
            "status": "created",
            "output_path": output_path,
            "fields_filled": total_replacements,
            "placeholders": placeholders_in_template,
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| format!("Failed to serialize result: {}", e))
    }

    /// Batch process a DOCX template with multiple data records.
    ///
    /// Processes records in parallel using rayon. Each record generates one output file
    /// named like `{output_dir}/{template_name}_record_{N}.docx`.
    ///
    /// Returns JSON summary with total, outputs, and duration_ms.
    pub fn batch_fill(
        input_path: &str,
        output_dir: &str,
        records: &[HashMap<String, String>],
    ) -> Result<String, String> {
        let start = Instant::now();

        let path = Path::new(input_path);
        if !path.exists() {
            return Err(format!("Template file not found: {}", input_path));
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "docx" {
            return Err(format!(
                "Unsupported format: '{}'. Only .docx files are supported.",
                ext
            ));
        }

        std::fs::create_dir_all(output_dir)
            .map_err(|e| format!("Failed to create output directory '{}': {}", output_dir, e))?;

        let template_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");

        // Read the template bytes once so we can copy for each record
        let template_bytes =
            std::fs::read(input_path).map_err(|e| format!("Failed to read template: {}", e))?;

        // Process each record in parallel
        let outputs: Vec<Result<(usize, String), String>> = records
            .par_iter()
            .enumerate()
            .map(|(idx, record)| {
                let filename = format!("{}_record_{}.docx", template_stem, idx + 1);
                let out_path = Path::new(output_dir).join(&filename);
                let out_str = out_path.to_string_lossy().to_string();

                std::fs::write(&out_str, &template_bytes)
                    .map_err(|e| format!("Failed to write output file: {}", e))?;

                let mut doc = rdocx::Document::open(&out_str)
                    .map_err(|e| format!("Failed to open output DOCX: {}", e))?;

                for (key, value) in record {
                    let single_braced = format!("{{{}}}", key);
                    doc.replace_text(&single_braced, value);
                    let double_braced = format!("{{{{{}}}}}", key);
                    doc.replace_text(&double_braced, value);
                }

                doc.save(&out_str)
                    .map_err(|e| format!("Failed to save output DOCX: {}", e))?;

                Ok((idx, out_str))
            })
            .collect();

        let duration_ms = start.elapsed().as_millis() as u64;

        let mut successful_outputs: Vec<String> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        for result in outputs {
            match result {
                Ok((_, path)) => successful_outputs.push(path),
                Err(e) => errors.push(e),
            }
        }

        let result = serde_json::json!({
            "total": records.len(),
            "successful": successful_outputs.len(),
            "failed": errors.len(),
            "outputs": successful_outputs,
            "errors": errors,
            "duration_ms": duration_ms,
        });

        serde_json::to_string_pretty(&result)
            .map_err(|e| format!("Failed to serialize result: {}", e))
    }

    /// Detect all unique placeholders in a DOCX template.
    ///
    /// Searches for patterns like `{name}` and `{{name}}` in all XML entries
    /// of the DOCX archive (document body, headers, footers, etc.).
    ///
    /// Returns JSON array of unique placeholder names (without braces).
    pub fn detect_placeholders(input_path: &str) -> Result<String, String> {
        let placeholders = Self::do_detect_placeholders(input_path)?;

        serde_json::to_string_pretty(&placeholders)
            .map_err(|e| format!("Failed to serialize placeholders: {}", e))
    }

    // ── Internal implementation ──

    fn do_detect_placeholders(input_path: &str) -> Result<Vec<String>, String> {
        let path = Path::new(input_path);
        if !path.exists() {
            return Err(format!("File not found: {}", input_path));
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "docx" {
            return Err(format!(
                "Unsupported format: '{}'. Only .docx files are supported.",
                ext
            ));
        }

        let file =
            std::fs::File::open(input_path).map_err(|e| format!("Failed to open file: {}", e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Failed to open DOCX as ZIP: {}", e))?;

        // Match both {name} and {{name}} — capture the inner name
        let re = Regex::new(r"\{\{?([^}]+)\}?\}")
            .map_err(|e| format!("Regex error: {}", e))?;
        let mut unique_placeholders: HashSet<String> = HashSet::new();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name().to_string();

            if !(name.ends_with(".xml") || name.ends_with(".rels") || name == "[Content_Types].xml")
            {
                continue;
            }

            let mut data = Vec::new();
            entry.read_to_end(&mut data).map_err(|e| e.to_string())?;

            if let Ok(text) = String::from_utf8(data) {
                for cap in re.captures_iter(&text) {
                    if let Some(matched) = cap.get(1) {
                        let placeholder_name = matched.as_str().trim().to_string();
                        if !placeholder_name.is_empty() {
                            unique_placeholders.insert(placeholder_name);
                        }
                    }
                }
            }
        }

        let mut result: Vec<String> = unique_placeholders.into_iter().collect();
        result.sort();
        Ok(result)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use tempfile::TempDir;

    /// Create a minimal but valid DOCX file with placeholder text in a temp dir.
    fn create_test_template_in(dir: &TempDir, name: &str, body_text: &str) -> String {
        let path = dir.path().join(name);
        let path_str = path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path_str);

        let file = std::fs::File::create(&path_str).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;
        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(content_types.as_bytes()).unwrap();

        let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;
        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(rels.as_bytes()).unwrap();

        let word_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
</Relationships>"#;
        zip.start_file("word/_rels/document.xml.rels", options)
            .unwrap();
        zip.write_all(word_rels.as_bytes()).unwrap();

        let document_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>{body_text}</w:t></w:r></w:p>
  </w:body>
</w:document>"#,
        );
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();

        zip.finish().unwrap();
        path_str
    }

    #[test]
    fn test_detect_simple_placeholder() {
        let dir = TempDir::new().unwrap();
        let path_str = create_test_template_in(&dir, "test.docx", "Hello {name}, your order {order_id} is ready.");

        let result = TemplateEngine::detect_placeholders(&path_str).unwrap();
        let placeholders: Vec<String> = serde_json::from_str(&result).unwrap();

        assert!(placeholders.contains(&"name".to_string()));
        assert!(placeholders.contains(&"order_id".to_string()));
        assert_eq!(placeholders.len(), 2);
    }

    #[test]
    fn test_detect_double_curly() {
        let dir = TempDir::new().unwrap();
        let path_str = create_test_template_in(&dir, "test.docx", "Hello {{name}}, your {{order_id}}.");

        let result = TemplateEngine::detect_placeholders(&path_str).unwrap();
        let placeholders: Vec<String> = serde_json::from_str(&result).unwrap();

        assert!(placeholders.contains(&"name".to_string()));
        assert!(placeholders.contains(&"order_id".to_string()));
    }

    #[test]
    fn test_fill_simple() {
        let dir = TempDir::new().unwrap();
        let tpl_str = create_test_template_in(&dir, "template.docx", "Hello {name}, date: {date}.");
        let out_str = dir.path().join("output.docx").to_string_lossy().to_string();

        let mut data = HashMap::new();
        data.insert("name".to_string(), "Alice".to_string());
        data.insert("date".to_string(), "2024-01-15".to_string());

        let result = TemplateEngine::fill_template(&tpl_str, &out_str, &data).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["status"], "created");
        assert!(json["fields_filled"].as_i64().unwrap_or(0) > 0);
        assert!(std::path::Path::new(&out_str).exists());

        // Verify replacement by reading the DOCX as ZIP
        let file = std::fs::File::open(&out_str).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut doc_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut doc_xml)
            .unwrap();
        assert!(doc_xml.contains("Alice"));
        assert!(doc_xml.contains("2024-01-15"));
        assert!(!doc_xml.contains("{name}"));
    }

    #[test]
    fn test_fill_double_curly() {
        let dir = TempDir::new().unwrap();
        let tpl_str = create_test_template_in(&dir, "template.docx", "Hello {{name}}, your {{item}}.");
        let out_str = dir.path().join("output.docx").to_string_lossy().to_string();

        let mut data = HashMap::new();
        data.insert("name".to_string(), "Bob".to_string());
        data.insert("item".to_string(), "Widget".to_string());

        let result = TemplateEngine::fill_template(&tpl_str, &out_str, &data).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["status"], "created");
        assert!(std::path::Path::new(&out_str).exists());

        let file = std::fs::File::open(&out_str).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut doc_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut doc_xml)
            .unwrap();
        assert!(doc_xml.contains("Bob"));
        assert!(doc_xml.contains("Widget"));
        assert!(!doc_xml.contains("{{name}}"));
    }

    #[test]
    fn test_fill_missing_key() {
        let dir = TempDir::new().unwrap();
        let tpl_str = create_test_template_in(&dir, "template.docx", "Hello {name} and {missing}.");
        let out_str = dir.path().join("output.docx").to_string_lossy().to_string();

        let mut data = HashMap::new();
        data.insert("name".to_string(), "Carol".to_string());

        let result = TemplateEngine::fill_template(&tpl_str, &out_str, &data).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["status"], "created");
        assert!(std::path::Path::new(&out_str).exists());

        let file = std::fs::File::open(&out_str).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut doc_xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut doc_xml)
            .unwrap();
        assert!(doc_xml.contains("Carol"));
        assert!(doc_xml.contains("{missing}"));
    }

    #[test]
    fn test_batch_fill() {
        let dir = TempDir::new().unwrap();
        let tpl_str = create_test_template_in(&dir, "template.docx", "Hello {name}, your {item}.");
        let out_dir = dir.path().join("output");
        let out_dir_str = out_dir.to_string_lossy().to_string();

        let records = vec![
            {
                let mut m = HashMap::new();
                m.insert("name".to_string(), "Alice".to_string());
                m.insert("item".to_string(), "Widget".to_string());
                m
            },
            {
                let mut m = HashMap::new();
                m.insert("name".to_string(), "Bob".to_string());
                m.insert("item".to_string(), "Gadget".to_string());
                m
            },
            {
                let mut m = HashMap::new();
                m.insert("name".to_string(), "Carol".to_string());
                m.insert("item".to_string(), "Service".to_string());
                m
            },
        ];

        let result = TemplateEngine::batch_fill(&tpl_str, &out_dir_str, &records).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["total"], 3);
        assert_eq!(json["successful"], 3);
        assert_eq!(json["failed"], 0);

        for i in 1..=3 {
            let out_path = out_dir.join(format!("template_record_{}.docx", i));
            assert!(out_path.exists(), "Output {} should exist", i);
        }
    }

    #[test]
    fn test_error_missing_file() {
        let result = TemplateEngine::fill_template(
            "/nonexistent/template.docx",
            "/tmp/out.docx",
            &HashMap::new(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
}

    #[test]
    fn test_detect_no_placeholders() {
        let dir = TempDir::new().unwrap();
        let path_str = create_test_template_in(&dir, "test.docx", "Just plain text without any placeholders.");

        let result = TemplateEngine::detect_placeholders(&path_str).unwrap();
        let placeholders: Vec<String> = serde_json::from_str(&result).unwrap();
        assert!(placeholders.is_empty());
    }
}
