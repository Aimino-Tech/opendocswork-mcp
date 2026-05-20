use rust_xlsxwriter::{
    Chart, ChartType, Format, FormatAlign, FormatBorder, Table, TableColumn,
    TableFunction, TableStyle, Workbook, Worksheet, XlsxError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<FormatDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FormatDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_h: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_v: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_rotation: Option<i16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<Vec<CellDef>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_widths: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freeze_rows: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freeze_cols: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autofilter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_row_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDef {
    pub data: Vec<Vec<serde_json::Value>>,
    pub column_headers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_zebra_stripes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_widths: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_total_row: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_format: Option<FormatDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartDef {
    pub data: Vec<Vec<serde_json::Value>>,
    pub chart_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories_col: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values_col: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_axis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y_axis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chart_col: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chart_row: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotDef {
    pub source_data: Vec<Vec<serde_json::Value>>,
    pub row_fields: Vec<String>,
    pub column_fields: Vec<String>,
    pub value_field: String,
    pub value_aggregation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_fields: Option<Vec<String>>,
}

pub fn write_excel_basic(
    file_path: &str,
    sheets: Vec<SheetDef>,
    use_constant_memory: bool,
) -> Result<PathBuf, XlsxError> {
    let mut workbook = Workbook::new();

    for sheet in &sheets {
        let worksheet = if use_constant_memory {
            workbook.add_worksheet_with_constant_memory()
        } else {
            workbook.add_worksheet()
        };

        if let Some(ref name) = sheet.name {
            worksheet.set_name(name)?;
        }

        if let Some(ref widths) = sheet.column_widths {
            for (i, &w) in widths.iter().enumerate() {
                worksheet.set_column_width(i as u16, w)?;
            }
        }

        if let Some(freeze_rows) = sheet.freeze_rows {
            worksheet.set_freeze_panes(freeze_rows as u32, sheet.freeze_cols.unwrap_or(0))?;
        }

        if let Some(ref data) = sheet.data {
            write_cell_grid(worksheet, data)?;

            if sheet.autofilter.unwrap_or(false) {
                let num_rows = data.len() as u32;
                let num_cols = data.first().map(|r| r.len() as u16).unwrap_or(0);
                if num_rows > 0 && num_cols > 0 {
                    let _ = worksheet.autofilter(0, 0, num_rows - 1, num_cols - 1);
                }
            }
        }

        worksheet.set_landscape();
        worksheet.set_margins(0.7, 0.7, 0.75, 0.75, 0.25, 0.25);
    }

    workbook.save(file_path)?;
    Ok(PathBuf::from(file_path))
}

pub fn write_excel_table(file_path: &str, def: &TableDef) -> Result<PathBuf, XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    if let Some(ref name) = def.sheet_name {
        worksheet.set_name(name)?;
    }

    let header_row = 0;
    let num_rows = def.data.len() as u32;
    let num_cols = def.column_headers.len() as u16;

    if num_rows == 0 || num_cols == 0 {
        return Err(XlsxError::from(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "table must have at least one row and one column",
        )));
    }

    let data_start_row = header_row + 1;
    let data_end_row = data_start_row + num_rows - 1;
    let col_end = num_cols - 1;

    if let Some(ref widths) = def.column_widths {
        for (i, &w) in widths.iter().enumerate() {
            worksheet.set_column_width(i as u16, w)?;
        }
    } else {
        for (i, header) in def.column_headers.iter().enumerate() {
            let col_width = (header.len() as f64).max(8.0).min(40.0);
            worksheet.set_column_width(i as u16, col_width)?;
        }
    }

    let header_format = if let Some(ref hf) = def.header_format {
        apply_format(Format::new(), hf)
    } else {
        Format::new()
            .set_bold()
            .set_font_color("FFFFFF")
            .set_background_color("4472C4")
            .set_border(FormatBorder::Thin)
            .set_align(FormatAlign::Center)
            .to_owned()
    };

    for (col, header) in def.column_headers.iter().enumerate() {
        worksheet.write_string_with_format(header_row, col as u16, header, &header_format)?;
    }

    let show_zebra = def.use_zebra_stripes.unwrap_or(true);
    let even_row_fmt = if show_zebra {
        Format::new().set_background_color("D9E2F3").to_owned()
    } else {
        Format::new().to_owned()
    };

    for (row_idx, row_data) in def.data.iter().enumerate() {
        let row = data_start_row + row_idx as u32;
        let row_format = if row_idx % 2 == 1 && show_zebra {
            Some(&even_row_fmt)
        } else {
            None
        };
        for (col, val) in row_data.iter().enumerate() {
            if col >= num_cols as usize {
                break;
            }
            write_json_value(worksheet, row, col as u16, val, row_format)?;
        }
    }

    let mut table_builder = Table::new();

    let mut columns = Vec::new();
    for header in &def.column_headers {
        let mut tc = TableColumn::new();
        tc = tc.set_header(header);
        if def.show_total_row.unwrap_or(false) {
            tc = tc.set_total_function(TableFunction::Sum);
        }
        columns.push(tc);
    }
    table_builder = table_builder.set_columns(&columns);

    if def.show_total_row.unwrap_or(false) {
        table_builder = table_builder.set_total_row(true);
    }

    if let Some(ref style_name) = def.table_style {
        if let Some(style) = parse_table_style(style_name) {
            table_builder = table_builder.set_style(style);
        }
    }

    worksheet.add_table(data_start_row - 1, 0, data_end_row, col_end, &table_builder)?;

    workbook.save(file_path)?;
    Ok(PathBuf::from(file_path))
}

pub fn write_excel_chart(file_path: &str, def: &ChartDef) -> Result<PathBuf, XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    if let Some(ref name) = def.sheet_name {
        worksheet.set_name(name)?;
    }

    if def.data.is_empty() || def.data[0].is_empty() {
        return Err(XlsxError::from(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "chart data must have at least one row and one column",
        )));
    }

    for (row_idx, row_data) in def.data.iter().enumerate() {
        for (col, val) in row_data.iter().enumerate() {
            write_json_value(worksheet, row_idx as u32, col as u16, val, None)?;
        }
    }

    let num_rows = def.data.len() as u32;
    let num_cols = def.data[0].len() as u16;

    let cat_col = def.categories_col.unwrap_or(0);
    let val_col = def
        .values_col
        .unwrap_or_else(|| if num_cols > 1 { 1 } else { 0 });

    let sheet_name_ref = def.sheet_name.as_deref().unwrap_or("Sheet1");
    let cat_ref = format!(
        "='{}'!${}${}:${}${}",
        sheet_name_ref,
        col_to_letter(cat_col),
        1,
        col_to_letter(cat_col),
        num_rows
    );
    let val_ref = format!(
        "='{}'!${}${}:${}${}",
        sheet_name_ref,
        col_to_letter(val_col),
        1,
        col_to_letter(val_col),
        num_rows
    );

    let chart_type = match def.chart_type.to_lowercase().as_str() {
        "bar" => ChartType::Bar,
        "line" => ChartType::Line,
        "pie" => ChartType::Pie,
        _ => ChartType::Column,
    };

    let mut chart = Chart::new(chart_type);

    if chart_type == ChartType::Pie {
        chart
            .add_series()
            .set_categories(&cat_ref)
            .set_values(&val_ref);
    } else {
        for col in 0..num_cols {
            if col == cat_col {
                continue;
            }
            let series_cat = format!(
                "='{}'!${}${}:${}${}",
                sheet_name_ref,
                col_to_letter(cat_col),
                1,
                col_to_letter(cat_col),
                num_rows
            );
            let series_val = format!(
                "='{}'!${}${}:${}${}",
                sheet_name_ref,
                col_to_letter(col),
                1,
                col_to_letter(col),
                num_rows
            );
            chart
                .add_series()
                .set_categories(&series_cat)
                .set_values(&series_val);
        }
    }

    if let Some(ref title) = def.title {
        chart.title().set_name(title);
    }
    if let Some(ref x_axis) = def.x_axis {
        chart.x_axis().set_name(x_axis);
    }
    if let Some(ref y_axis) = def.y_axis {
        chart.y_axis().set_name(y_axis);
    }

    let chart_col = def.chart_col.unwrap_or(num_cols + 2);
    let chart_row = def.chart_row.unwrap_or(0);
    worksheet.insert_chart(chart_row as u32, chart_col as u16, &chart)?;

    workbook.save(file_path)?;
    Ok(PathBuf::from(file_path))
}

