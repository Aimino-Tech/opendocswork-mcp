use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordDocument {
    pub metadata: WordMetadata,
    pub outline: Vec<OutlineEntry>,
    pub content: Vec<ContentBlock>,
    pub images: Vec<ImageEntry>,
    pub links: Vec<LinkEntry>,
    pub comments: Vec<CommentEntry>,
    pub tracked_changes: Vec<TrackedChange>,
    pub markdown: String,
    pub plain_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordMetadata {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
    pub word_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineEntry {
    pub level: u32,
    pub text: String,
    pub children: Vec<OutlineEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "paragraph")]
    Paragraph {
        text: String,
        style: Option<String>,
        runs: Vec<RunInfo>,
    },
    #[serde(rename = "heading")]
    Heading { text: String, level: u32 },
    #[serde(rename = "list_item")]
    ListItem {
        text: String,
        level: u32,
        ordered: bool,
    },
    #[serde(rename = "table")]
    Table { rows: Vec<TableRow> },
    #[serde(rename = "image")]
    Image {
        alt_text: Option<String>,
        width_emu: i64,
        height_emu: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInfo {
    pub text: String,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strike: Option<bool>,
    pub size: Option<f64>,
    pub color: Option<String>,
    pub font: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEntry {
    pub name: Option<String>,
    pub alt_text: Option<String>,
    pub width_emu: i64,
    pub height_emu: i64,
    pub is_floating: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkEntry {
    pub text: String,
    pub url: Option<String>,
    pub anchor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentEntry {
    pub id: String,
    pub author: String,
    pub text: String,
    pub target_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedChange {
    pub text: String,
    pub is_insertion: bool,
    pub author: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub index: usize,
    pub heading: Option<String>,
    pub heading_level: Option<u32>,
    pub text: String,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub source: String,
    pub format: String,
    pub chunk_type: String,
}

/// Format a single run with markdown formatting (bold, italic, strike, underline)
fn format_run_text(run: &RunInfo) -> String {
    let text = &run.text;
    if text.is_empty() {
        return String::new();
    }

    let mut result = text.clone();

    if run.strike.unwrap_or(false) {
        result = format!("~~{}~~", result);
    }
    let bold = run.bold.unwrap_or(false);
    let italic = run.italic.unwrap_or(false);
    if bold && italic {
        result = format!("***{}***", result);
    } else if bold {
        result = format!("**{}**", result);
    } else if italic {
        result = format!("*{}*", result);
    }
    if run.underline.unwrap_or(false) {
        result = format!("<u>{}</u>", result);
    }

    result
}

/// Format a table as a proper markdown pipe table
fn format_table(rows: &[TableRow]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let max_cols = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return String::new();
    }

    let mut md = String::new();

    for (row_idx, row) in rows.iter().enumerate() {
        md.push('|');
        for cell in &row.cells {
            let clean = cell.replace('\n', " ");
            md.push_str(&format!(" {} |", clean));
        }
        for _ in row.cells.len()..max_cols {
            md.push_str(" |");
        }
        md.push('\n');

        if row_idx == 0 {
            md.push('|');
            for _ in 0..max_cols {
                md.push_str("---|");
            }
            md.push('\n');
        }
    }

    md
}

/// Generate proper markdown from structured content blocks
pub fn content_blocks_to_markdown(content: &[ContentBlock]) -> String {
    if content.is_empty() {
        return String::new();
    }

    let mut md = String::new();

    for block in content {
        match block {
            ContentBlock::Heading { text, level } => {
                let prefix = "#".repeat(*level as usize);
                md.push_str(&format!("{} {}\n\n", prefix, text));
            }
            ContentBlock::Paragraph {
                text: _text, runs, ..
            } => {
                if !runs.is_empty() {
                    for (i, run) in runs.iter().enumerate() {
                        if i > 0 && !run.text.starts_with(' ') && !runs[i - 1].text.ends_with(' ') {
                            md.push(' ');
                        }
                        md.push_str(&format_run_text(run));
                    }
                } else {
                    md.push_str(_text);
                }
                md.push_str("\n\n");
            }
            ContentBlock::ListItem {
                text,
                level,
                ordered,
            } => {
                let indent = "  ".repeat(*level as usize);
                let prefix = if *ordered { "1." } else { "-" };
                md.push_str(&format!("{}{} {}\n", indent, prefix, text));
            }
            ContentBlock::Table { rows } => {
                md.push_str(&format_table(rows));
                md.push('\n');
            }
            ContentBlock::Image { alt_text, .. } => {
                let alt = alt_text.as_deref().unwrap_or("image");
                md.push_str(&format!("![{}](image.png)\n\n", alt));
            }
        }
    }

    md.trim_end().to_string()
}

pub fn read_word_to_json(path: &str) -> Result<WordDocument, String> {
    let doc = open_document(path)?;

    let outline = extract_outline(&doc);
    let images = extract_images(&doc);
    let links = extract_links(&doc);
    let comments = extract_comments(path);
    let tracked_changes = extract_tracked_changes(path);
    let content = extract_content_blocks(path)?;
    let markdown = content_blocks_to_markdown(&content);
    let plain_text = {
        let text = extract_plain_text(&doc);
        if text.is_empty() {
            extract_plain_text_fallback(path)
        } else {
            text
        }
    };
    let metadata = extract_metadata(&doc);

    Ok(WordDocument {
        metadata,
        outline,
        content,
        images,
        links,
        comments,
        tracked_changes,
        markdown,
        plain_text,
    })
}

pub fn read_word_to_md(path: &str) -> Result<String, String> {
    let doc = open_document(path)?;
    let comments = extract_comments(path);
    let metadata = extract_metadata(&doc);

    let mut frontmatter = String::from("---\n");
    if !metadata.title.is_empty() {
        frontmatter.push_str(&format!("title: \"{}\"\n", metadata.title));
    }
    if !metadata.author.is_empty() {
        frontmatter.push_str(&format!("author: \"{}\"\n", metadata.author));
    }
    if !metadata.subject.is_empty() {
        frontmatter.push_str(&format!("subject: \"{}\"\n", metadata.subject));
    }
    if !metadata.keywords.is_empty() {
        frontmatter.push_str(&format!("keywords: [{}]\n", metadata.keywords));
    }
    if metadata.word_count > 0 {
        frontmatter.push_str(&format!("word_count: {}\n", metadata.word_count));
    }
    frontmatter.push_str("---\n\n");

    let mut md = frontmatter;
    let content = extract_content_blocks(path)?;
    md.push_str(&content_blocks_to_markdown(&content));

    if !comments.is_empty() {
        md.push_str("\n\n---\n\n## Comments\n\n");
        for comment in &comments {
            md.push_str(&format!("> **{}:** {}\n>\n", comment.author, comment.text));
        }
    }

    let tracked = extract_tracked_changes(path);
    if !tracked.is_empty() {
        md.push_str("\n\n---\n\n## Tracked Changes\n\n");
        for change in &tracked {
            if change.is_insertion {
                md.push_str(&format!("- <ins>{}</ins>\n", change.text));
            } else {
                md.push_str(&format!("- <del>{}</del>\n", change.text));
            }
        }
    }

    Ok(md)
}

pub fn read_word_to_chunks(path: &str) -> Result<Vec<Chunk>, String> {
    let md = read_word_to_md(path)?;
    let body = strip_yaml_frontmatter(&md);
    let chunks = split_markdown_into_chunks(body, path);
    Ok(chunks)
}

fn strip_yaml_frontmatter(md: &str) -> &str {
    let trimmed = md.trim_start();
    if !trimmed.starts_with("---") {
        return trimmed;
    }
    let after_first = trimmed.trim_start_matches("---").trim_start();
    if let Some(end_pos) = after_first.find("\n---") {
        let after_frontmatter = &after_first[end_pos + 4..];
        after_frontmatter.trim_start()
    } else {
        trimmed
    }
}

fn open_document(path: &str) -> Result<rdocx::Document, String> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "doc" => {
            let err_msg = format!("Failed to open DOC: {}", path);
            if let Ok(doc) = office_oxide::Document::open(path) {
                let md = doc.to_markdown();
                let mut rdoc = rdocx::Document::new();
                let lines: Vec<&str> = md.lines().collect();
                for line in lines {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if line.starts_with("# ") {
                        rdoc.add_paragraph(line.trim_start_matches("# ").trim())
                            .style("Heading1");
                    } else if line.starts_with("## ") {
                        rdoc.add_paragraph(line.trim_start_matches("## ").trim())
                            .style("Heading2");
                    } else {
                        rdoc.add_paragraph(line);
                    }
                }
                return Ok(rdoc);
            }
            Err(err_msg)
        }
        _ => rdocx::Document::open(path).map_err(|e| format!("Failed to open DOCX: {}", e)),
    }
}

fn extract_metadata(doc: &rdocx::Document) -> WordMetadata {
    WordMetadata {
        title: doc.title().unwrap_or("").to_string(),
        author: doc.author().unwrap_or("").to_string(),
        subject: doc.subject().unwrap_or("").to_string(),
        keywords: doc.keywords().unwrap_or("").to_string(),
        word_count: doc.word_count(),
    }
}

fn outline_node_to_entry(node: &rdocx::OutlineNode) -> OutlineEntry {
    OutlineEntry {
        level: node.level,
        text: node.text.clone(),
        children: node.children.iter().map(outline_node_to_entry).collect(),
    }
}

fn extract_outline(doc: &rdocx::Document) -> Vec<OutlineEntry> {
    doc.document_outline()
        .into_iter()
        .map(|n| outline_node_to_entry(&n))
        .collect()
}

fn extract_images(doc: &rdocx::Document) -> Vec<ImageEntry> {
    doc.images()
        .into_iter()
        .map(|img| ImageEntry {
            name: img.name,
            alt_text: img.description,
            width_emu: img.width_emu,
            height_emu: img.height_emu,
            is_floating: img.is_anchor,
        })
        .collect()
}

fn extract_links(doc: &rdocx::Document) -> Vec<LinkEntry> {
    doc.links()
        .into_iter()
        .map(|link| LinkEntry {
            text: link.text,
            url: link.url,
            anchor: link.anchor,
        })
        .collect()
}

fn extract_comments(path: &str) -> Vec<CommentEntry> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut comments_xml = match archive.by_name("word/comments.xml") {
        Ok(entry) => entry,
        Err(_) => return Vec::new(),
    };
    let mut xml = String::new();
    if comments_xml.read_to_string(&mut xml).is_err() || xml.is_empty() {
        return Vec::new();
    }

    let mut comments = Vec::new();
    let mut pos = 0;

    while let Some(start) = xml[pos..].find("<w:comment ") {
        let abs_start = pos + start;
        let author = extract_attr(&xml[abs_start..], "w:author=\"", "\"");
        let id = extract_attr(&xml[abs_start..], "w:id=\"", "\"");

        let content_start = if let Some(cs) = xml[abs_start..].find(">") {
            abs_start + cs + 1
        } else {
            break;
        };

        let end = if let Some(e) = xml[abs_start..].find("</w:comment>") {
            abs_start + e
        } else {
            break;
        };

        let inner = &xml[content_start..end];
        let mut text_buf = String::new();
        let mut tpos = 0;
        while let Some(ts) = inner[tpos..].find("<w:t") {
            let content_from = tpos + ts;
            let tc = if let Some(tc) = inner[content_from..].find(">") {
                content_from + tc + 1
            } else {
                break;
            };
            let te = if let Some(te) = inner[tc..].find("</w:t>") {
                tc + te
            } else {
                break;
            };
            if !text_buf.is_empty() {
                text_buf.push(' ');
            }
            text_buf.push_str(&inner[tc..te]);
            tpos = te + 6;
        }

        comments.push(CommentEntry {
            id,
            author,
            text: text_buf,
            target_text: None,
        });

        pos = abs_start + 1;
    }

    comments
}

fn extract_attr(xml: &str, key: &str, term: &str) -> String {
    if let Some(start) = xml.find(key) {
        let val_start = start + key.len();
        if let Some(end) = xml[val_start..].find(term) {
            return xml[val_start..val_start + end].to_string();
        }
    }
    String::new()
}

fn extract_tracked_changes(path: &str) -> Vec<TrackedChange> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return Vec::new(),
    };
    let mut doc_xml = match archive.by_name("word/document.xml") {
        Ok(entry) => entry,
        Err(_) => return Vec::new(),
    };
    let mut xml = String::new();
    if doc_xml.read_to_string(&mut xml).is_err() || xml.is_empty() {
        return Vec::new();
    }

    let mut changes = Vec::new();
    for tag in &["<w:ins ", "<w:del "] {
        let is_insertion = *tag == "<w:ins ";
        let close_tag = if is_insertion { "</w:ins>" } else { "</w:del>" };
        let mut pos = 0;
        while let Some(start) = xml[pos..].find(tag) {
            let abs_start = pos + start;
            let author = extract_attr(&xml[abs_start..], "w:author=\"", "\"");
            let date = extract_attr(&xml[abs_start..], "w:date=\"", "\"");

            let content_start = if let Some(cs) = xml[abs_start..].find(">") {
                abs_start + cs + 1
            } else {
                break;
            };
            let end = if let Some(e) = xml[abs_start..].find(close_tag) {
                abs_start + e
            } else {
                break;
            };

            let inner = &xml[content_start..end];
            let mut text_buf = String::new();
            let mut tpos = 0;
            while let Some(ts) = inner[tpos..].find("<w:t") {
                let from = tpos + ts;
                let tc = if let Some(tc) = inner[from..].find(">") {
                    from + tc + 1
                } else {
                    break;
                };
                let te = if let Some(te) = inner[tc..].find("</w:t>") {
                    tc + te
                } else {
                    break;
                };
                if !text_buf.is_empty() {
                    text_buf.push(' ');
                }
                text_buf.push_str(&inner[tc..te]);
                tpos = te + 6;
            }
            let mut dpos = 0;
            while let Some(ds) = inner[dpos..].find("<w:delText") {
                let from = dpos + ds;
                let dc = if let Some(dc) = inner[from..].find(">") {
                    from + dc + 1
                } else {
                    break;
                };
                let de = if let Some(de) = inner[dc..].find("</w:delText>") {
                    dc + de
                } else {
                    break;
                };
                if !text_buf.is_empty() {
                    text_buf.push(' ');
                }
                text_buf.push_str(&inner[dc..de]);
                dpos = de + 12;
            }

            if !text_buf.is_empty() {
                changes.push(TrackedChange {
                    text: text_buf,
                    is_insertion,
                    author: if author.is_empty() {
                        None
                    } else {
                        Some(author)
                    },
                    date: if date.is_empty() { None } else { Some(date) },
                });
            }
            pos = abs_start + 1;
        }
    }
    changes
}

