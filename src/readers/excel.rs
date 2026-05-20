use calamine::{open_workbook, Data, Dimensions, Reader, Xlsx};
use serde::Serialize;
use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct CellValue(pub String);

impl From<&Data> for CellValue {
    fn from(d: &Data) -> Self {
        match d {
            Data::String(s) => CellValue(s.clone()),
            Data::Float(f) => CellValue(f.to_string()),
            Data::Int(i) => CellValue(i.to_string()),
            Data::Bool(b) => CellValue(if *b { "true".into() } else { "false".into() }),
            Data::DateTime(dt) => CellValue(dt.to_string()),
            Data::DateTimeIso(dt) => CellValue(dt.clone()),
            Data::DurationIso(d) => CellValue(d.clone()),
            Data::Error(e) => CellValue(format!("#{}", e)),
            Data::Empty => CellValue(String::new()),
        }
    }
}

impl CellValue {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn data_to_json_value(d: &Data) -> serde_json::Value {
    match d {
        Data::Float(f) => serde_json::Value::Number(
            serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from_f64(0.0).unwrap()),
        ),
        Data::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        Data::Bool(b) => serde_json::Value::Bool(*b),
        Data::String(s) => serde_json::Value::String(s.clone()),
        Data::DateTime(dt) => serde_json::Value::String(dt.to_string()),
        Data::DateTimeIso(dt) => serde_json::Value::String(dt.clone()),
        Data::DurationIso(d) => serde_json::Value::String(d.clone()),
        Data::Error(e) => serde_json::Value::String(format!("#{}", e)),
        Data::Empty => serde_json::Value::Null,
    }
}

