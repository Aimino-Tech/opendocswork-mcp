#![allow(dead_code)]

mod coherence;
mod diff;
mod formats;
mod readers;
mod writers;
mod patchers;
mod skills;
mod integrity;
mod export;
mod pdf_form;
mod template;

use rmcp::{
    handler::server::wrapper::Parameters, model::Content, schemars, serve_server, tool,
    tool_handler, tool_router, ErrorData, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::Mutex;
use office_oxide_mcp::validation::ValidationEngine;

use crate::coherence::{
    CoherenceEngine, ConsistencyCheckRequest, EntityGraphRequest, PropagateEditRequest,
};
use crate::export::PdfExport;
use crate::skills::registry::SkillRegistry;

#[derive(Debug)]
struct OfficeService {
    counter: Arc<Mutex<i32>>,
    engine: Arc<ValidationEngine>,
    skill_registry: Arc<Mutex<SkillRegistry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct DocumentInfoRequest {
    file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct OfficeReadRequest {
    file_path: String,
    output_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ValidateRequest {
    file_path: String,
    checks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rules_config: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct FixRequest {
    file_path: String,
    tool_name: String,
    args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ReplaceTextRequest {
    file_path: String,
    replacements: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct DiffDocumentsRequest {
    file_path_a: String,
    file_path_b: String,
    #[serde(default)]
    detailed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ListSkillsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct GetSkillRequest {
    skill_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct ValidateSkillRequest {
    skill_name: String,
    params: HashMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct RunSkillRequest {
    skill_name: String,
    params: HashMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct SkillRegisterRequest {
    definition_yaml: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct SkillRemoveRequest {
    skill_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct PdfExportRequest {
    file_path: String,
    output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct PdfFillFormRequest {
    file_path: String,
    output_path: String,
    fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct PdfListFieldsRequest {
    file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct PdfAnalyzeLayoutRequest {
    file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct PdfOverlayTextRequest {
    file_path: String,
    output_path: String,
    fields: Vec<TextFieldOverlayParam>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct TextFieldOverlayParam {
    page: u32,
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_name: Option<String>,
}

// ── Template Engine request structs ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct TemplateFillRequest {
    file_path: String,
    output_path: String,
    data: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct TemplateBatchRequest {
    file_path: String,
    output_dir: String,
    records: Vec<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct TemplateDetectRequest {
    file_path: String,
}

impl OfficeService {
    fn new() -> Self {
        Self::with_skills_dir("skills")
    }

    fn with_skills_dir(skills_dir: &str) -> Self {
        let mut registry = skills::registry::SkillRegistry::new();
        registry.load_builtins();
        let _ = registry.load_from_filesystem(skills_dir);
        Self {
            counter: Arc::new(Mutex::new(0)),
            engine: Arc::new(ValidationEngine::new()),
            skill_registry: Arc::new(Mutex::new(registry)),
        }
    }

    fn is_supported(ext: &str) -> bool {
        matches!(ext, "docx" | "xlsx" | "pptx" | "doc" | "xls" | "ppt" | "pdf")
    }

    fn json_content(value: impl serde::Serialize) -> Result<Content, ErrorData> {
        let text = serde_json::to_string_pretty(&value).map_err(|e| {
            ErrorData::internal_error(
                "serialization_error",
                Some(serde_json::json!({"detail": e.to_string()})),
            )
        })?;
        Ok(Content::text(text))
    }
}

#[tool_router]
impl OfficeService {
    #[tool(description = "List all supported Office document formats with capabilities")]
    async fn list_formats(&self) -> Result<Content, ErrorData> {
        let formats = serde_json::json!([
            {"extension": "pdf", "name": "PDF Document", "read": true, "write": false, "reader": "lopdf + office_oxide (native)", "tools": ["office_read", "office_fill_pdf_form", "office_list_pdf_fields", "office_overlay_pdf_text", "office_analyze_pdf_layout"]},
            {"extension": "docx", "name": "Word Document", "read": true, "write": false, "reader": "EPIC-1 Word (AIM-877)"},
            {"extension": "doc", "name": "Word 97-2003 Document", "read": true, "write": false, "reader": "EPIC-1 Word (AIM-877)"},
            {"extension": "xlsx", "name": "Excel Workbook", "read": false, "write": false},
            {"extension": "xls", "name": "Excel 97-2003 Workbook", "read": false, "write": false},
            {"extension": "pptx", "name": "PowerPoint Presentation", "read": false, "write": false},
            {"extension": "ppt", "name": "PowerPoint 97-2003 Presentation", "read": false, "write": false}
        ]);
        Ok(Content::text(
            serde_json::to_string_pretty(&formats).expect("valid json"),
        ))
    }

    #[tool(description = "Get metadata about an Office document")]
    async fn get_document_info(
        &self,
        Parameters(req): Parameters<DocumentInfoRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_lowercase();
        let metadata = std::fs::metadata(path).map_err(|e| {
            ErrorData::internal_error(
                "metadata_error",
                Some(serde_json::json!({"detail": e.to_string()})),
            )
        })?;
        let readable = Self::is_supported(&ext);
        let mut info = serde_json::json!({
            "path": req.file_path, "format": ext,
            "size_bytes": metadata.len(), "readable": readable
        });
        if ext == "xlsx" || ext == "xls" {
            match readers::excel::ExcelReader::read_to_json(path) {
                Ok(excel_info) => {
                    let sheet_summaries: Vec<serde_json::Value> = excel_info
                        .sheets
                        .iter()
                        .map(|s| {
                            serde_json::json!({
                                "name": s.name,
                                "dimensions": s.dimensions,
                                "row_count": s.row_count,
                                "column_count": s.column_count,
                                "column_types": s.column_types,
                            })
                        })
                        .collect();
                    info["sheets"] = serde_json::json!(sheet_summaries);
                    info["named_ranges"] = serde_json::json!(excel_info.named_ranges);
                }
                Err(e) => {
                    info["read_error"] = serde_json::json!(e);
                }
            }
        }
        Ok(Content::text(serde_json::to_string_pretty(&info).unwrap()))
    }

    #[tool(
        description = "Read content from an Office document or PDF. Output formats: json, markdown, chunks. PDF also supports 'text' format. For PDFs with form fields, use office_list_pdf_fields first to see available fields."
    )]
    async fn office_read(
        &self,
        Parameters(req): Parameters<OfficeReadRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_lowercase();
        if !Self::is_supported(&ext) {
            let supported: Vec<&str> = vec!["docx", "xlsx", "pptx", "doc", "xls", "ppt"];
            return Err(ErrorData::invalid_params(
                "unsupported_format",
                Some(serde_json::json!({"format": ext, "supported": supported})),
            ));
        }
        let _size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let result = match (ext.as_str(), req.output_format.as_str()) {
            ("xlsx" | "xls", "json") => readers::excel::ExcelReader::read_to_json(path)
                .map(|output| serde_json::to_string_pretty(&output).unwrap()),
            ("xlsx" | "xls", "markdown") => readers::excel::ExcelReader::read_to_md(path),
            ("xlsx" | "xls", "chunks") => readers::excel::ExcelReader::read_to_chunks(path)
                .map(|chunks| serde_json::to_string_pretty(&chunks).unwrap()),
            ("docx" | "doc", "json") => readers::word::read_word_to_json(&req.file_path)
                .map(|output| serde_json::to_string_pretty(&output).unwrap()),
            ("docx" | "doc", "markdown") => readers::word::read_word_to_md(&req.file_path),
            ("docx" | "doc", "chunks") => readers::word::read_word_to_chunks(&req.file_path)
                .map(|chunks| serde_json::to_string_pretty(&chunks).unwrap()),
            ("pptx" | "ppt", "json") => readers::powerpoint::read_ppt_to_json(&req.file_path)
                .map(|output| serde_json::to_string_pretty(&output).unwrap()),
            ("pptx" | "ppt", "markdown") => readers::powerpoint::read_ppt_to_md(&req.file_path),
            ("pptx" | "ppt", "chunks") => readers::powerpoint::read_ppt_to_chunks(&req.file_path)
                .map(|chunks| serde_json::to_string_pretty(&chunks).unwrap()),
            ("pdf", "markdown") => pdf_form::read_pdf_to_md(&req.file_path),
            ("pdf", "text") => pdf_form::read_pdf_text(&req.file_path),
            ("pdf", "json") => pdf_form::read_pdf_json(&req.file_path),
            ("pdf", "chunks") => pdf_form::read_pdf_chunks(&req.file_path)
                .map(|chunks| serde_json::to_string_pretty(&chunks).unwrap()),
            _ => {
                let supported: Vec<&str> = vec!["docx", "xlsx", "pptx", "doc", "xls", "ppt"];
                let valid_formats = if Self::is_supported(&ext) {
                    if ext == "pdf" {
                        vec![
                            "markdown".to_string(),
                            "text".to_string(),
                            "json".to_string(),
                            "chunks".to_string(),
                        ]
                    } else {
                        vec![
                            "json".to_string(),
                            "markdown".to_string(),
                            "chunks".to_string(),
                        ]
                    }
                } else {
                    vec![]
                };
                return Err(ErrorData::invalid_params(
                    if Self::is_supported(&ext) {
                        "unsupported_output_format"
                    } else {
                        "unsupported_format"
                    },
                    Some(serde_json::json!({
                        "format": ext, "supported": supported,
                        "valid_output_formats": valid_formats
                    })),
                ));
            }
        };
        match result {
            Ok(text) => Ok(Content::text(text)),
            Err(e) => Err(ErrorData::internal_error(
                "read_error",
                Some(serde_json::json!({"path": req.file_path, "detail": e})),
            )),
        }
    }

    #[tool(
        description = "Compare two Office documents and return semantic diff. Supports DOCX, XLSX, PPTX. Returns additions, deletions, and similarity score."
    )]
    async fn office_diff_documents(
        &self,
        Parameters(req): Parameters<DiffDocumentsRequest>,
    ) -> Result<Content, ErrorData> {
        let result = if req.detailed {
            diff::document_diff::DocumentDiff::diff_documents_detailed(
                &req.file_path_a,
                &req.file_path_b,
            )
        } else {
            diff::document_diff::DocumentDiff::diff_documents(
                &req.file_path_a,
                &req.file_path_b,
            )
        };

        match result {
            Ok(text) => Ok(Content::text(text)),
            Err(e) => {
                // Determine if this is a user error (missing file, format mismatch) or internal
                if e.starts_with("File not found:") || e.starts_with("Format mismatch:")
                    || e.starts_with("Unsupported format:")
                    || e.starts_with("No slides found")
                    || e.starts_with("Cannot find word/document.xml")
                    || e.contains("different extensions")
                {
                    Err(ErrorData::invalid_params(
                        "diff_error",
                        Some(serde_json::json!({"detail": e})),
                    ))
                } else {
                    Err(ErrorData::internal_error(
                        "diff_error",
                        Some(serde_json::json!({"detail": e})),
                    ))
                }
            }
        }
    }

    #[tool(
        description = "Propagate an entity edit through the dependency graph. Updates the entity value and BFS-cascade notifies all dependents, capped at depth 3. Creates/updates the coherence manifest sidecar file."
    )]
    async fn office_propagate_edit(
        &self,
        Parameters(req): Parameters<PropagateEditRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }

        let result = CoherenceEngine::propagate(
            &req.file_path,
            &req.entity_id,
            &req.new_value,
            &req.dependents,
        )
        .map_err(|e| {
            ErrorData::internal_error("propagation_error", Some(serde_json::json!({"detail": e})))
        })?;

        Self::json_content(&result)
    }

    #[tool(
        description = "Check consistency of entities in an Office document. Re-hashes all manifest values and reports any stale (externally modified) entities."
    )]
    async fn office_check_consistency(
        &self,
        Parameters(req): Parameters<ConsistencyCheckRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }

        let result = CoherenceEngine::check_consistency(&req.file_path).map_err(|e| {
            ErrorData::internal_error("consistency_error", Some(serde_json::json!({"detail": e})))
        })?;

        Self::json_content(&result)
    }

    #[tool(
        description = "Get the entity dependency graph for an Office document. Lists all tracked entities, their values, hashes, dependency relationships from the manifest."
    )]
    async fn office_get_entity_graph(
        &self,
        Parameters(req): Parameters<EntityGraphRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }

        let result = CoherenceEngine::get_entity_graph(&req.file_path).map_err(|e| {
            ErrorData::internal_error("entity_graph_error", Some(serde_json::json!({"detail": e})))
        })?;

        Self::json_content(&result)
    }

    #[tool(
        description = "Export an Office document to PDF. Supports DOCX, XLSX, PPTX. Returns the output path on success."
    )]
    async fn office_export_pdf(
        &self,
        Parameters(req): Parameters<PdfExportRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_lowercase();
        let supported = vec!["docx", "doc", "xlsx", "xls", "pptx", "ppt"];
        if !supported.contains(&ext.as_str()) {
            return Err(ErrorData::invalid_params(
                "unsupported_format",
                Some(serde_json::json!({
                    "format": ext,
                    "supported": supported,
                    "detail": "Supported formats: docx, xlsx, pptx"
                })),
            ));
        }
        let result = PdfExport::export_to_pdf(&req.file_path, &req.output_path);
        match result {
            Ok(json) => Ok(Content::text(json)),
            Err(e) => Err(ErrorData::internal_error(
                "pdf_export_error",
                Some(serde_json::json!({
                    "path": req.file_path,
                    "output_path": req.output_path,
                    "detail": e
                })),
            )),
        }
    }

    #[tool(description = "Increment the internal counter by 1")]
    async fn increment(&self) -> Result<Content, ErrorData> {
        let mut c = self.counter.lock().await;
        *c += 1;
        Ok(Content::text(c.to_string()))
    }

    #[tool(description = "Get the current counter value")]
    async fn get_value(&self) -> Result<Content, ErrorData> {
        let c = self.counter.lock().await;
        Ok(Content::text(c.to_string()))
    }

    #[tool(
        description = "Validate an Office document for structural integrity, formatting, and coherence."
    )]
    async fn office_validate(
        &self,
        Parameters(req): Parameters<ValidateRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }

        let rules_config_value = req.rules_config.map(|m| serde_json::Value::Object(m.into_iter().collect()));
        let report = self
            .engine
            .validate(
                &req.file_path,
                req.checks.as_deref(),
                rules_config_value.as_ref(),
            )
            .await;

        Self::json_content(&report)
    }

    #[tool(
        description = "Fill form fields in a PDF document. Supports AcroForm and XFA forms. Provide field name to value mappings."
    )]
    async fn office_fill_pdf_form(
        &self,
        Parameters(req): Parameters<PdfFillFormRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_lowercase();
        if ext != "pdf" {
            return Err(ErrorData::invalid_params(
                "invalid_extension",
                Some(serde_json::json!({"expected": "pdf", "got": ext})),
            ));
        }
        if req.fields.is_empty() {
            return Err(ErrorData::invalid_params(
                "empty_fields",
                Some(serde_json::json!({"detail": "At least one field mapping is required"})),
            ));
        }
        let filler = pdf_form::PdfFormFiller::new();
        match filler.fill_form(&req.file_path, &req.output_path, &req.fields) {
            Ok(json) => Ok(Content::text(json)),
            Err(e) => Err(ErrorData::internal_error(
                "fill_form_error",
                Some(serde_json::json!({"detail": e})),
            )),
        }
    }

    #[tool(
        description = "List all form fields in a PDF document with their current values."
    )]
    async fn office_list_pdf_fields(
        &self,
        Parameters(req): Parameters<PdfListFieldsRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_lowercase();
        if ext != "pdf" {
            return Err(ErrorData::invalid_params(
                "invalid_extension",
                Some(serde_json::json!({"expected": "pdf", "got": ext})),
            ));
        }
        let filler = pdf_form::PdfFormFiller::new();
        match filler.list_fields(&req.file_path) {
            Ok(json) => Ok(Content::text(json)),
            Err(e) => Err(ErrorData::internal_error(
                "list_fields_error",
                Some(serde_json::json!({"detail": e})),
            )),
        }
    }

    #[tool(
        description = "Insert text at specific positions on PDF pages without form fields. Uses content stream overlay to add text to existing PDFs. Specify coordinates in PDF points (72 dpi, bottom-left origin). Supports standard fonts (Helvetica, Times-Roman, Courier)."
    )]
    async fn office_overlay_pdf_text(
        &self,
        Parameters(req): Parameters<PdfOverlayTextRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown")
            .to_lowercase();
        if ext != "pdf" {
            return Err(ErrorData::invalid_params(
                "invalid_extension",
                Some(serde_json::json!({"expected": "pdf", "got": ext})),
            ));
        }
        if req.fields.is_empty() {
            return Err(ErrorData::invalid_params(
                "empty_fields",
                Some(serde_json::json!({"detail": "At least one field is required"})),
            ));
        }

        let fields: Vec<pdf_form::TextFieldOverlay> = req
            .fields
            .iter()
            .map(|f| pdf_form::TextFieldOverlay {
                page: f.page,
                x: f.x,
                y: f.y,
                text: f.text.clone(),
                font_size: f.font_size.unwrap_or(11.0),
                font_name: f.font_name.clone().unwrap_or_else(|| "Helvetica".to_string()),
            })
            .collect();

        let filler = pdf_form::FlatPdfFiller::new();
        match filler.fill_flat_pdf(&req.file_path, &req.output_path, &fields) {
            Ok(json) => Ok(Content::text(json)),
            Err(e) => Err(ErrorData::internal_error(
                "overlay_text_error",
                Some(serde_json::json!({"detail": e})),
            )),
        }
    }

    #[tool(
        description = "Analyze a PDF page layout: extract all text with positions, detect form field labels, and suggest overlay coordinates. Use this before office_overlay_pdf_text to find where to place text on flat PDFs."
    )]
    async fn office_analyze_pdf_layout(
        &self,
        Parameters(req): Parameters<PdfAnalyzeLayoutRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if ext != "pdf" {
            return Err(ErrorData::invalid_params(
                "invalid_extension",
                Some(serde_json::json!({"expected": "pdf", "got": ext})),
            ));
        }

        let analysis = pdf_form::PdfFormFiller::analyze_layout(&req.file_path)
            .map_err(|e| ErrorData::internal_error(
                "layout_analysis_error",
                Some(serde_json::json!({"detail": e})),
            ))?;

        Ok(Content::text(analysis))
    }

    // ── New skill tools matching e2e_skills tests ─────────────

    #[tool(
        description = "List available skills with metadata. Optionally filter by category: excel, word, ppt"
    )]
    async fn list_skills(
        &self,
        Parameters(req): Parameters<ListSkillsRequest>,
    ) -> Result<Content, ErrorData> {
        let registry = self.skill_registry.lock().await;
        let skills = match req.category {
            Some(ref cat) => {
                let category = match cat.as_str() {
                    "excel" => skills::SkillCategory::Excel,
                    "word" => skills::SkillCategory::Word,
                    "ppt" => skills::SkillCategory::Ppt,
                    _ => {
                        return Err(ErrorData::invalid_params(
                            "invalid_category",
                            Some(serde_json::json!({"valid": ["excel", "word", "ppt"]})),
                        ))
                    }
                };
                registry.list_by_category(&category)
            }
            None => registry.list(),
        };
        Ok(Content::text(
            serde_json::to_string_pretty(&skills).unwrap(),
        ))
    }

    #[tool(description = "Get a skill definition by name")]
    async fn get_skill(
        &self,
        Parameters(req): Parameters<GetSkillRequest>,
    ) -> Result<Content, ErrorData> {
        let registry = self.skill_registry.lock().await;
        let skill = registry.get(&req.skill_name).ok_or_else(|| {
            ErrorData::invalid_params(
                "skill_not_found",
                Some(serde_json::json!({"skill_name": req.skill_name})),
            )
        })?;
        Ok(Content::text(
            serde_json::to_string_pretty(&skill).unwrap(),
        ))
    }

    #[tool(description = "Validate skill parameters against a skill's validation rules")]
    async fn validate_skill(
        &self,
        Parameters(req): Parameters<ValidateSkillRequest>,
    ) -> Result<Content, ErrorData> {
        let registry = self.skill_registry.lock().await;
        let params: serde_json::Map<String, serde_json::Value> = req.params.into_iter().collect();
        let result = registry
            .validate_skill(&req.skill_name, &params)
            .map_err(|e| {
                ErrorData::invalid_params(
                    "skill_not_found",
                    Some(serde_json::json!({"detail": e})),
                )
            })?;
        Ok(Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        ))
    }

    #[tool(description = "Execute a skill by name with provided parameters")]
    async fn run_skill(
        &self,
        Parameters(req): Parameters<RunSkillRequest>,
    ) -> Result<Content, ErrorData> {
        let registry = self.skill_registry.lock().await;
        let params: serde_json::Map<String, serde_json::Value> = req.params.into_iter().collect();
        let result = registry.run_skill(&req.skill_name, &params).map_err(|e| {
            ErrorData::invalid_params(
                "skill_error",
                Some(serde_json::json!({"detail": e})),
            )
        })?;
        Ok(Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        ))
    }

    // ── Legacy skill tools (keep for backward compat) ────────

    #[tool(
        description = "Register a custom skill at runtime from a YAML definition string. Persists to disk in skills/ directory."
    )]
    async fn skill_register(
        &self,
        Parameters(req): Parameters<SkillRegisterRequest>,
    ) -> Result<Content, ErrorData> {
        let skill: skills::SkillDefinition =
            serde_yaml::from_str(&req.definition_yaml).map_err(|e| {
                ErrorData::invalid_params(
                    "invalid_yaml",
                    Some(serde_json::json!({"detail": e.to_string()})),
                )
            })?;

        let mut registry = self.skill_registry.lock().await;
        registry.register_and_persist(skill.clone()).map_err(|e| {
            ErrorData::invalid_params("registration_error", Some(serde_json::json!({"detail": e})))
        })?;

        Ok(Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "status": "registered",
                "name": skill.name,
                "version": skill.version,
                "persisted": true,
            }))
            .unwrap(),
        ))
    }

    #[tool(description = "Remove a registered skill by name. Built-in skills cannot be removed.")]
    async fn skill_remove(
        &self,
        Parameters(req): Parameters<SkillRemoveRequest>,
    ) -> Result<Content, ErrorData> {
        let mut registry = self.skill_registry.lock().await;
        let removed = registry.remove(&req.skill_name);
        if !removed {
            return Err(ErrorData::invalid_params(
                "skill_not_found",
                Some(serde_json::json!({"skill": req.skill_name})),
            ));
        }
        Ok(Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "status": "removed",
                "name": req.skill_name,
            }))
            .unwrap(),
        ))
    }

    // ── Template Engine tools ─────────────────────────────────

    #[tool(
        description = "Fill a DOCX template with data. Replaces {placeholders} with provided values. Supports mail merge."
    )]
    async fn office_template_fill(
        &self,
        Parameters(req): Parameters<TemplateFillRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "docx" {
            return Err(ErrorData::invalid_params(
                "unsupported_format",
                Some(serde_json::json!({"format": ext, "supported": ["docx"]})),
            ));
        }

        match template::TemplateEngine::fill_template(
            &req.file_path,
            &req.output_path,
            &req.data,
        ) {
            Ok(text) => Ok(Content::text(text)),
            Err(e) => Err(ErrorData::internal_error(
                "template_fill_error",
                Some(serde_json::json!({"detail": e})),
            )),
        }
    }

    #[tool(
        description = "Batch process a DOCX template with multiple data records. Uses parallel processing with rayon."
    )]
    async fn office_template_batch(
        &self,
        Parameters(req): Parameters<TemplateBatchRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "docx" {
            return Err(ErrorData::invalid_params(
                "unsupported_format",
                Some(serde_json::json!({"format": ext, "supported": ["docx"]})),
            ));
        }

        match template::TemplateEngine::batch_fill(
            &req.file_path,
            &req.output_dir,
            &req.records,
        ) {
            Ok(text) => Ok(Content::text(text)),
            Err(e) => Err(ErrorData::internal_error(
                "template_batch_error",
                Some(serde_json::json!({"detail": e})),
            )),
        }
    }

    #[tool(
        description = "Detect all placeholders in a DOCX template. Returns unique placeholder names found."
    )]
    async fn office_template_detect(
        &self,
        Parameters(req): Parameters<TemplateDetectRequest>,
    ) -> Result<Content, ErrorData> {
        let path = std::path::Path::new(&req.file_path);
        if !path.exists() {
            return Err(ErrorData::invalid_params(
                "file_not_found",
                Some(serde_json::json!({"path": req.file_path})),
            ));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "docx" {
            return Err(ErrorData::invalid_params(
                "unsupported_format",
                Some(serde_json::json!({"format": ext, "supported": ["docx"]})),
            ));
        }

        match template::TemplateEngine::detect_placeholders(&req.file_path) {
            Ok(text) => Ok(Content::text(text)),
            Err(e) => Err(ErrorData::internal_error(
                "template_detect_error",
                Some(serde_json::json!({"detail": e})),
            )),
        }
    }
}