fn extract_plain_text(doc: &rdocx::Document) -> String {
    let mut text = String::new();
    for para in doc.paragraphs() {
        let t = para.text();
        if !t.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&t);
        }
    }
    text
}

fn extract_plain_text_fallback(path: &str) -> String {
    if let Ok(oxide_doc) = office_oxide::Document::open(path) {
        return oxide_doc.plain_text();
    }
    if let Ok(rdoc_doc) = rdocx::Document::open(path) {
        let mut text = String::new();
        for para in rdoc_doc.paragraphs() {
            let t = para.text();
            if !t.is_empty() {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&t);
            }
        }
        return text;
    }
    String::new()
}

fn is_list_style(style_id: Option<&str>) -> Option<(u32, bool)> {
    let s = style_id?;
    let lower = s.to_lowercase();
    if lower.starts_with("listbullet")
        || lower.starts_with("listnumber")
        || lower.starts_with("listparagraph")
    {
        let level = s.chars().last().and_then(|c| c.to_digit(10)).unwrap_or(0);
        let ordered = lower.starts_with("listnumber");
        return Some((level, ordered));
    }
    None
}

fn extract_run_info(para: &rdocx::ParagraphRef) -> Vec<RunInfo> {
    para.runs()
        .map(|run| RunInfo {
            text: run.text(),
            bold: Some(run.is_bold()),
            italic: Some(run.is_italic()),
            underline: None,
            strike: Some(run.is_strike()),
            size: run.size(),
            color: run.color().map(|s| s.to_string()),
            font: run.font_name().map(|s| s.to_string()),
        })
        .collect()
}

