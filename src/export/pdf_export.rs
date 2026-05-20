use calamine::Reader as CalamineReader;
use printpdf::*;
use std::io::Write;
use std::path::Path;

const PAGE_WIDTH_MM: f32 = 210.0;
const PAGE_HEIGHT_MM: f32 = 297.0;
const MARGIN_MM: f32 = 20.0;
const BODY_FONT_SIZE: f32 = 10.0;
const TITLE_FONT_SIZE: f32 = 18.0;
const LINE_HEIGHT_MM: f32 = 5.0;
const MAX_LINE_CHARS: usize = 90;

pub struct PdfExport;

impl PdfExport {
    pub fn export_docx_to_pdf(input_path: &str, output_path: &str) -> Result<String, String> {
        let file_path = Path::new(input_path);
        if !file_path.exists() {
            return Err(format!("File not found: {}", input_path));
        }

        let doc = rdocx::Document::open(input_path)
            .map_err(|e| format!("Failed to open DOCX: {}", e))?;

        let mut text_lines: Vec<String> = Vec::new();
        let mut empty_count = 0;

        for para in doc.paragraphs() {
            let t = para.text().trim().to_string();
            if t.is_empty() {
                empty_count += 1;
                if empty_count <= 2 {
                    text_lines.push(String::new());
                }
            } else {
                empty_count = 0;
                text_lines.push(t);
            }
        }

        if text_lines.is_empty() {
            text_lines.push("(empty document)".to_string());
        }

        Self::render_text_to_pdf(&text_lines, output_path, "DOCX Export")
    }

    pub fn export_xlsx_to_pdf(input_path: &str, output_path: &str) -> Result<String, String> {
        let file_path = Path::new(input_path);
        if !file_path.exists() {
            return Err(format!("File not found: {}", input_path));
        }

        let mut workbook: calamine::Xlsx<std::io::BufReader<std::fs::File>> =
            calamine::open_workbook(input_path)
                .map_err(|e| format!("Failed to open XLSX: {}", e))?;

        let sheet_names = workbook.sheet_names().to_vec();
        if sheet_names.is_empty() {
            return Err("XLSX has no sheets".to_string());
        }

        let mut doc = PdfDocument::new("XLSX Export");
        let mut all_pages = Vec::new();

        for (si, name) in sheet_names.iter().enumerate() {
            if si > 0 {
                let sep_ops = vec![
                    Op::StartTextSection,
                    Op::SetTextCursor { pos: Point::new(Mm(40.0_f32), Mm(PAGE_HEIGHT_MM / 2.0_f32)) },
                    Op::SetFontSizeBuiltinFont { size: Pt(16.0_f32), font: BuiltinFont::Helvetica },
                    Op::SetLineHeight { lh: Pt(16.0_f32) },
                    Op::WriteTextBuiltinFont {
                        items: vec![TextItem::Text("--- Next Sheet ---".to_string())],
                        font: BuiltinFont::Helvetica,
                    },
                    Op::EndTextSection,
                ];
                all_pages.push(PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), sep_ops));
            }

            let range = workbook
                .worksheet_range(name)
                .map_err(|e| format!("Failed to read sheet '{}': {}", name, e))?;

            let mut ops = Vec::new();
            let mut cursor_y = PAGE_HEIGHT_MM - MARGIN_MM;

            ops.extend(Self::make_title_line(&format!("Sheet: {}", name), &mut cursor_y));

            let mut first_row = true;
            let rows: Vec<Vec<String>> = range
                .rows()
                .map(|r| {
                    r.iter()
                        .map(|c| match c {
                            calamine::Data::String(s) => s.clone(),
                            calamine::Data::Float(f) => format!("{}", f),
                            calamine::Data::Int(i) => format!("{}", i),
                            calamine::Data::Bool(b) => format!("{}", b),
                            calamine::Data::DateTime(dt) => dt.to_string(),
                            calamine::Data::DateTimeIso(s) => s.clone(),
                            calamine::Data::DurationIso(s) => s.clone(),
                            calamine::Data::Error(e) => format!("#{}", e),
                            calamine::Data::Empty => String::new(),
                        })
                        .collect()
                })
                .collect();

            for (_ri, row) in rows.iter().enumerate() {
                if cursor_y < MARGIN_MM + 10.0_f32 {
                    let page = PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops);
                    all_pages.push(page);
                    ops = Vec::new();
                    cursor_y = PAGE_HEIGHT_MM - MARGIN_MM;
                    ops.extend(Self::make_title_line(
                        &format!("Sheet: {} (cont.)", name),
                        &mut cursor_y,
                    ));
                }