impl OfficeService {
    async fn dispatch_fix(
        &self,
        tool_name: &str,
        file_path: &str,
        args: &serde_json::Value,
    ) -> Result<(), String> {
        match tool_name {
            "office_validate" => Ok(()),
            "office_replace_text" => {
                let req: ReplaceTextRequest = serde_json::from_value(args.clone())
                    .map_err(|e| format!("Invalid args for office_replace_text: {}", e))?;
                let target_path = if req.file_path.is_empty() { file_path } else { &req.file_path };
                replace_text_in_zip(target_path, &req.replacements)
            }
            name => Err(format!("Unknown fix tool '{}'. Available: ['office_validate', 'office_replace_text']", name)),
        }
    }
}

fn replace_text_in_zip(file_path: &str, replacements: &HashMap<String, String>) -> Result<(), String> {
    let tmp_path = format!("{}.tmp", file_path);
    let _ = std::fs::remove_file(&tmp_path);

    let file = std::fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;

        let should_process = name.ends_with(".xml")
            || name.ends_with(".rels")
            || name == "[Content_Types].xml";

        if should_process {
            match String::from_utf8(data) {
                Ok(text) => {
                    let mut modified = text;
                    for (search, replace) in replacements {
                        modified = modified.replace(search, replace);
                    }
                    entries.push((name, modified.into_bytes()));
                }
                Err(e) => {
                    entries.push((name, e.into_bytes()));
                }
            }
        } else {
            entries.push((name, data));
        }
    }

    let result = (|| -> Result<(), String> {
        let tmp_file = std::fs::File::create(&tmp_path).map_err(|e| e.to_string())?;
        let mut zip_writer = zip::ZipWriter::new(tmp_file);

        let options = zip::write::FileOptions::<'_, ()>::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for (name, data) in &entries {
            zip_writer.start_file(name, options).map_err(|e| e.to_string())?;
            zip_writer.write_all(data).map_err(|e| e.to_string())?;
        }

        zip_writer.finish().map_err(|e| e.to_string())?;
        std::fs::rename(&tmp_path, file_path).map_err(|e| e.to_string())?;
        Ok(())
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    result
}

