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

    /// Create a minimal but valid DOCX file with placeholder text.
    fn create_test_template(path: &str, body_text: &str) {
        let tmp_path = format!("{}.tmp", path);
        let _ = std::fs::remove_file(&tmp_path);

        let file = std::fs::File::create(&tmp_path).unwrap();
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
        std::fs::rename(&tmp_path, path).unwrap();
    }

    #[test]
    fn test_detect_simple_placeholder() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_detect_simple.docx");
        let path_str = path.to_string_lossy().to_string();

        create_test_template(&path_str, "Hello {name}, your order {order_id} is ready.");

        let result = TemplateEngine::detect_placeholders(&path_str).unwrap();
        let placeholders: Vec<String> = serde_json::from_str(&result).unwrap();

        assert!(placeholders.contains(&"name".to_string()));
        assert!(placeholders.contains(&"order_id".to_string()));
        assert_eq!(placeholders.len(), 2);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_detect_double_curly() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_detect_double.docx");
        let path_str = path.to_string_lossy().to_string();

        create_test_template(&path_str, "Hello {{name}}, your {{order_id}}.");

        let result = TemplateEngine::detect_placeholders(&path_str).unwrap();
        let placeholders: Vec<String> = serde_json::from_str(&result).unwrap();

        assert!(placeholders.contains(&"name".to_string()));
        assert!(placeholders.contains(&"order_id".to_string()));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_fill_simple() {
        let dir = std::env::temp_dir();
        let template_path = dir.join("test_fill_simple.docx");
        let output_path = dir.join("test_fill_simple_output.docx");
        let tpl_str = template_path.to_string_lossy().to_string();
        let out_str = output_path.to_string_lossy().to_string();

        create_test_template(&tpl_str, "Hello {name}, date: {date}.");

        let mut data = HashMap::new();
        data.insert("name".to_string(), "Alice".to_string());
        data.insert("date".to_string(), "2024-01-15".to_string());

        let result = TemplateEngine::fill_template(&tpl_str, &out_str, &data).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["status"], "created");
        assert!(json["fields_filled"].as_i64().unwrap_or(0) > 0);

        assert!(output_path.exists());

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

        std::fs::remove_file(&tpl_str).ok();
        std::fs::remove_file(&out_str).ok();
    }

    #[test]
    fn test_fill_double_curly() {
        let dir = std::env::temp_dir();
        let template_path = dir.join("test_fill_double.docx");
        let output_path = dir.join("test_fill_double_output.docx");
        let tpl_str = template_path.to_string_lossy().to_string();
        let out_str = output_path.to_string_lossy().to_string();

        create_test_template(&tpl_str, "Hello {{name}}, your {{item}}.");

        let mut data = HashMap::new();
        data.insert("name".to_string(), "Bob".to_string());
        data.insert("item".to_string(), "Widget".to_string());

        let result = TemplateEngine::fill_template(&tpl_str, &out_str, &data).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["status"], "created");
        assert!(output_path.exists());

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

        std::fs::remove_file(&tpl_str).ok();
        std::fs::remove_file(&out_str).ok();
    }

    #[test]
    fn test_fill_missing_key() {
        let dir = std::env::temp_dir();
        let template_path = dir.join("test_fill_missing.docx");
        let output_path = dir.join("test_fill_missing_output.docx");
        let tpl_str = template_path.to_string_lossy().to_string();
        let out_str = output_path.to_string_lossy().to_string();

        create_test_template(&tpl_str, "Hello {name} and {missing}.");

        let mut data = HashMap::new();
        data.insert("name".to_string(), "Carol".to_string());

        let result = TemplateEngine::fill_template(&tpl_str, &out_str, &data).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(json["status"], "created");
        assert!(output_path.exists());

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

        std::fs::remove_file(&tpl_str).ok();
        std::fs::remove_file(&out_str).ok();
    }

    #[test]
    fn test_batch_fill() {
        let dir = std::env::temp_dir();
        let template_path = dir.join("test_batch.docx");
        let output_dir = dir.join("test_batch_output");
        let tpl_str = template_path.to_string_lossy().to_string();
        let out_dir_str = output_dir.to_string_lossy().to_string();

        create_test_template(&tpl_str, "Hello {name}, your {item}.");

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
            let out_path = output_dir.join(format!("test_batch_record_{}.docx", i));
            assert!(out_path.exists(), "Output {} should exist", i);
            std::fs::remove_file(&out_path).ok();
        }

        std::fs::remove_file(&tpl_str).ok();
        std::fs::remove_dir(&out_dir_str).ok();
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
    fn test_error_bad_extension() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_bad_ext.txt");
        std::fs::write(&path, b"not a docx").unwrap();
        let path_str = path.to_string_lossy().to_string();

        let result =
            TemplateEngine::fill_template(&path_str, "/tmp/out.docx", &HashMap::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported format"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_detect_no_placeholders() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_no_placeholders.docx");
        let path_str = path.to_string_lossy().to_string();

        create_test_template(&path_str, "Just plain text without any placeholders.");

        let result = TemplateEngine::detect_placeholders(&path_str).unwrap();
        let placeholders: Vec<String> = serde_json::from_str(&result).unwrap();
        assert!(placeholders.is_empty());

        std::fs::remove_file(&path).ok();
    }
}