pub fn write_excel_pivot(file_path: &str, def: &PivotDef) -> Result<PathBuf, XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    if let Some(ref name) = def.sheet_name {
        worksheet.set_name(name)?;
    }

    if def.source_data.is_empty() {
        return Err(XlsxError::from(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "pivot source data must have at least one row",
        )));
    }

    let headers: Vec<&str> = def.source_data[0]
        .iter()
        .map(|v| v.as_str().unwrap_or(""))
        .collect();

    let row_field_indices: Vec<usize> = def
        .row_fields
        .iter()
        .filter_map(|f| headers.iter().position(|h| *h == f.as_str()))
        .collect();
    let col_field_indices: Vec<usize> = def
        .column_fields
        .iter()
        .filter_map(|f| headers.iter().position(|h| *h == f.as_str()))
        .collect();
    let val_idx = headers
        .iter()
        .position(|h| *h == def.value_field.as_str())
        .unwrap_or(0);

    let mut pivot_rows: HashMap<(Vec<String>, Vec<String>), f64> = HashMap::new();
    let mut col_keys: Vec<Vec<String>> = Vec::new();

    for row in &def.source_data[1..] {
        let row_key: Vec<String> = row_field_indices
            .iter()
            .map(|&i| {
                row.get(i)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();
        let col_key: Vec<String> = col_field_indices
            .iter()
            .map(|&i| {
                row.get(i)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();

        let val = row.get(val_idx).and_then(|v| v.as_f64()).unwrap_or(0.0);

        let key = (row_key, col_key.clone());
        *pivot_rows.entry(key).or_insert(0.0) += val;

        if !col_keys.contains(&col_key) {
            col_keys.push(col_key);
        }
    }

    let mut row_keys_set: std::collections::BTreeSet<Vec<String>> =
        std::collections::BTreeSet::new();
    for (rk, _) in pivot_rows.keys() {
        row_keys_set.insert(rk.clone());
    }
    let row_keys: Vec<Vec<String>> = row_keys_set.into_iter().collect();

    let agg_label = format!("{} ({})", def.value_field, def.value_aggregation);

    let mut col_offset: u16 = 0;
    for rf in &def.row_fields {
        worksheet.write_string(0, col_offset, rf)?;
        col_offset += 1;
    }
    worksheet.write_string(0, col_offset, &agg_label)?;
    col_offset += 1;

    for ck in &col_keys {
        worksheet.write_string(0, col_offset, &ck.join(" | "))?;
        col_offset += 1;
    }

    for (i, rk) in row_keys.iter().enumerate() {
        let row = (i + 1) as u32;
        let mut col_offset: u16 = 0;
        for val in rk {
            worksheet.write_string(row, col_offset, val)?;
            col_offset += 1;
        }

        let total_key = (rk.clone(), vec![]);
        let total_val = pivot_rows.get(&total_key).copied().unwrap_or(0.0);
        worksheet.write_number(row, col_offset, total_val)?;
        col_offset += 1;

        for ck in &col_keys {
            let key = (rk.clone(), ck.clone());
            let val = pivot_rows.get(&key).copied().unwrap_or(0.0);
            worksheet.write_number(row, col_offset, val)?;
            col_offset += 1;
        }
    }

    worksheet.set_column_range_width(0, col_offset, 14.0)?;
    workbook.save(file_path)?;
    Ok(PathBuf::from(file_path))
}

fn write_cell_grid(worksheet: &mut Worksheet, data: &[Vec<CellDef>]) -> Result<(), XlsxError> {
    for (row_idx, row_data) in data.iter().enumerate() {
        for (col, cell) in row_data.iter().enumerate() {
            let row = row_idx as u32;
            let col_u16 = col as u16;

            let fmt = if let Some(ref fd) = cell.format {
                Some(apply_format(Format::new(), fd))
            } else {
                None
            };

            match &cell.value {
                None => {
                    if let Some(ref f) = fmt {
                        worksheet.write_with_format(row, col_u16, "", f)?;
                    }
                }
                Some(v) => {
                    if let Some(ref f) = fmt {
                        write_json_value(worksheet, row, col_u16, v, Some(f))?;
                    } else {
                        write_json_value(worksheet, row, col_u16, v, None)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn write_json_value(
    worksheet: &mut Worksheet,
    row: u32,
    col: u16,
    val: &serde_json::Value,
    fmt: Option<&Format>,
) -> Result<(), XlsxError> {
    match val {
        serde_json::Value::String(s) => {
            // Detect formulas: strings starting with =
            if s.starts_with('=') {
                // Support embedded result: "=A1+B1||123.45" → formula="A1+B1", result="123.45"
                let (formula_str, result_opt) = if let Some(pos) = s.find("||") {
                    let f = &s[1..pos];
                    let r = &s[pos+2..];
                    (f, Some(r.to_string()))
                } else {
                    (&s[1..], None)
                };
                let mut formula = rust_xlsxwriter::Formula::new(formula_str);
                if let Some(ref r) = result_opt {
                    formula = formula.set_result(r.as_str());
                }
                if let Some(f) = fmt {
                    worksheet.write_formula_with_format(row, col, formula, f)?;
                } else {
                    worksheet.write_formula(row, col, formula)?;
                }
            } else if let Some(f) = fmt {
                worksheet.write_string_with_format(row, col, s, f)?;
            } else {
                worksheet.write_string(row, col, s)?;
            }
        }
        serde_json::Value::Number(n) => {
            if let Some(f) = fmt {
                if let Some(v) = n.as_f64() {
                    worksheet.write_number_with_format(row, col, v, f)?;
                } else {
                    worksheet.write_string_with_format(row, col, &n.to_string(), f)?;
                }
            } else if let Some(v) = n.as_f64() {
                worksheet.write_number(row, col, v)?;
            } else {
                worksheet.write_string(row, col, &n.to_string())?;
            }
        }
        serde_json::Value::Bool(b) => {
            if let Some(f) = fmt {
                worksheet.write_string_with_format(
                    row,
                    col,
                    if *b { "TRUE" } else { "FALSE" },
                    f,
                )?;
            } else {
                worksheet.write_string(row, col, if *b { "TRUE" } else { "FALSE" })?;
            }
        }
        serde_json::Value::Null => {
            if let Some(f) = fmt {
                worksheet.write_with_format(row, col, "", f)?;
            } else {
                worksheet.write_string(row, col, "")?;
            }
        }
        _ => {
            let s = serde_json::to_string(val).unwrap_or_default();
            if let Some(f) = fmt {
                worksheet.write_string_with_format(row, col, &s, f)?;
            } else {
                worksheet.write_string(row, col, &s)?;
            }
        }
    }
    Ok(())
}

fn apply_format(mut format: Format, def: &FormatDef) -> Format {
    if let Some(true) = def.bold {
        format = format.set_bold();
    }
    if let Some(true) = def.italic {
        format = format.set_italic();
    }
    if let Some(ref color) = def.font_color {
        if let Ok(c) = parse_color(color) {
            format = format.set_font_color(c.as_str());
        }
    }
    if let Some(size) = def.font_size {
        format = format.set_font_size(size);
    }
    if let Some(ref color) = def.bg_color {
        if let Ok(c) = parse_color(color) {
            format = format.set_background_color(c.as_str());
        }
    }
    if let Some(ref nf) = def.num_format {
        format = format.set_num_format(nf);
    }
    if let Some(true) = def.border {
        format = format.set_border(FormatBorder::Thin);
    }
    if let Some(ref align) = def.align_h {
        match align.to_lowercase().as_str() {
            "left" => { format = format.set_align(FormatAlign::Left); }
            "center" => { format = format.set_align(FormatAlign::Center); }
            "right" => { format = format.set_align(FormatAlign::Right); }
            _ => {}
        }
    }
    if let Some(ref align) = def.align_v {
        match align.to_lowercase().as_str() {
            "top" => { format = format.set_align(FormatAlign::Top); }
            "center" | "middle" => { format = format.set_align(FormatAlign::VerticalCenter); }
            "bottom" => { format = format.set_align(FormatAlign::Bottom); }
            _ => {}
        }
    }
    if let Some(true) = def.wrap {
        format = format.set_text_wrap();
    }
    if let Some(rot) = def.text_rotation {
        format = format.set_rotation(rot);
    }
    format
}

fn parse_color(s: &str) -> Result<String, String> {
    let hex = s.trim_start_matches('#');
    if hex.len() == 6 || hex.len() == 8 {
        Ok(hex.to_uppercase())
    } else {
        Err(format!("invalid color: {}", s))
    }
}

fn parse_table_style(name: &str) -> Option<TableStyle> {
    match name.to_lowercase().as_str() {
        "none" => Some(TableStyle::None),
        "light1" => Some(TableStyle::Light1),
        "light2" => Some(TableStyle::Light2),
        "light3" => Some(TableStyle::Light3),
        "light4" => Some(TableStyle::Light4),
        "light5" => Some(TableStyle::Light5),
        "light6" => Some(TableStyle::Light6),
        "light7" => Some(TableStyle::Light7),
        "light8" => Some(TableStyle::Light8),
        "light9" => Some(TableStyle::Light9),
        "light10" => Some(TableStyle::Light10),
        "light11" => Some(TableStyle::Light11),
        "light12" => Some(TableStyle::Light12),
        "light13" => Some(TableStyle::Light13),
        "light14" => Some(TableStyle::Light14),
        "light15" => Some(TableStyle::Light15),
        "light16" => Some(TableStyle::Light16),
        "light17" => Some(TableStyle::Light17),
        "light18" => Some(TableStyle::Light18),
        "light19" => Some(TableStyle::Light19),
        "light20" => Some(TableStyle::Light20),
        "light21" => Some(TableStyle::Light21),
        "medium1" => Some(TableStyle::Medium1),
        "medium2" => Some(TableStyle::Medium2),
        "medium3" => Some(TableStyle::Medium3),
        "medium4" => Some(TableStyle::Medium4),
        "medium5" => Some(TableStyle::Medium5),
        "medium6" => Some(TableStyle::Medium6),
        "medium7" => Some(TableStyle::Medium7),
        "medium8" => Some(TableStyle::Medium8),
        "medium9" => Some(TableStyle::Medium9),
        "medium10" => Some(TableStyle::Medium10),
        "medium11" => Some(TableStyle::Medium11),
        "medium12" => Some(TableStyle::Medium12),
        "medium13" => Some(TableStyle::Medium13),
        "medium14" => Some(TableStyle::Medium14),
        "medium15" => Some(TableStyle::Medium15),
        "medium16" => Some(TableStyle::Medium16),
        "medium17" => Some(TableStyle::Medium17),
        "medium18" => Some(TableStyle::Medium18),
        "medium19" => Some(TableStyle::Medium19),
        "medium20" => Some(TableStyle::Medium20),
        "medium21" => Some(TableStyle::Medium21),
        "medium22" => Some(TableStyle::Medium22),
        "medium23" => Some(TableStyle::Medium23),
        "medium24" => Some(TableStyle::Medium24),
        "medium25" => Some(TableStyle::Medium25),
        "medium26" => Some(TableStyle::Medium26),
        "medium27" => Some(TableStyle::Medium27),
        "medium28" => Some(TableStyle::Medium28),
        "dark1" => Some(TableStyle::Dark1),
        "dark2" => Some(TableStyle::Dark2),
        "dark3" => Some(TableStyle::Dark3),
        "dark4" => Some(TableStyle::Dark4),
        "dark5" => Some(TableStyle::Dark5),
        "dark6" => Some(TableStyle::Dark6),
        "dark7" => Some(TableStyle::Dark7),
        "dark8" => Some(TableStyle::Dark8),
        "dark9" => Some(TableStyle::Dark9),
        "dark10" => Some(TableStyle::Dark10),
        "dark11" => Some(TableStyle::Dark11),
        _ => None,
    }
}

fn col_to_letter(col: u16) -> String {
    let mut n = col;
    let mut s = String::new();
    loop {
        let rem = n % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        n /= 26;
        if n == 0 {
            break;
        }
        n -= 1;
    }
    s
}