#[tool_handler(instructions = "Office Document MCP Server — read, validate, fix, export to PDF, and replace text in Office files (DOCX, XLSX, PPTX, DOC, XLS, PPT, PDF) and fill/manipulate PDFs. Tools: list_formats, get_document_info, office_read (including PDF text extraction), office_validate, office_export_pdf, office_replace_text, office_fix, office_fill_pdf_form, office_list_pdf_fields, office_overlay_pdf_text, increment, get_value. Usage: validate → check report → fix with office_fix using suggested tool/args → re-validate. For PDFs without form fields, use office_overlay_pdf_text with page coordinates. To read PDF content, use office_read with format 'markdown', 'text', 'json', or 'chunks'.")]
impl ServerHandler for OfficeService {}

fn parse_args() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                println!("office-oxide-mcp v{}", env!("CARGO_PKG_VERSION"));
                println!("Rust-native MCP server for Office document processing");
                println!();
                println!("Usage: office-oxide-mcp [OPTIONS]");
                println!("Usage: office-oxide-mcp --verify <file>");
                println!();
                println!("Options:");
                println!("  --verify <file>     Verify OOXML structural integrity and exit");
                println!("  --transport <mode>  Transport mode (stdio only; accepted for MCP client compatibility)");
                println!("  --version, -V       Print version and exit");
                println!("  --help, -h          Print this help and exit");
                println!();
                println!("The server communicates over stdio using the MCP JSON-RPC protocol.");
                println!(
                    "Configure as an MCP server in your MCP client (Claude Desktop, Cursor, etc.)."
                );
                std::process::exit(0);
            }
            "--version" | "-V" => {
                println!("office-oxide-mcp v{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--verify" => {
                if args.len() < 3 {
                    eprintln!("error: --verify requires a file path argument");
                    std::process::exit(1);
                }
                let path = &args[2];
                match integrity::IntegrityValidator::verify(path) {
                    Ok(report) => {
                        println!("{}", serde_json::to_string_pretty(&report).unwrap());
                        std::process::exit(if report.passed { 0 } else { 1 });
                    }
                    Err(e) => {
                        eprintln!("error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "--transport" => {
                // Accept --transport stdio (or any mode) for MCP client compatibility
                if args.len() > 2 {
                    // mode is args[2], we accept all modes since we only support stdio
                }
            }
            _ => {
                eprintln!("error: unknown option '{}'", args[1]);
                eprintln!("Usage: office-oxide-mcp [--transport stdio] [--verify <file>] [--version] [--help]");
                std::process::exit(1);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    parse_args();

    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_writer(std::io::stderr)
        .init();

    let service = OfficeService::new();

    let io = (tokio::io::stdin(), tokio::io::stdout());
    let running = serve_server(service, io).await?;
    running
        .waiting()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(())
}