fn detect_list_item(text: &str) -> Option<(u32, bool)> {
    let trimmed = text.trim_start();
    if trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("+ ")
        || trimmed.starts_with("\u{2022} ")
    {
        return Some((0, false));
    }
    if trimmed.starts_with("o ") || trimmed.starts_with("\u{25E6} ") {
        return Some((1, false));
    }
    if trimmed.starts_with("\u{25CB} ") {
        return Some((2, false));
    }
    if trimmed.starts_with("\u{25A0} ") {
        return Some((3, false));
    }
    if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
        if let Some(rest) = rest.strip_prefix(|c: char| c == '.' || c == ')') {
            if rest.starts_with(' ') {
                return Some((0, true));
            }
        }
    }
    if let Some(rest) = trimmed.strip_prefix(|c: char| c == '(') {
        if let Some(rest) = rest.strip_prefix(|c: char| c.is_ascii_digit()) {
            if rest.starts_with(')') {
                return Some((0, true));
            }
        }
    }
    None
}

fn extract_content_blocks(path: &str) -> Result<Vec<ContentBlock>, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let doc_xml = read_docx_xml(path)?;
    let mut reader = Reader::from_str(&doc_xml);
    reader.config_mut().trim_text(true);

    let mut blocks: Vec<ContentBlock> = Vec::new();

    // Depth tracking for each element type
    // We track depth to properly handle nested elements (e.g. paragraphs inside table cells)
    let mut in_body = false;
    let mut para_depth: u32 = 0;
    let mut run_depth: u32 = 0;
    let mut rpr_depth: u32 = 0;
    let mut ppr_depth: u32 = 0;
    let mut tbl_depth: u32 = 0;
    let mut tr_depth: u32 = 0;
    let mut tc_depth: u32 = 0;

    // Per-paragraph accumulators
    let mut para_pstyle = String::new();
    let mut para_has_numpr = false;
    let mut para_has_drawing = false;
    let mut para_runs: Vec<RunInfo> = Vec::new();
    let mut para_text = String::new();

    // Per-run accumulators
    let mut run_text = String::new();
    let mut run_bold = false;
    let mut run_italic = false;
    let mut run_strike = false;
    let mut run_underline = false;

    // Per-table accumulators
    let mut table_rows: Vec<TableRow> = Vec::new();
    let mut table_cells: Vec<String> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag == "w:body" {
                    in_body = true;
                    continue;
                }
                if tag == "w:p" && in_body {
                    if para_depth == 0 {
                        para_pstyle.clear();
                        para_has_numpr = false;
                        para_has_drawing = false;
                        para_runs.clear();
                        para_text.clear();
                        run_text.clear();
                        run_bold = false;
                        run_italic = false;
                        run_strike = false;
                        run_underline = false;
                    }
                    para_depth += 1;
                    continue;
                }
                if tag == "w:pPr" && para_depth > 0 {
                    ppr_depth += 1;
                    continue;
                }
                if tag == "w:numPr" && ppr_depth > 0 {
                    para_has_numpr = true;
                }
                if tag == "w:pStyle" && ppr_depth > 0 {
                    para_pstyle = e
                        .try_get_attribute("w:val")
                        .ok()
                        .flatten()
                        .and_then(|attr| {
                            let s = std::str::from_utf8(&attr.value).ok()?;
                            if s.is_empty() {
                                None
                            } else {
                                Some(s.to_string())
                            }
                        })
                        .or_else(|| {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"val"
                                    || attr.key.as_ref().ends_with(b"val")
                                {
                                    return std::str::from_utf8(&attr.value)
                                        .ok()
                                        .map(|s| s.to_string());
                                }
                            }
                            None
                        })
                        .unwrap_or_default();
                    continue;
                }
                if tag == "w:r" && para_depth > 0 {
                    run_depth += 1;
                    if run_depth == 1 {
                        run_text.clear();
                        run_bold = false;
                        run_italic = false;
                        run_strike = false;
                        run_underline = false;
                    }
                    continue;
                }
                if tag == "w:rPr" && run_depth > 0 {
                    rpr_depth += 1;
                    continue;
                }
                if tag == "w:b" && rpr_depth > 0 {
                    run_bold = true;
                }
                if tag == "w:i" && rpr_depth > 0 {
                    run_italic = true;
                }
                if tag == "w:strike" && rpr_depth > 0 {
                    run_strike = true;
                }
                if tag == "w:u" && rpr_depth > 0 {
                    run_underline = true;
                }
                if tag == "w:drawing" && run_depth > 0 {
                    para_has_drawing = true;
                }
                if tag == "w:tbl" && in_body {
                    if tbl_depth == 0 {
                        table_rows.clear();
                    }
                    tbl_depth += 1;
                    continue;
                }
                if tag == "w:tr" && tbl_depth > 0 {
                    if tr_depth == 0 {
                        table_cells.clear();
                    }
                    tr_depth += 1;
                    continue;
                }
                if tag == "w:tc" && tr_depth > 0 {
                    if tc_depth == 0 {
                        table_cells.push(String::new());
                    }
                    tc_depth += 1;
                    continue;
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag == "w:body" {
                    in_body = false;
                    continue;
                }
                if tag == "w:p" && para_depth > 0 {
                    para_depth -= 1;
                    if para_depth > 0 {
                        continue;
                    }

                    // Skip paragraph blocks inside table cells (text collected via table path)
                    if tc_depth > 0 {
                        continue;
                    }

                    // Finalize paragraph block
                    let text = para_text.trim().to_string();
                    if !text.is_empty() || !para_runs.is_empty() || para_has_drawing {
                        let heading_level = para_pstyle
                            .strip_prefix("Heading")
                            .or_else(|| para_pstyle.strip_prefix("heading"))
                            .and_then(|n| n.parse::<u32>().ok())
                            .or_else(|| {
                                if para_pstyle.eq_ignore_ascii_case("title") {
                                    Some(1u32)
                                } else if para_pstyle.eq_ignore_ascii_case("subtitle")
                                    || para_pstyle.eq_ignore_ascii_case("heading")
                                {
                                    Some(2u32)
                                } else {
                                    None
                                }
                            });

                        if let Some(lvl) = heading_level {
                            blocks.push(ContentBlock::Heading {
                                text: text.clone(),
                                level: lvl,
                            });
                        } else if para_has_drawing {
                            let alt = if text.is_empty() { "image" } else { &text };
                            blocks.push(ContentBlock::Image {
                                alt_text: Some(alt.to_string()),
                                width_emu: 0,
                                height_emu: 0,
                            });
                        } else if para_has_numpr {
                            let list_level = para_pstyle
                                .chars()
                                .last()
                                .and_then(|c| c.to_digit(10))
                                .unwrap_or(0);
                            let ordered = para_pstyle.to_lowercase().starts_with("listnumber");
                            blocks.push(ContentBlock::ListItem {
                                text: text.clone(),
                                level: list_level,
                                ordered,
                            });
                        } else {
                            blocks.push(ContentBlock::Paragraph {
                                text: text.clone(),
                                style: if para_pstyle.is_empty() {
                                    None
                                } else {
                                    Some(para_pstyle.clone())
                                },
                                runs: para_runs.clone(),
                            });
                        }
                    }
                    continue;
                }
                if tag == "w:pPr" && ppr_depth > 0 {
                    ppr_depth -= 1;
                    continue;
                }
                if tag == "w:r" && run_depth > 0 {
                    run_depth -= 1;
                    if run_depth > 0 {
                        continue;
                    }

                    if !run_text.is_empty() {
                        para_runs.push(RunInfo {
                            text: run_text.clone(),
                            bold: if run_bold { Some(true) } else { None },
                            italic: if run_italic { Some(true) } else { None },
                            underline: if run_underline { Some(true) } else { None },
                            strike: if run_strike { Some(true) } else { None },
                            size: None,
                            color: None,
                            font: None,
                        });
                        // Add space between runs if the previous run's text
                        // doesn't already end with whitespace
                        if !para_text.is_empty()
                            && !para_text.ends_with(' ')
                            && !run_text.starts_with(' ')
                        {
                            para_text.push(' ');
                        }
                        para_text.push_str(&run_text);
                    }
                    continue;
                }
                if tag == "w:rPr" && rpr_depth > 0 {
                    rpr_depth -= 1;
                    continue;
                }
                if tag == "w:tbl" && tbl_depth > 0 {
                    tbl_depth -= 1;
                    if tbl_depth > 0 {
                        continue;
                    }
                    if !table_rows.is_empty() {
                        blocks.push(ContentBlock::Table {
                            rows: std::mem::take(&mut table_rows),
                        });
                    }
                    continue;
                }
                if tag == "w:tr" && tr_depth > 0 {
                    tr_depth -= 1;
                    if tr_depth > 0 {
                        continue;
                    }
                    if !table_cells.is_empty() {
                        table_rows.push(TableRow {
                            cells: std::mem::take(&mut table_cells),
                        });
                    }
                    continue;
                }
                if tag == "w:tc" && tc_depth > 0 {
                    tc_depth -= 1;
                    continue;
                }
            }
            Ok(Event::Text(ref e)) => {
                if run_depth > 0 && rpr_depth == 0 {
                    if let Ok(text) = e.unescape() {
                        run_text.push_str(&text);
                    }
                }
                // Collect text for table cells (paragraph text inside cells)
                if tc_depth > 0 {
                    if let Ok(text) = e.unescape() {
                        if let Some(last) = table_cells.last_mut() {
                            last.push_str(&text);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
    }

    Ok(blocks)
}

/// Read the document.xml content from a docx file
fn read_docx_xml(path: &str) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut doc_xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|_| "No word/document.xml found in docx".to_string())?
        .read_to_string(&mut doc_xml)
        .map_err(|e| format!("Failed to read document.xml: {}", e))?;
    Ok(doc_xml)
}