                let line = if first_row {
                    format!("  {}", row.join("  |  "))
                } else {
                    row.join("  |  ")
                };

                let wrapped = Self::word_wrap(&line, MAX_LINE_CHARS);
                for (wi, wline) in wrapped.iter().enumerate() {
                    if cursor_y < MARGIN_MM + 5.0_f32 {
                        let page = PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops);
                        all_pages.push(page);
                        ops = Vec::new();
                        cursor_y = PAGE_HEIGHT_MM - MARGIN_MM;
                    }
                    let is_header = first_row && wi == 0;
                    ops.extend(Self::make_text_line(
                        wline,
                        Mm(MARGIN_MM),
                        Mm(cursor_y),
                        if is_header { Pt(11.0_f32) } else { Pt(BODY_FONT_SIZE) },
                        if is_header {
                            Color::Rgb(Rgb { r: 1.0, g: 1.0, b: 1.0, icc_profile: None })
                        } else {
                            Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None })
                        },
                        if is_header { BuiltinFont::HelveticaBold } else { BuiltinFont::Helvetica },
                    ));
                    cursor_y -= LINE_HEIGHT_MM;
                }

                if first_row {
                    cursor_y -= 2.0_f32;
                }
                first_row = false;
            }

            let page = PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops);
            all_pages.push(page);
        }

        doc.with_pages(all_pages);
        Self::save_pdf(&mut doc, output_path)?;

        Ok(serde_json::json!({"status": "success", "output_path": output_path}).to_string())
    }

    pub fn export_pptx_to_pdf(input_path: &str, output_path: &str) -> Result<String, String> {
        let file_path = Path::new(input_path);
        if !file_path.exists() {
            return Err(format!("File not found: {}", input_path));
        }

        let doc_pptx = office_oxide::pptx::PptxDocument::open(input_path)
            .map_err(|e| format!("Failed to open PPTX: {}", e))?;

        let mut doc = PdfDocument::new("PPTX Export");
        let mut all_pages = Vec::new();

        for (i, slide) in doc_pptx.slides.iter().enumerate() {
            let mut ops = Vec::new();
            let mut cursor_y = PAGE_HEIGHT_MM - MARGIN_MM;

            let slide_title = if slide.name.is_empty() {
                format!("Slide {}", i + 1)
            } else {
                format!("Slide {}: {}", i + 1, slide.name)
            };
            ops.extend(Self::make_title_line(&slide_title, &mut cursor_y));
            cursor_y -= 3.0_f32;

            use office_oxide::pptx::shape::{Shape, TextContent};
            for shape in &slide.shapes {
                if let Shape::AutoShape(a) = shape {
                    if cursor_y < MARGIN_MM + 5.0_f32 {
                        let page = PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops);
                        all_pages.push(page);
                        ops = Vec::new();
                        cursor_y = PAGE_HEIGHT_MM - MARGIN_MM;
                        ops.extend(Self::make_title_line(
                            &format!("{} (cont.)", slide_title),
                            &mut cursor_y,
                        ));
                        cursor_y -= 3.0_f32;
                    }

                    let mut slide_text = String::new();
                    if let Some(tb) = &a.text_body {
                        for (pi, p) in tb.paragraphs.iter().enumerate() {
                            if pi > 0 {
                                slide_text.push('\n');
                            }
                            for c in &p.content {
                                if let TextContent::Run(r) = c {
                                    slide_text.push_str(&r.text);
                                }
                            }
                        }
                    }

                    if !slide_text.is_empty() {
                        let wrapped = Self::word_wrap(&slide_text, MAX_LINE_CHARS);
                        for wline in &wrapped {
                            if cursor_y < MARGIN_MM + 5.0_f32 {
                                let page = PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops);
                                all_pages.push(page);
                                ops = Vec::new();
                                cursor_y = PAGE_HEIGHT_MM - MARGIN_MM;
                            }
                            ops.extend(Self::make_text_line(
                                wline,
                                Mm(MARGIN_MM + 5.0_f32),
                                Mm(cursor_y),
                                Pt(BODY_FONT_SIZE),
                                Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }),
                                BuiltinFont::Helvetica,
                            ));
                            cursor_y -= LINE_HEIGHT_MM;
                        }
                        cursor_y -= 2.0_f32;
                    }
                }
            }

            if let Some(notes) = &slide.notes {
                if cursor_y < MARGIN_MM + 15.0_f32 {
                    let page = PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops);
                    all_pages.push(page);
                    ops = Vec::new();
                    cursor_y = PAGE_HEIGHT_MM - MARGIN_MM;
                }
                cursor_y -= 3.0_f32;
                ops.extend(Self::make_text_line(
                    "Speaker Notes:",
                    Mm(MARGIN_MM),
                    Mm(cursor_y),
                    Pt(9.0_f32),
                    Color::Rgb(Rgb { r: 0.4, g: 0.4, b: 0.4, icc_profile: None }),
                    BuiltinFont::HelveticaOblique,
                ));
                cursor_y -= LINE_HEIGHT_MM;

                let wrapped = Self::word_wrap(notes, MAX_LINE_CHARS);
                for wline in &wrapped {
                    if cursor_y < MARGIN_MM + 5.0_f32 {
                        let page = PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops);
                        all_pages.push(page);
                        ops = Vec::new();
                        cursor_y = PAGE_HEIGHT_MM - MARGIN_MM;
                    }
                    ops.extend(Self::make_text_line(
                        wline,
                        Mm(MARGIN_MM + 5.0_f32),
                        Mm(cursor_y),
                        Pt(9.0_f32),
                        Color::Rgb(Rgb { r: 0.4, g: 0.4, b: 0.4, icc_profile: None }),
                        BuiltinFont::HelveticaOblique,
                    ));
                    cursor_y -= LINE_HEIGHT_MM;
                }
            }

            let page = PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops);
            all_pages.push(page);
        }

        doc.with_pages(all_pages);
        Self::save_pdf(&mut doc, output_path)?;

        Ok(serde_json::json!({"status": "success", "output_path": output_path}).to_string())
    }

    pub fn export_to_pdf(input_path: &str, output_path: &str) -> Result<String, String> {
        let file_path = Path::new(input_path);
        if !file_path.exists() {
            return Err(format!("File not found: {}", input_path));
        }

        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "docx" | "doc" => Self::export_docx_to_pdf(input_path, output_path),
            "xlsx" | "xls" => Self::export_xlsx_to_pdf(input_path, output_path),
            "pptx" | "ppt" => Self::export_pptx_to_pdf(input_path, output_path),
            other => Err(format!(
                "Unsupported format: '{}'. Supported: docx, xlsx, pptx",
                other
            )),
        }
    }

    // ── Private helpers ──────────────────────────────────

    fn word_wrap(text: &str, max_chars: usize) -> Vec<String> {
        let mut result = Vec::new();
        for line in text.lines() {
            if line.len() <= max_chars {
                result.push(line.to_string());
            } else {
                let mut start = 0;
                while start < line.len() {
                    let end = std::cmp::min(start + max_chars, line.len());
                    if end < line.len() {
                        if let Some(space) = line[start..end].rfind(' ') {
                            let break_at = start + space + 1;
                            result.push(line[start..break_at].trim_end().to_string());
                            start = break_at;
                        } else {
                            result.push(line[start..end].to_string());
                            start = end;
                        }
                    } else {
                        result.push(line[start..].to_string());
                        break;
                    }
                }
            }
        }
        if result.is_empty() {
            result.push(String::new());
        }
        result
    }

    fn make_title_line(title: &str, cursor_y: &mut f32) -> Vec<Op> {
        let mut ops = Vec::new();
        ops.push(Op::SetFillColor {
            col: Color::Rgb(Rgb { r: 0.15, g: 0.35, b: 0.65, icc_profile: None }),
        });
        let y_top = Mm(*cursor_y + 4.0_f32);
        let y_bot = Mm(*cursor_y - 2.0_f32);
        ops.push(Op::DrawPolygon {
            polygon: Polygon {
                rings: vec![PolygonRing {
                    points: vec![
                        LinePoint { p: Point::new(Mm(0.0_f32), y_top), bezier: false },
                        LinePoint { p: Point::new(Mm(PAGE_WIDTH_MM), y_top), bezier: false },
                        LinePoint { p: Point::new(Mm(PAGE_WIDTH_MM), y_bot), bezier: false },
                        LinePoint { p: Point::new(Mm(0.0_f32), y_bot), bezier: false },
                    ],
                }],
                mode: PaintMode::Fill,
                winding_order: WindingOrder::NonZero,
            },
        });

        ops.push(Op::StartTextSection);
        ops.push(Op::SetTextCursor { pos: Point::new(Mm(MARGIN_MM), Mm(*cursor_y)) });
        ops.push(Op::SetFontSizeBuiltinFont { size: Pt(TITLE_FONT_SIZE), font: BuiltinFont::HelveticaBold });
        ops.push(Op::SetLineHeight { lh: Pt(TITLE_FONT_SIZE) });
        ops.push(Op::SetFillColor {
            col: Color::Rgb(Rgb { r: 1.0, g: 1.0, b: 1.0, icc_profile: None }),
        });
        ops.push(Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text(title.to_string())],
            font: BuiltinFont::HelveticaBold,
        });
        ops.push(Op::EndTextSection);

        *cursor_y -= 10.0_f32;
        ops
    }

    #[allow(clippy::too_many_arguments)]
    fn make_text_line(
        text: &str,
        x: Mm,
        y: Mm,
        font_size: Pt,
        color: Color,
        font: BuiltinFont,
    ) -> Vec<Op> {
        vec![
            Op::StartTextSection,
            Op::SetTextCursor { pos: Point::new(x, y) },
            Op::SetFontSizeBuiltinFont { size: font_size, font: font.clone() },
            Op::SetLineHeight { lh: font_size },
            Op::SetFillColor { col: color },
            Op::WriteTextBuiltinFont {
                items: vec![TextItem::Text(text.to_string())],
                font,
            },
            Op::EndTextSection,
        ]
    }

    fn render_text_to_pdf(
        lines: &[String],
        output_path: &str,
        title: &str,
    ) -> Result<String, String> {
        let mut doc = PdfDocument::new(title);
        let mut all_pages = Vec::new();

        let page_count = if lines.len() <= 1 {
            1
        } else {
            (lines.len() + 45) / 46
        };

        let lines_per_page = if lines.len() <= 46 {
            lines.len()
        } else {
            46
        };

        let mut page_start = 0;
        for pi in 0..page_count {
            let mut ops = Vec::new();
            let mut cursor_y = PAGE_HEIGHT_MM - MARGIN_MM;

            let page_title = if pi == 0 {
                title.to_string()
            } else {
                format!("{} (page {})", title, pi + 1)
            };
            ops.extend(Self::make_title_line(&page_title, &mut cursor_y));

            let end = std::cmp::min(page_start + lines_per_page, lines.len());
            for line in &lines[page_start..end] {
                if cursor_y < MARGIN_MM + 5.0_f32 {
                    break;
                }
                let wrapped = Self::word_wrap(line, MAX_LINE_CHARS);
                for wline in &wrapped {
                    if cursor_y < MARGIN_MM + 5.0_f32 {
                        break;
                    }
                    ops.extend(Self::make_text_line(
                        wline,
                        Mm(MARGIN_MM),
                        Mm(cursor_y),
                        Pt(BODY_FONT_SIZE),
                        Color::Rgb(Rgb { r: 0.0, g: 0.0, b: 0.0, icc_profile: None }),
                        BuiltinFont::Helvetica,
                    ));
                    cursor_y -= LINE_HEIGHT_MM;
                }
            }

            page_start += lines_per_page;
            let page = PdfPage::new(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), ops);
            all_pages.push(page);
        }

        doc.with_pages(all_pages);
        Self::save_pdf(&mut doc, output_path)?;

        Ok(serde_json::json!({"status": "success", "output_path": output_path}).to_string())
    }

    fn save_pdf(doc: &mut PdfDocument, output_path: &str) -> Result<(), String> {
        let bytes = doc.save(&PdfSaveOptions::default(), &mut Vec::new());
        let mut file = std::fs::File::create(output_path)
            .map_err(|e| format!("Failed to create output file: {}", e))?;
        file.write_all(&bytes)
            .map_err(|e| format!("Failed to write PDF: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_wrap_short() {
        let result = PdfExport::word_wrap("hello world", 80);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "hello world");
    }

    #[test]
    fn test_word_wrap_long() {
        let text = "a".repeat(100);
        let result = PdfExport::word_wrap(&text, 50);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 50);
        assert_eq!(result[1].len(), 50);
    }

    #[test]
    fn test_word_wrap_empty() {
        let result = PdfExport::word_wrap("", 80);
        assert_eq!(result.len(), 1);
        assert!(result[0].is_empty());
    }

    #[test]
    fn test_export_missing_file() {
        let result = PdfExport::export_to_pdf("/nonexistent/file.docx", "/tmp/out.pdf");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("File not found"));
    }

    #[test]
    fn test_export_unsupported_format() {
        let tmp = std::env::temp_dir().join("test_unsupported.xyz");
        std::fs::write(&tmp, b"dummy").ok();
        let result = PdfExport::export_to_pdf(tmp.to_str().unwrap(), "/tmp/out.pdf");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unsupported format") || err.contains("unsupported"));
        let _ = std::fs::remove_file(&tmp);
    }
}