fn json_value_display(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f == f.trunc() && f.is_finite() && f.abs() < 1e15 {
                    format!("{:.0}", f)
                } else {
                    n.to_string()
                }
            } else {
                n.to_string()
            }
        }
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SheetData {
    pub name: String,
    pub dimensions: String,
    pub row_count: usize,
    pub column_count: usize,
    pub header_row: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub column_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NamedRangeInfo {
    pub name: String,
    pub range: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExcelOutput {
    pub format: String,
    pub file: String,
    pub size_bytes: u64,
    pub sheets: Vec<SheetData>,
    pub named_ranges: Vec<NamedRangeInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Chunk {
    pub index: usize,
    pub text: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct ExcelReader;

impl ExcelReader {
    fn open(path: &Path) -> Result<Xlsx<BufReader<std::fs::File>>, String> {
        open_workbook(path).map_err(|e| format!("Failed to open workbook: {}", e))
    }

    pub fn read_to_json(path: &Path) -> Result<ExcelOutput, String> {
        let file = path.to_string_lossy().to_string();
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let mut workbook = Self::open(path)?;

        let sheet_names = workbook.sheet_names().to_vec();
        let mut sheets = Vec::with_capacity(sheet_names.len());

        workbook
            .load_merged_regions()
            .map_err(|e| format!("Failed to load merged regions: {}", e))?;
        let merged_all: Vec<(String, String, Dimensions)> = workbook.merged_regions().clone();

        for name in &sheet_names {
            let range = workbook
                .worksheet_range(name)
                .map_err(|e| format!("Failed to read sheet '{}': {}", e, name))?;

            let merged_for_sheet: Vec<(u32, u32, u32, u32)> = merged_all
                .iter()
                .filter(|(sn, _, _)| sn == name)
                .map(|(_, _, dims)| (dims.start.0, dims.start.1, dims.end.0, dims.end.1))
                .collect();

            let (start_row, start_col) = range.start().map(|s| (s.0, s.1)).unwrap_or((0, 0));
            let (end_row, end_col) = range.end().map(|e| (e.0, e.1)).unwrap_or((0, 0));

            let dims = format!(
                "{}{}:{}{}",
                column_letter(start_col),
                start_row + 1,
                column_letter(end_col),
                end_row + 1
            );

            let raw_rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();

            let resolved: Vec<Vec<CellValue>> = raw_rows
                .iter()
                .enumerate()
                .map(|(ri, row)| {
                    let ri_u32 = ri as u32;
                    row.iter()
                        .enumerate()
                        .map(|(ci, cell)| {
                            let ci_u32 = ci as u32;
                            let world_row = start_row + ri_u32;
                            let world_col = start_col + ci_u32;
                            if cell == &Data::Empty {
                                if merged_contains(&merged_for_sheet, world_row, world_col) {
                                    CellValue(String::new())
                                } else {
                                    CellValue::from(cell)
                                }
                            } else {
                                CellValue::from(cell)
                            }
                        })
                        .collect()
                })
                .collect();

            let header_row_index = detect_header_row(&resolved);
            let column_count = resolved.iter().map(|r| r.len()).max().unwrap_or(0);

            let header_row: Vec<String> = if header_row_index < resolved.len() {
                let hr = &resolved[header_row_index];
                (0..column_count)
                    .map(|ci| {
                        hr.get(ci)
                            .map(|c| c.as_str().to_string())
                            .unwrap_or_default()
                    })
                    .collect()
            } else {
                vec![]
            };

            let data_rows: Vec<Vec<serde_json::Value>> = raw_rows
                .iter()
                .skip(if header_row_index < resolved.len() {
                    header_row_index + 1
                } else {
                    0
                })
                .map(|row| {
                    (0..column_count)
                        .map(|ci| {
                            row.get(ci)
                                .map(data_to_json_value)
                                .unwrap_or(serde_json::Value::Null)
                        })
                        .collect()
                })
                .collect();

            let col_types = if !data_rows.is_empty() && !header_row.is_empty() {
                infer_column_types(&data_rows)
            } else if !raw_rows.is_empty() {
                let all_data: Vec<Vec<serde_json::Value>> = raw_rows
                    .iter()
                    .map(|row| {
                        (0..column_count)
                            .map(|ci| {
                                row.get(ci)
                                    .map(data_to_json_value)
                                    .unwrap_or(serde_json::Value::Null)
                            })
                            .collect()
                    })
                    .collect();
                infer_column_types(&all_data)
            } else {
                vec![]
            };

            sheets.push(SheetData {
                name: name.clone(),
                dimensions: dims,
                row_count: (end_row - start_row + 1) as usize,
                column_count,
                header_row,
                rows: data_rows,
                column_types: col_types,
            });
        }

        let named_ranges = extract_named_ranges(&workbook);

        Ok(ExcelOutput {
            format: "xlsx".into(),
            file,
            size_bytes: size,
            sheets,
            named_ranges,
        })
    }

    pub fn read_to_md(path: &Path) -> Result<String, String> {
        let json_output = Self::read_to_json(path)?;
        let mut md = String::new();

        md.push_str(&format!(
            "---\nformat: {}\nfile: {}\nsheets: {}\ntotal_rows: {}\n---\n\n",
            json_output.format,
            json_output.file,
            json_output.sheets.len(),
            json_output
                .sheets
                .iter()
                .map(|s| s.rows.len())
                .sum::<usize>(),
        ));

        for sheet in &json_output.sheets {
            md.push_str(&format!("# {}\n\n", sheet.name));

            md.push_str(&format!(
                "> *Dimensions: {} · {} rows × {} columns*  \n",
                sheet.dimensions, sheet.row_count, sheet.column_count
            ));

            if !sheet.column_types.is_empty() {
                md.push_str("> *Column types: ");
                let type_strs: Vec<String> = sheet
                    .column_types
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let h = sheet.header_row.get(i).map(|h| h.as_str()).unwrap_or("");
                        format!("`{}`: {}", h, t)
                    })
                    .collect();
                md.push_str(&type_strs.join(", "));
                md.push_str("*  \n");
            }

            md.push('\n');

            if !sheet.header_row.is_empty() {
                md.push('|');
                for h in &sheet.header_row {
                    md.push_str(&format!(" {} |", h));
                }
                md.push('\n');
                md.push('|');
                for _ in &sheet.header_row {
                    md.push_str(" :- |");
                }
                md.push('\n');

                for row in &sheet.rows {
                    md.push('|');
                    for val in row {
                        let s = json_value_display(val);
                        md.push_str(&format!(" {} |", s));
                    }
                    md.push('\n');
                }
            }

            md.push_str("\n---\n\n");
        }

        if !json_output.named_ranges.is_empty() {
            md.push_str("## Named Ranges\n\n");
            for nr in &json_output.named_ranges {
                md.push_str(&format!("- **{}**: `{}`\n", nr.name, nr.range));
            }
            md.push('\n');
        }

        Ok(md)
    }

    pub fn read_to_chunks(path: &Path) -> Result<Vec<Chunk>, String> {
        let json_output = Self::read_to_json(path)?;
        let mut chunks = Vec::new();
        let mut index = 0;

        for sheet in &json_output.sheets {
            let mut chunk_text = String::new();
            chunk_text.push_str(&format!("# Sheet: {}\n\n", sheet.name));
            chunk_text.push_str(&format!(
                "Dimensions: {} · {} rows × {} cols\n\n",
                sheet.dimensions, sheet.row_count, sheet.column_count
            ));

            if !sheet.column_types.is_empty() {
                chunk_text.push_str("Column types:\n");
                for (i, t) in sheet.column_types.iter().enumerate() {
                    let h = sheet.header_row.get(i).map(|h| h.as_str()).unwrap_or("");
                    chunk_text.push_str(&format!("- `{}`: {}\n", h, t));
                }
                chunk_text.push('\n');
            }

            let row_chunk_size = 50;
            let has_header = !sheet.header_row.is_empty();

            let table_header = if has_header {
                let header_line = format!("| {} |\n", sheet.header_row.join(" | "));
                let sep_line = format!(
                    "|{}|\n",
                    sheet
                        .header_row
                        .iter()
                        .map(|_| " :- ")
                        .collect::<Vec<_>>()
                        .join("|")
                );
                Some((header_line, sep_line))
            } else {
                None
            };

            for (row_idx, row) in sheet.rows.iter().enumerate() {
                if row_idx > 0 && row_idx % row_chunk_size == 0 {
                    let mut meta = HashMap::new();
                    meta.insert("format".into(), serde_json::Value::String("xlsx".into()));
                    meta.insert(
                        "sheet".into(),
                        serde_json::Value::String(sheet.name.clone()),
                    );
                    meta.insert(
                        "row_start".into(),
                        serde_json::Value::Number((row_idx - row_chunk_size).into()),
                    );
                    meta.insert(
                        "row_end".into(),
                        serde_json::Value::Number((row_idx - 1).into()),
                    );
                    chunks.push(Chunk {
                        index,
                        text: chunk_text.clone(),
                        metadata: meta,
                    });
                    index += 1;
                    chunk_text.clear();
                    chunk_text.push_str(&format!("# Sheet: {} (continued)\n\n", sheet.name));
                    if let Some((ref hdr, ref sep)) = table_header {
                        chunk_text.push_str(hdr);
                        chunk_text.push_str(sep);
                    }
                }

                if row_idx == 0 {
                    if let Some((ref hdr, ref sep)) = table_header {
                        chunk_text.push_str(hdr);
                        chunk_text.push_str(sep);
                    }
                }

                let vals: Vec<String> = row.iter().map(|v| json_value_display(v)).collect();
                chunk_text.push_str(&format!("| {} |\n", vals.join(" | ")));
            }

            if !chunk_text.is_empty() {
                let mut meta = HashMap::new();
                meta.insert("format".into(), serde_json::Value::String("xlsx".into()));
                meta.insert(
                    "sheet".into(),
                    serde_json::Value::String(sheet.name.clone()),
                );
                let total_rows = sheet.rows.len();
                let chunk_start = if total_rows > 0 {
                    ((total_rows - 1) / row_chunk_size) * row_chunk_size
                } else {
                    0
                };
                meta.insert(
                    "row_start".into(),
                    serde_json::Value::Number(chunk_start.into()),
                );
                meta.insert(
                    "row_end".into(),
                    serde_json::Value::Number((total_rows.saturating_sub(1)).into()),
                );
                chunks.push(Chunk {
                    index,
                    text: chunk_text,
                    metadata: meta,
                });
                index += 1;
            }
        }

        Ok(chunks)
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

fn merged_contains(regions: &[(u32, u32, u32, u32)], row: u32, col: u32) -> bool {
    for &(r_start, c_start, r_end, c_end) in regions {
        if row >= r_start && row <= r_end && col >= c_start && col <= c_end {
            return true;
        }
    }
    false
}

fn detect_header_row(rows: &[Vec<CellValue>]) -> usize {
    if rows.is_empty() {
        return 0;
    }
    for (i, row) in rows.iter().enumerate() {
        let non_empty: Vec<&CellValue> = row.iter().filter(|c| !c.is_empty()).collect();
        if non_empty.is_empty() {
            continue;
        }
        let non_numeric = non_empty
            .iter()
            .filter(|c| c.as_str().parse::<f64>().is_err())
            .count();
        let ratio = non_numeric as f64 / non_empty.len() as f64;
        if ratio >= 0.5 && non_empty.len() >= 2 {
            let unique_count = {
                let mut vals: Vec<&str> = non_empty.iter().map(|c| c.as_str()).collect();
                vals.sort();
                vals.dedup();
                vals.len()
            };
            if unique_count as f64 / non_empty.len() as f64 >= 0.5 {
                return i;
            }
        }
    }
    0
}

fn infer_column_types(rows: &[Vec<serde_json::Value>]) -> Vec<String> {
    if rows.is_empty() {
        return vec![];
    }
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut types: Vec<String> = Vec::with_capacity(col_count);

    for ci in 0..col_count {
        let values: Vec<String> = rows
            .iter()
            .filter_map(|r| r.get(ci))
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => String::new(),
            })
            .filter(|s| !s.is_empty())
            .collect();

        if values.is_empty() {
            types.push("string".into());
            continue;
        }

        let total = values.len();
        let currency_count = values
            .iter()
            .filter(|v| {
                let s = v.trim();
                (s.starts_with('$') || s.starts_with('\u{20ac}') || s.starts_with('\u{a3}'))
                    && s[1..].replace(',', "").parse::<f64>().is_ok()
            })
            .count();

        let pct_count = values.iter().filter(|v| v.ends_with('%')).count();

        let date_count = values
            .iter()
            .filter(|v| {
                let s = v.trim();
                s.len() == 10
                    && s.chars().filter(|&c| c == '-').count() == 2
                    && s[..4].parse::<i32>().is_ok()
                    && s[5..7].parse::<u32>().is_ok()
                    && s[8..10].parse::<u32>().is_ok()
            })
            .count();

        let bool_count = values
            .iter()
            .filter(|v| matches!(v.to_lowercase().as_str(), "true" | "false" | "yes" | "no"))
            .count();

        let number_count = values
            .iter()
            .filter(|v| v.replace(',', "").parse::<f64>().is_ok())
            .count();

        if currency_count as f64 / total as f64 >= 0.8 {
            types.push("currency".into());
        } else if pct_count as f64 / total as f64 >= 0.8 {
            types.push("pct".into());
        } else if date_count as f64 / total as f64 >= 0.8 {
            types.push("date".into());
        } else if bool_count as f64 / total as f64 >= 0.8 {
            types.push("boolean".into());
        } else if number_count as f64 / total as f64 >= 0.8 {
            types.push("number".into());
        } else {
            types.push("string".into());
        }
    }

    types
}

fn extract_named_ranges(workbook: &Xlsx<BufReader<std::fs::File>>) -> Vec<NamedRangeInfo> {
    let mut ranges = Vec::new();
    for (name, range_str) in workbook.defined_names() {
        ranges.push(NamedRangeInfo {
            name: name.to_string(),
            range: range_str.to_string(),
        });
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> String {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("test");
        p.push("fixtures");
        p.push(name);
        p.to_string_lossy().to_string()
    }

    #[test]
    fn test_read_excel_to_json_basic() {
        let fp = fixture_path("sample.xlsx");
        let path = std::path::Path::new(&fp);
        let result = ExcelReader::read_to_json(path).unwrap();
        assert_eq!(result.format, "xlsx");
        assert!(result.sheets.len() >= 3, "Should have 3+ sheets");

        let sheet1 = &result.sheets[0];
        assert_eq!(sheet1.name, "Sales Data");
        assert!(!sheet1.header_row.is_empty(), "Should detect header row");
        assert_eq!(sheet1.header_row[0], "Product");
        assert!(!sheet1.rows.is_empty(), "Should have data rows");
    }

    #[test]
    fn test_read_excel_to_md_basic() {
        let fp = fixture_path("sample.xlsx");
        let path = std::path::Path::new(&fp);
        let md = ExcelReader::read_to_md(path).unwrap();
        assert!(md.contains('|'), "Markdown should contain tables");
        assert!(md.contains("---"), "Should have YAML frontmatter");
        assert!(md.contains("Sales Data"), "Should have sheet names");
    }

    #[test]
    fn test_read_excel_to_chunks_basic() {
        let fp = fixture_path("sample.xlsx");
        let path = std::path::Path::new(&fp);
        let chunks = ExcelReader::read_to_chunks(path).unwrap();
        assert!(!chunks.is_empty(), "Should have at least one chunk");
        assert_eq!(chunks[0].metadata["format"], "xlsx");
        assert!(
            chunks[0].text.contains("Sheet:"),
            "Chunk should contain sheet name"
        );
    }

    #[test]
    fn test_column_type_inference() {
        let fp = fixture_path("sample.xlsx");
        let path = std::path::Path::new(&fp);
        let result = ExcelReader::read_to_json(path).unwrap();

        let inventory = result
            .sheets
            .iter()
            .find(|s| s.name == "Inventory")
            .unwrap();
        assert_eq!(inventory.column_types[0], "string");
        assert_eq!(inventory.column_types[1], "string");
    }

    #[test]
    fn test_named_ranges() {
        let fp = fixture_path("sample.xlsx");
        let path = std::path::Path::new(&fp);
        let result = ExcelReader::read_to_json(path).unwrap();
        assert!(!result.named_ranges.is_empty(), "Should have named ranges");
        assert_eq!(result.named_ranges[0].name, "InventoryRange");
    }

    #[test]
    fn test_file_not_found() {
        let path = std::path::Path::new("/nonexistent/file.xlsx");
        let result = ExcelReader::read_to_json(path);
        assert!(result.is_err(), "Should error on missing file");
    }
}