fn split_markdown_into_chunks(markdown: &str, source: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut current_text = String::new();
    let mut current_heading: Option<String> = None;
    let mut current_heading_level: Option<u32> = None;
    let mut chunk_index = 0;

    for line in markdown.lines() {
        if line.starts_with('#') {
            if !current_text.trim().is_empty() {
                chunks.push(Chunk {
                    index: chunk_index,
                    heading: current_heading.clone(),
                    heading_level: current_heading_level,
                    text: current_text.trim().to_string(),
                    metadata: ChunkMetadata {
                        source: source.to_string(),
                        format: "docx".to_string(),
                        chunk_type: "section".to_string(),
                    },
                });
                chunk_index += 1;
            }

            let level = line.chars().take_while(|&c| c == '#').count() as u32;
            current_heading = Some(line.trim_start_matches('#').trim().to_string());
            current_heading_level = Some(level);
            current_text = String::new();
        } else {
            if !current_text.is_empty() {
                current_text.push('\n');
            }
            current_text.push_str(line);
        }
    }

    if !current_text.trim().is_empty() {
        chunks.push(Chunk {
            index: chunk_index,
            heading: current_heading.clone(),
            heading_level: current_heading_level,
            text: current_text.trim().to_string(),
            metadata: ChunkMetadata {
                source: source.to_string(),
                format: "docx".to_string(),
                chunk_type: "section".to_string(),
            },
        });
    }

    if chunks.is_empty() && !markdown.trim().is_empty() {
        chunks.push(Chunk {
            index: 0,
            heading: None,
            heading_level: None,
            text: markdown.trim().to_string(),
            metadata: ChunkMetadata {
                source: source.to_string(),
                format: "docx".to_string(),
                chunk_type: "full".to_string(),
            },
        });
    }

    chunks
}
