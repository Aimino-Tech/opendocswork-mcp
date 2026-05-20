use similar::{ChangeTag, TextDiff};
use serde::Serialize;
use std::io::Read;

// ── Data structures ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct DiffChange {
    pub change_type: String,
    pub content: String,
    pub line_number_a: Option<i32>,
    pub line_number_b: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffStats {
    pub additions: i32,
    pub deletions: i32,
    pub unchanged_lines: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffResult {
    pub changeset: Vec<DiffChange>,
    pub stats: DiffStats,
    pub similarity_score: f64,
}

// ── DocumentDiff ────────────────────────────────────────────────

#[derive(Debug)]
pub struct DocumentDiff;

impl DocumentDiff {
    pub fn new() -> Self {
        Self
    }

    /// Compare two Office documents and return a structured JSON diff.
    /// Validates files exist, have matching extensions, extracts text
    /// from both, and performs a line-by-line diff using `similar::TextDiff`.
    pub fn diff_documents(path_a: &str, path_b: &str) -> Result<String, String> {
        Self::validate_paths(path_a, path_b)?;

        let text_a = Self::extract_text(path_a)?;
        let text_b = Self::extract_text(path_b)?;

        let result = Self::text_diff(&text_a, &text_b);
        serde_json::to_string_pretty(&result)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    /// Same as `diff_documents` but adds per-paragraph/slide/sheet granularity.
    /// For DOCX: paragraphs are separated cleanly.
    /// For XLSX: cells are tab-separated within rows, rows newline-separated.
    /// For PPTX: slides are separated by markers.
    pub fn diff_documents_detailed(path_a: &str, path_b: &str) -> Result<String, String> {
        Self::validate_paths(path_a, path_b)?;

        let ext = Self::get_extension(path_a)?;
        let text_a = Self::extract_text_detailed(path_a, &ext)?;
        let text_b = Self::extract_text_detailed(path_b, &ext)?;

        let result = Self::text_diff(&text_a, &text_b);
        serde_json::to_string_pretty(&result)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    /// Compute a structured diff between two text strings using `similar::TextDiff`.
    /// Returns a serde_json::Value with changeset, stats, and similarity_score.
    pub fn text_diff(text_a: &str, text_b: &str) -> serde_json::Value {
        let diff = TextDiff::from_lines(text_a, text_b);

        let mut changeset = Vec::new();
        let mut line_a: i32 = 0;
        let mut line_b: i32 = 0;

        for change in diff.iter_all_changes() {
            let tag = change.tag();
            let value = change.value();
            // Remove trailing newline for cleaner display
            let content = if value.ends_with('\n') {
                value[..value.len() - 1].to_string()
            } else {
                value.to_string()
            };

            let (change_type, count_a, count_b) = match tag {
                ChangeTag::Equal => {
                    line_a += 1;
                    line_b += 1;
                    ("unchanged", Some(line_a - 1), Some(line_b - 1))
                }
                ChangeTag::Delete => {
                    line_a += 1;
                    ("removed", Some(line_a - 1), None)
                }
                ChangeTag::Insert => {
                    line_b += 1;
                    ("added", None, Some(line_b - 1))
                }
            };

            changeset.push(DiffChange {
                change_type: change_type.to_string(),
                content,
                line_number_a: count_a,
                line_number_b: count_b,
            });
        }

        // Compute stats
        let total_changes = changeset.len() as f64;
        let additions = changeset.iter().filter(|c| c.change_type == "added").count() as i32;
        let deletions = changeset.iter().filter(|c| c.change_type == "removed").count() as i32;
        let unchanged = changeset.iter().filter(|c| c.change_type == "unchanged").count() as i32;
        let similarity = if total_changes > 0.0 {
            unchanged as f64 / total_changes
        } else {
            1.0
        };

        serde_json::json!({
            "changeset": changeset,
            "stats": {
                "additions": additions,
                "deletions": deletions,
                "unchanged_lines": unchanged,
            },
            "similarity_score": similarity,
        })
    }

    /// Extract plain text from a supported Office document based on its extension.
    pub fn extract_text(path: &str) -> Result<String, String> {
        let ext = Self::get_extension(path)?;
        match ext.as_str() {
            "docx" | "doc" => Self::extract_docx_text(path),
            "xlsx" | "xls" => Self::extract_xlsx_text(path),
            "pptx" | "ppt" => Self::extract_pptx_text(path),
            other => Err(format!("Unsupported format: {}. Supported: docx, xlsx, pptx", other)),
        }
    }

    // ── Private helpers ──────────────────────────────────────────

    fn validate_paths(path_a: &str, path_b: &str) -> Result<(), String> {
        let path_a = std::path::Path::new(path_a);
        let path_b = std::path::Path::new(path_b);

        if !path_a.exists() {
            return Err(format!("File not found: {}", path_a.display()));
        }
        if !path_b.exists() {
            return Err(format!("File not found: {}", path_b.display()));
        }

        let ext_a = Self::get_extension(path_a.to_str().unwrap_or(""))?;
        let ext_b = Self::get_extension(path_b.to_str().unwrap_or(""))?;

        if ext_a != ext_b {
            return Err(format!(
                "Format mismatch: files have different extensions ({} vs {}). Both files must be the same format.",
                ext_a, ext_b
            ));
        }

        Ok(())
    }

    fn get_extension(path: &str) -> Result<String, String> {
        let p = std::path::Path::new(path);
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext.is_empty() {
            return Err(format!("File has no extension: {}", path));
        }
        Ok(ext)
    }

    /// Extract text from DOCX by parsing word/document.xml <w:t> elements.
    fn extract_docx_text(path: &str) -> Result<String, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Cannot open file '{}': {}", path, e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Cannot read ZIP archive '{}': {}", path, e))?;

        let mut entry = archive
            .by_name("word/document.xml")
            .map_err(|_| "Cannot find word/document.xml in archive".to_string())?;
        let mut xml = String::new();
        entry
            .read_to_string(&mut xml)
            .map_err(|e| format!("Cannot read word/document.xml: {}", e))?;

        Self::parse_docx_xml(&xml)
    }

    fn parse_docx_xml(xml: &str) -> Result<String, String> {
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut buf = Vec::new();
        let mut text = String::new();
        let mut in_w_p = false;
        let mut in_w_t = false;
        let mut paragraph_text = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Start(ref e))
                | Ok(quick_xml::events::Event::Empty(ref e)) => {
                    let qn = e.name();
                    let tag = qn.as_ref();
                    if tag == b"w:p" {
                        in_w_p = true;
                        paragraph_text.clear();
                    } else if tag == b"w:t" && in_w_p {
                        in_w_t = true;
                    }
                }
                Ok(quick_xml::events::Event::Text(ref e)) => {
                    if in_w_t {
                        if let Ok(t) = e.unescape() {
                            paragraph_text.push_str(&t);
                        }
                    }
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    let qn = e.name();
                    let tag = qn.as_ref();
                    if tag == b"w:t" {
                        in_w_t = false;
                    } else if tag == b"w:p" && in_w_p {
                        let trimmed = paragraph_text.trim().to_string();
                        if !trimmed.is_empty() {
                            if !text.is_empty() {
                                text.push('\n');
                            }
                            text.push_str(&trimmed);
                        }
                        in_w_p = false;
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(e) => return Err(format!("XML parse error in word/document.xml: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(text)
    }

    /// Extract text from XLSX using calamine: iterate sheets and rows.
    fn extract_xlsx_text(path: &str) -> Result<String, String> {
        use calamine::{open_workbook, Data, Reader, Xlsx};

        let mut workbook: Xlsx<std::io::BufReader<std::fs::File>> =
            open_workbook(path).map_err(|e| format!("Cannot open workbook '{}': {}", path, e))?;

        let mut text = String::new();
        let sheet_names = workbook.sheet_names().to_vec();

        for (si, sheet_name) in sheet_names.iter().enumerate() {
            if si > 0 {
                text.push('\n');
            }
            text.push_str(&format!("=== Sheet: {} ===\n", sheet_name));

            let range = workbook
                .worksheet_range(sheet_name)
                .map_err(|e| format!("Cannot read sheet '{}': {}", sheet_name, e))?;

            for row in range.rows() {
                let row_text: Vec<String> = row
                    .iter()
                    .map(|cell| match cell {
                        Data::String(s) => s.clone(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        Data::DateTime(dt) => dt.to_string(),
                        Data::DateTimeIso(s) => s.clone(),
                        Data::DurationIso(s) => s.clone(),
                        Data::Error(e) => format!("#{}", e),
                        Data::Empty => String::new(),
                    })
                    .collect();
                text.push_str(&row_text.join("\t"));
                text.push('\n');
            }
        }

        Ok(text)
    }

    /// Extract text from PPTX by parsing ppt/slides/slide*.xml <a:t> elements.
    fn extract_pptx_text(path: &str) -> Result<String, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Cannot open file '{}': {}", path, e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Cannot read ZIP archive '{}': {}", path, e))?;

        // Collect slide entry names sorted
        let mut slide_names: Vec<String> = Vec::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name().to_string();
            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                slide_names.push(name);
            }
        }
        slide_names.sort();

        if slide_names.is_empty() {
            return Err("No slides found in presentation".to_string());
        }

        let mut text = String::new();

        for (si, slide_name) in slide_names.iter().enumerate() {
            if si > 0 {
                text.push('\n');
            }
            text.push_str(&format!("--- Slide {} ---\n", si + 1));

            let mut entry = archive
                .by_name(slide_name)
                .map_err(|e| format!("Cannot read {}: {}", slide_name, e))?;
            let mut xml = String::new();
            entry
                .read_to_string(&mut xml)
                .map_err(|e| format!("Cannot read {}: {}", slide_name, e))?;

            let slide_text = Self::parse_pptx_slide_xml(&xml);
            text.push_str(slide_text.trim());
        }

        Ok(text)
    }

    fn parse_pptx_slide_xml(xml: &str) -> String {
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut buf = Vec::new();
        let mut in_a_t = false;
        let mut slide_text = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    let qn = e.name();
                    let tag = qn.as_ref();
                    if tag == b"a:t" {
                        in_a_t = true;
                    }
                }
                Ok(quick_xml::events::Event::Text(ref e)) => {
                    if in_a_t {
                        if let Ok(t) = e.unescape() {
                            slide_text.push_str(&t);
                        }
                    }
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    let qn = e.name();
                    let tag = qn.as_ref();
                    if tag == b"a:t" {
                        in_a_t = false;
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        slide_text
    }

    /// Extract text with structural markers for detailed diff (per-paragraph/slide/sheet).
    fn extract_text_detailed(path: &str, ext: &str) -> Result<String, String> {
        match ext {
            "docx" | "doc" => Self::extract_docx_text_detailed(path),
            "xlsx" | "xls" => Self::extract_xlsx_text_detailed(path),
            "pptx" | "ppt" => Self::extract_pptx_text(path),
            other => Err(format!("Unsupported format: {}", other)),
        }
    }

    /// DOCX detailed: each paragraph on its own line with paragraph index.
    fn extract_docx_text_detailed(path: &str) -> Result<String, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Cannot open file '{}': {}", path, e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Cannot read ZIP: {}", e))?;

        let mut entry = archive
            .by_name("word/document.xml")
            .map_err(|_| "Cannot find word/document.xml".to_string())?;
        let mut xml = String::new();
        entry
            .read_to_string(&mut xml)
            .map_err(|e| format!("Cannot read XML: {}", e))?;

        let mut reader = quick_xml::Reader::from_str(&xml);
        let mut buf = Vec::new();
        let mut text = String::new();
        let mut in_w_p = false;
        let mut in_w_t = false;
        let mut paragraph_text = String::new();
        let mut para_index: usize = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Start(ref e))
                | Ok(quick_xml::events::Event::Empty(ref e)) => {
                    let qn = e.name();
                    let tag = qn.as_ref();
                    if tag == b"w:p" {
                        in_w_p = true;
                        paragraph_text.clear();
                    } else if tag == b"w:t" && in_w_p {
                        in_w_t = true;
                    }
                }
                Ok(quick_xml::events::Event::Text(ref e)) => {
                    if in_w_t {
                        if let Ok(t) = e.unescape() {
                            paragraph_text.push_str(&t);
                        }
                    }
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    let qn = e.name();
                    let tag = qn.as_ref();
                    if tag == b"w:t" {
                        in_w_t = false;
                    } else if tag == b"w:p" && in_w_p {
                        let trimmed = paragraph_text.trim().to_string();
                        if !trimmed.is_empty() {
                            if !text.is_empty() {
                                text.push('\n');
                            }
                            text.push_str(&format!("[P{}] {}", para_index, trimmed));
                            para_index += 1;
                        }
                        in_w_p = false;
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        Ok(text)
    }

    /// XLSX detailed: cell-by-cell per sheet with cell references.
    fn extract_xlsx_text_detailed(path: &str) -> Result<String, String> {
        use calamine::{open_workbook, Data, Reader, Xlsx};

        let mut workbook: Xlsx<std::io::BufReader<std::fs::File>> =
            open_workbook(path).map_err(|e| format!("Cannot open workbook: {}", e))?;

        let mut text = String::new();
        let sheet_names = workbook.sheet_names().to_vec();

        for (si, sheet_name) in sheet_names.iter().enumerate() {
            if si > 0 {
                text.push('\n');
            }
            text.push_str(&format!("=== Sheet: {} ===\n", sheet_name));

            let range = workbook
                .worksheet_range(sheet_name)
                .map_err(|e| format!("Cannot read sheet '{}': {}", sheet_name, e))?;

            for row in range.rows() {
                for (ci, cell) in row.iter().enumerate() {
                    let cell_text = match cell {
                        Data::String(s) => s.clone(),
                        Data::Float(f) => f.to_string(),
                        Data::Int(i) => i.to_string(),
                        Data::Bool(b) => b.to_string(),
                        Data::DateTime(dt) => dt.to_string(),
                        _ => String::new(),
                    };
                    if !cell_text.is_empty() {
                        let col_letter = column_letter(ci as u32);
                        text.push_str(&format!("  {}:{}\n", col_letter, cell_text));
                    }
                }
            }
        }

        Ok(text)
    }
}

fn column_letter(col: u32) -> String {
    let mut c = col;
    let mut result = String::new();
    loop {
        let rem = c % 26;
        result.insert(0, (b'A' + rem as u8) as char);
        c /= 26;
        if c == 0 {
            break;
        }
        c -= 1;
    }
    result
}

impl Default for DocumentDiff {
    fn default() -> Self {
        Self::new()
    }
}
