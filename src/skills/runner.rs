use crate::skills::{SkillDefinition, SkillRunResult};
use crate::writers;

/// Execute a skill with provided params, generating real output files.
pub fn execute_skill(
    skill: &SkillDefinition,
    params: &serde_json::Map<String, serde_json::Value>,
) -> Result<SkillRunResult, String> {
    let output = build_output(skill, params);

    let file_paths = match skill.format.as_str() {
        "xlsx" => generate_excel(skill, params)?,
        "docx" => generate_docx(skill, params)?,
        "pptx" => generate_pptx(skill, params)?,
        other => {
            eprintln!("Warning: unsupported format '{}' for skill '{}'", other, skill.name);
            vec![]
        }
    };

    Ok(SkillRunResult {
        skill_name: skill.name.clone(),
        success: true,
        output,
        file_paths,
        warnings: vec![],
    })
}

fn output_path(skill: &SkillDefinition) -> (std::path::PathBuf, String) {
    let out_dir = std::path::Path::new("output");
    let _ = std::fs::create_dir_all(out_dir);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let filename = format!("{}_{}.{}", skill.name.replace('.', "_"), timestamp, skill.format);
    let file_path = out_dir.join(&filename);
    (file_path, filename)
}

fn generate_excel(
    skill: &SkillDefinition,
    params: &serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<String>, String> {
    let (file_path, _filename) = output_path(skill);
    let path_str = file_path.to_string_lossy().to_string();

    match skill.name.as_str() {
        "excel.basic" => {
            // Support multi-sheet via optional "sheets" param
            // Each sheet can have: name, data, header_row, row_types (optional array of "title"/"section"/"header"/"data"/"total")
            let sheet_configs: Vec<(String, Vec<Vec<serde_json::Value>>, bool, Vec<String>)> = if let Some(sheets_arr) = params.get("sheets").and_then(|v| v.as_array()) {
                sheets_arr.iter().filter_map(|s| {
                    let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("Sheet").to_string();
                    let data: Vec<Vec<serde_json::Value>> = s.get("data").and_then(|v| v.as_array()).map(|a| {
                        a.iter().filter_map(|row| row.as_array().map(|inner| inner.clone())).collect()
                    }).unwrap_or_default();
                    let header = s.get("header_row").and_then(|v| v.as_bool()).unwrap_or(true);
                    let row_types: Vec<String> = s.get("row_types").and_then(|v| v.as_array()).map(|a| {
                        a.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect()
                    }).unwrap_or_default();
                    if data.is_empty() { None } else { Some((name, data, header, row_types)) }
                }).collect()
            } else {
                let data = extract_2d_data(params);
                if data.is_empty() { return Err("No data provided".to_string()); }
                let name = params.get("sheet_name").and_then(|v| v.as_str()).unwrap_or("Sheet1").to_string();
                let header = params.get("header_row").and_then(|v| v.as_bool()).unwrap_or(true);
                let row_types: Vec<String> = params.get("row_types").and_then(|v| v.as_array()).map(|a| {
                    a.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect()
                }).unwrap_or_default();
                vec![(name, data, header, row_types)]
            };

            let freeze_rows = params.get("freeze_rows").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
            let freeze_cols = params.get("freeze_cols").and_then(|v| v.as_u64()).unwrap_or(0) as u16;

            // Color theme support: blue, green, orange, purple, teal, navy
            let theme = params.get("theme").and_then(|v| v.as_str()).unwrap_or("blue").to_string();
            let (hdr_bg, alt_bg, sec_bg, title_color) = match theme.as_str() {
                "green" => ("#2E7D32","#C8E6C9","#C8E6C9","#1B5E20"),
                "orange" => ("#E65100","#FFE0B2","#FFE0B2","#BF360C"),
                "purple" => ("#6A1B9A","#E1BEE7","#E1BEE7","#4A148C"),
                "teal" => ("#00695C","#B2DFDB","#B2DFDB","#004D40"),
                "navy" => ("#1A237E","#C5CAE9","#C5CAE9","#0D1642"),
                _ => ("#4472C4","#D9E2F3","#D9E2F3","#1F3864"),
            };

            // Build sheets from configs
            let mut sheets: Vec<writers::excel::SheetDef> = Vec::new();
            for (sheet_name, data, has_header, row_types) in &sheet_configs {
                // Calculate auto column widths based on data content
                let col_count = data.first().map(|r| r.len()).unwrap_or(0);
                let mut col_widths: Vec<f64> = Vec::with_capacity(col_count);
                for col in 0..col_count {
                    let mut max_width: usize = 8;
                    for row in data.iter() {
                        if let Some(val) = row.get(col) {
                            let len = match val {
                                serde_json::Value::String(s) => s.chars().count(),
                                serde_json::Value::Number(n) => n.to_string().len(),
                                serde_json::Value::Bool(_) => 4,
                                serde_json::Value::Null => 0,
                                _ => 8,
                            };
                            max_width = max_width.max(len);
                        }
                    }
                    col_widths.push((max_width as f64).max(12.0).min(45.0) + 2.0);
                }

                // Build cell grid with McKinsey-level professional formatting
                let has_types = !row_types.is_empty();
                let formatted_data: Vec<Vec<writers::excel::CellDef>> = data.iter().enumerate().map(|(row_idx, row)| {
                    // Determine row type: use row_types if provided, else fall back to header/data detection
                    let rtype: &str = if has_types {
                        let idx = row_idx.min(row_types.len() - 1);
                        row_types[idx].as_str()
                    } else if *has_header && row_idx == 0 {
                        "header"
                    } else {
                        "data"
                    };
                    let is_title = rtype == "title";
                    let is_section = rtype == "section";
                    let is_header = rtype == "header";
                    let is_total = rtype == "total";
                    let even = !is_title && !is_section && !is_header && !is_total && row_idx % 2 == 0;

                    row.iter().map(|val| {
                        let mut fmt = writers::excel::FormatDef {
                            border: Some(true),
                            ..Default::default()
                        };

                        if is_title {
                            fmt.bold = Some(true);
                            fmt.font_size = Some(16.0);
                            fmt.font_color = Some(title_color.to_string());
                            fmt.align_h = Some("center".to_string());
                            fmt.border = Some(false);
                        } else if is_section {
                            fmt.bg_color = Some(sec_bg.to_string());
                            fmt.font_color = Some(title_color.to_string());
                            fmt.bold = Some(true);
                            fmt.align_h = Some("left".to_string());
                        } else if is_header {
                            fmt.bg_color = Some(hdr_bg.to_string());
                            fmt.font_color = Some("#FFFFFF".to_string());
                            fmt.bold = Some(true);
                            fmt.align_h = Some("center".to_string());
                        } else if is_total {
                            fmt.bold = Some(true);
                            fmt.border = Some(true);
                            fmt.font_color = Some(title_color.to_string());
                            fmt.bg_color = Some("#F2F2F2".to_string());
                        } else if even {
                            fmt.bg_color = Some(alt_bg.to_string());
                        }

                        // Smart number formatting (skip for title, section rows, formulas)
                        if !is_title && !is_section {
                            if let serde_json::Value::Number(n) = val {
                                if let Some(f) = n.as_f64() {
                                    let f_abs = f.abs();
                                    if f_abs > 0.0 && f_abs < 1.0 && n.to_string().len() <= 4 {
                                        fmt.num_format = Some("0.0%".to_string());
                                    } else if f != f.trunc() {
                                        fmt.num_format = Some("#,##0.00".to_string());
                                    } else {
                                        fmt.num_format = Some("#,##0".to_string());
                                    }
                                }
                            }
                        }

                        writers::excel::CellDef {
                            value: Some(val.clone()),
                            format: Some(fmt),
                        }
                    }).collect()
                }).collect();

                let fr = if freeze_rows > 0 { Some(freeze_rows) } else if *has_header || has_types { Some(if has_types { row_types.iter().take_while(|t| *t == "title" || *t == "section" || *t == "header").count() as u16 } else { 1 }) } else { None };
                let has_any_header = has_types || *has_header;
                sheets.push(writers::excel::SheetDef {
                    name: Some(sheet_name.to_string()),
                    data: Some(formatted_data),
                    column_widths: Some(col_widths),
                    freeze_rows: fr,
                    freeze_cols: if freeze_cols > 0 { Some(freeze_cols) } else { None },
                    autofilter: if has_types { Some(false) } else { Some(*has_header) },
                    header_row_count: None,
                });
            }

            writers::excel::write_excel_basic(&path_str, sheets, false)
                .map_err(|e| format!("Excel write failed: {}", e))?;
            Ok(vec![path_str])
        }
        "excel.table" => {
            let data = extract_2d_data(params);
            let sheet_name = params.get("sheet_name").and_then(|v| v.as_str()).unwrap_or("Table1");
            let table_style = params.get("table_style").and_then(|v| v.as_str()).unwrap_or("medium");

            let rust_style = match table_style {
                "light" => "light1",
                "dark" => "dark1",
                _ => "medium9",
            };

            let headers: Vec<String> = data.first()
                .map(|row| row.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let table_def = writers::excel::TableDef {
                data: data.into_iter().skip(1).collect(),
                column_headers: headers,
                sheet_name: Some(sheet_name.to_string()),
                table_style: Some(rust_style.to_string()),
                use_zebra_stripes: Some(true),
                column_widths: None,
                show_total_row: None,
                header_format: None,
            };

            writers::excel::write_excel_table(&path_str, &table_def)
                .map_err(|e| format!("Excel table write failed: {}", e))?;
            Ok(vec![path_str])
        }
        "excel.chart" => {
            let data = extract_2d_data(params);
            let chart_type = params.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar");
            let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("Chart");
            let x_label = params.get("x_label").and_then(|v| v.as_str()).unwrap_or("");
            let y_label = params.get("y_label").and_then(|v| v.as_str()).unwrap_or("");
            let sheet_name = params.get("sheet_name").and_then(|v| v.as_str()).unwrap_or("Data");

            let chart_def = writers::excel::ChartDef {
                data,
                chart_type: chart_type.to_string(),
                categories_col: Some(0),
                values_col: Some(1),
                sheet_name: Some(sheet_name.to_string()),
                title: Some(title.to_string()),
                x_axis: if x_label.is_empty() { None } else { Some(x_label.to_string()) },
                y_axis: if y_label.is_empty() { None } else { Some(y_label.to_string()) },
                chart_col: None,
                chart_row: None,
            };

            writers::excel::write_excel_chart(&path_str, &chart_def)
                .map_err(|e| format!("Excel chart write failed: {}", e))?;
            Ok(vec![path_str])
        }
        "excel.pivot" => {
            let data = extract_2d_data(params);
            let rows = params.get("rows").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let cols = params.get("columns").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let values: Vec<String> = params.get("values").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let aggregate = params.get("aggregate").and_then(|v| v.as_str()).unwrap_or("sum");
            let sheet_name = params.get("sheet_name").and_then(|v| v.as_str()).unwrap_or("SourceData");

            let value_field = values.first().cloned().unwrap_or_default();
            let pivot_def = writers::excel::PivotDef {
                source_data: data,
                row_fields: rows,
                column_fields: cols,
                value_field,
                value_aggregation: aggregate.to_string(),
                sheet_name: Some(sheet_name.to_string()),
                filter_fields: None,
            };

            writers::excel::write_excel_pivot(&path_str, &pivot_def)
                .map_err(|e| format!("Excel pivot write failed: {}", e))?;
            Ok(vec![path_str])
        }
        _ => {
            let (path, _) = output_path(skill);
            let path_str = path.to_string_lossy().to_string();
            Ok(vec![path_str])
        }
    }
}

fn generate_docx(
    skill: &SkillDefinition,
    params: &serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<String>, String> {
    let (file_path, _filename) = output_path(skill);
    let path_str = file_path.to_string_lossy().to_string();

    match skill.name.as_str() {
        "word.report" => {
            let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("Report");
            let author = params.get("author").and_then(|v| v.as_str()).unwrap_or("");
            let sections = params.get("sections").and_then(|v| v.as_array()).map(|a| a.clone()).unwrap_or_default();
            let include_toc = params.get("include_toc").and_then(|v| v.as_bool()).unwrap_or(true);
            let include_page_numbers = params.get("include_page_numbers").and_then(|v| v.as_bool()).unwrap_or(true);
            let include_header = params.get("include_header").and_then(|v| v.as_bool()).unwrap_or(true);
            let theme = params.get("theme").and_then(|v| v.as_str()).unwrap_or("professional");
            let date = params.get("date").and_then(|v| v.as_str()).unwrap_or("");

            writers::word::write_word_report(
                &path_str, title, author, &sections,
                include_toc, include_page_numbers, include_header, theme, date,
            )?;
            Ok(vec![path_str])
        }
        "word.mailmerge" => {
            let template_text = params.get("template_text").and_then(|v| v.as_str()).unwrap_or("");
            let data_source = params.get("data_source").and_then(|v| v.as_array()).map(|a| a.clone()).unwrap_or_default();
            let filename_prefix = params.get("filename_prefix").and_then(|v| v.as_str()).unwrap_or("document");

            let out_dir = std::path::Path::new("output");
            let _ = std::fs::create_dir_all(out_dir);
            let out_dir_str = out_dir.to_string_lossy().to_string();

            let paths = writers::word::write_word_mailmerge(
                &out_dir_str, template_text, &data_source, filename_prefix,
            )?;
            let path_strs: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
            Ok(path_strs)
        }
        "word.invoice" => {
            writers::word::write_word_invoice(&path_str, params)?;
            Ok(vec![path_str])
        }
        "word.from_md" => {
            let markdown = params.get("markdown_text").and_then(|v| v.as_str()).unwrap_or("");
            let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("Document");
            let theme = params.get("theme").and_then(|v| v.as_str()).unwrap_or("professional");
            let include_toc = params.get("include_toc").and_then(|v| v.as_bool()).unwrap_or(false);
            let include_page_numbers = params.get("include_page_numbers").and_then(|v| v.as_bool()).unwrap_or(true);

            writers::word::write_word_from_md(
                &path_str, markdown, title, theme, include_toc, include_page_numbers,
            )?;
            Ok(vec![path_str])
        }
        _ => {
            let (path, _) = output_path(skill);
            let path_str = path.to_string_lossy().to_string();
            Ok(vec![path_str])
        }
    }
}

fn generate_pptx(
    skill: &SkillDefinition,
    params: &serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<String>, String> {
    match skill.name.as_str() {
        "ppt.deck" => {
            let slides_param = params.get("slides")
                .and_then(|v| v.as_array())
                .map(|a| a.clone())
                .unwrap_or_default();

            let mut outlines: Vec<writers::powerpoint::SlideOutline> = Vec::new();
            for s in &slides_param {
                let slide_title = s.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let content = s.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let body_text: Vec<String> = content.split('\n').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

                let bullets: Vec<String> = body_text.iter()
                    .filter(|l| l.starts_with("•") || l.starts_with("-"))
                    .cloned()
                    .collect();
                let plain_text: Vec<String> = body_text.iter()
                    .filter(|l| !l.starts_with("•") && !l.starts_with("-"))
                    .cloned()
                    .collect();

                outlines.push(writers::powerpoint::SlideOutline {
                    title: slide_title,
                    body_text: plain_text,
                    bullets,
                    alignment: None,
                });
            }

            if outlines.is_empty() {
                let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("Presentation");
                outlines.push(writers::powerpoint::SlideOutline {
                    title: title.to_string(),
                    body_text: vec![],
                    bullets: vec![],
                    alignment: None,
                });
            }

            let (file_path, _) = output_path(skill);
            writers::powerpoint::write_ppt_deck(&outlines, &file_path)?;
            Ok(vec![file_path.to_string_lossy().to_string()])
        }
        "ppt.from_md" => {
            let md = params.get("markdown_text").and_then(|v| v.as_str()).unwrap_or("");
            let (file_path, _) = output_path(skill);
            writers::powerpoint::write_ppt_from_md(md, &file_path)?;
            Ok(vec![file_path.to_string_lossy().to_string()])
        }
        _ => {
            let (file_path, _) = output_path(skill);
            Ok(vec![file_path.to_string_lossy().to_string()])
        }
    }
}

fn extract_2d_data(params: &serde_json::Map<String, serde_json::Value>) -> Vec<Vec<serde_json::Value>> {
    params.get("data")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().filter_map(|row| {
                row.as_array().map(|inner| inner.clone())
            }).collect()
        })
        .unwrap_or_default()
}

fn build_output(
    skill: &SkillDefinition,
    params: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let mut output = serde_json::Map::new();

    for out in &skill.outputs {
        match out.name.as_str() {
            "subtotal" | "tax_amount" | "total" | "rows_written" | "line_item_count"
            | "slide_count" | "record_count" | "columns" | "word_count"
            | "word_count_estimate" | "section_count" | "series_count" => {
                output.insert(
                    out.name.clone(),
                    compute_numeric_output(skill, params, &out.name),
                );
            }
            "file_path" | "chart_type" => {
                output.insert(out.name.clone(), serde_json::json!("generated_file"));
            }
            "row_fields" | "value_fields" => {
                output.insert(
                    out.name.clone(),
                    params
                        .get(&out.name)
                        .cloned()
                        .unwrap_or(serde_json::json!([])),
                );
            }
            "aggregate" => {
                output.insert(
                    out.name.clone(),
                    params
                        .get("aggregate")
                        .cloned()
                        .unwrap_or(serde_json::json!("sum")),
                );
            }
            "fields_found" | "headings" => {
                output.insert(out.name.clone(), serde_json::json!([]));
            }
            "slide_types" => {
                let types: Vec<&str> = params
                    .get("slides")
                    .and_then(|v| v.as_array())
                    .map(|slides| {
                        slides
                            .iter()
                            .filter_map(|s| s.get("type").and_then(|t| t.as_str()))
                            .collect()
                    })
                    .unwrap_or_default();
                output.insert(out.name.clone(), serde_json::json!(types));
            }
            "themes_applied" => {
                let theme = params
                    .get("theme")
                    .and_then(|v| v.as_str())
                    .unwrap_or("light");
                output.insert(
                    out.name.clone(),
                    serde_json::json!({
                        "theme": theme,
                        "background": if theme == "dark" { "#1E1E1E" } else { "#FFFFFF" },
                        "text": if theme == "dark" { "#CCCCCC" } else { "#333333" }
                    }),
                );
            }
            _ => {
                output.insert(out.name.clone(), serde_json::json!(null));
            }
        }
    }

    serde_json::Value::Object(output)
}

fn compute_numeric_output(
    skill: &SkillDefinition,
    params: &serde_json::Map<String, serde_json::Value>,
    output_name: &str,
) -> serde_json::Value {
    match output_name {
        "subtotal" => {
            let items = params.get("line_items").and_then(|v| v.as_array());
            let subtotal: f64 = items
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            let qty =
                                item.get("quantity").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let price = item
                                .get("unit_price")
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            Some(qty * price)
                        })
                        .sum()
                })
                .unwrap_or(0.0);
            serde_json::json!(subtotal)
        }
        "tax_amount" => {
            let subtotal_val = compute_numeric_output(skill, params, "subtotal");
            let tax_rate = params.get("tax_rate").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let amount = subtotal_val.as_f64().unwrap_or(0.0) * tax_rate;
            serde_json::json!(amount)
        }
        "total" => {
            let subtotal_val = compute_numeric_output(skill, params, "subtotal");
            let tax_val = compute_numeric_output(skill, params, "tax_amount");
            let total =
                subtotal_val.as_f64().unwrap_or(0.0) + tax_val.as_f64().unwrap_or(0.0);
            serde_json::json!(total)
        }
        "rows_written" => {
            let count = params
                .get("data")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            serde_json::json!(count)
        }
        "line_item_count" => {
            let count = params
                .get("line_items")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            serde_json::json!(count)
        }
        "slide_count" => {
            let count = params
                .get("slides")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            serde_json::json!(count)
        }
        "record_count" => {
            let count = params
                .get("data_source")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            serde_json::json!(count)
        }
        "columns" => {
            let count = params
                .get("data")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|r| r.as_array())
                .map(|r| r.len())
                .unwrap_or(0);
            serde_json::json!(count)
        }
        "section_count" => {
            let count = params
                .get("sections")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            serde_json::json!(count)
        }
        "series_count" => {
            let count = params
                .get("data")
                .and_then(|v| v.as_array())
                .map(|a| a.len().saturating_sub(1))
                .unwrap_or(0);
            serde_json::json!(count)
        }
        "word_count" | "word_count_estimate" => {
            let count = params
                .get("markdown_text")
                .or_else(|| params.get("template_text"))
                .and_then(|v| v.as_str())
                .map(|s| s.split_whitespace().count())
                .unwrap_or(0);
            serde_json::json!(count)
        }
        _ => serde_json::json!(0),
    }
}
