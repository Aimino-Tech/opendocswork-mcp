use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use rdocx::{Alignment, BorderStyle, Document, Length};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::writers::docx_enricher;

/// Fix rdocx bug: it writes a duplicate opening `<Relationships>` tag
/// in `word/_rels/document.xml.rels`. This breaks LibreOffice and some
/// Word versions. We open the zip, fix the XML inline, and rewrite it.
fn fix_docx_rels(file_path: &str) -> Result<(), String> {
    let fp = Path::new(file_path);
    let f = std::fs::File::open(fp).map_err(|e| format!("open: {e}"))?;
    let mut arc = ZipArchive::new(f).map_err(|e| format!("zip: {e}"))?;
    let mut entries: Vec<(String, Vec<u8>)> = Vec::with_capacity(arc.len());
    for i in 0..arc.len() {
        let mut e = arc.by_index(i).map_err(|e| format!("entry {i}: {e}"))?;
        let name = e.name().to_string();
        let mut data = Vec::new();
        e.read_to_end(&mut data)
            .map_err(|e| format!("read {name}: {e}"))?;
        if name == "word/_rels/document.xml.rels" {
            let s = String::from_utf8_lossy(&data);
            // Fix: duplicate <Relationships> opening tag
            // e.g. "<Relationships ...><Relationships ...>" → "<Relationships ...>"
            let fixed = if let Some(pos) = s[1..].find("<Relationships") {
                let s2 = s[1 + pos..].to_string();
                // Second opening tag found — strip everything before it
                // but keep the xmlns attr from the first
                let first_end = s[..7 + pos].find('>').unwrap_or(0);
                let prefix = &s[..first_end + 1];
                // Remove the second opening tag entirely
                let second_close = s2.find('>').unwrap_or(0);
                let rest = &s2[second_close + 1..];
                format!("{}{}", prefix, rest)
            } else {
                s.to_string()
            };
            data = fixed.into_bytes();
        }
        entries.push((name, data));
    }
    drop(arc);
    // Rewrite the zip
    let tmp = fp.with_extension("docx.tmp");
    let f = std::fs::File::create(&tmp).map_err(|e| format!("create: {e}"))?;
    let mut zw = ZipWriter::new(f);
    for (name, data) in &entries {
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zw.start_file::<&str, _>(name, opts)
            .map_err(|e| format!("start {name}: {e}"))?;
        zw.write_all(data)
            .map_err(|e| format!("write {name}: {e}"))?;
    }
    zw.finish().map_err(|e| format!("finish: {e}"))?;
    std::fs::rename(&tmp, fp).map_err(|e| format!("rename: {e}"))?;
    Ok(())
}

fn currency_symbol(code: &str) -> &str {
    match code {
        "USD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "JPY" => "¥",
        "VND" => "₫",
        _ => "$",
    }
}

fn set_margins_one_inch(doc: &mut Document) -> Result<(), String> {
    let m = Length::inches(1.0);
    doc.set_margins(m, m, m, m);
    Ok(())
}

/// Set formatted text in a table cell using the add_paragraph + run API.
/// The cell must be freshly created (no existing paragraphs) for best results.
fn cell_set_text(
    cell: &mut rdocx::table::Cell,
    text: &str,
    bold: bool,
    color: &str,
    size: f64,
    font_name: &str,
) {
    let mut para = cell.add_paragraph("");
    let mut run = para.add_run(text);
    if bold {
        run = run.bold(true);
    }
    if !color.is_empty() {
        run = run.color(color);
    }
    if size > 0.0 {
        run = run.size(size);
    }
    if !font_name.is_empty() {
        let _ = run.font(font_name);
    }
}

pub fn write_word_report(
    file_path: &str,
    title: &str,
    author: &str,
    sections: &[serde_json::Value],
    _include_toc: bool,
    include_page_numbers: bool,
    include_header: bool,
    _theme: &str,
    date: &str,
) -> Result<PathBuf, String> {
    let mut doc = Document::new();
    doc.set_title(title);
    if !author.is_empty() {
        doc.set_author(author);
    }

    set_margins_one_inch(&mut doc)?;

    if include_header {
        doc.set_header(title);
    }

    if include_page_numbers {
        doc.set_footer("Page ");
    }

    doc.add_paragraph(title)
        .style("Heading1")
        .space_after(Length::twips(200));

    if !author.is_empty() || !date.is_empty() {
        let mut p = doc.add_paragraph("");
        if !author.is_empty() {
            p.add_run(&format!("By: {}", author))
                .italic(true)
                .color("555555");
        }
        if !date.is_empty() {
            if !author.is_empty() {
                p.add_run("  |  ");
            }
            p.add_run(&format!("Date: {}", date))
                .italic(true)
                .color("555555");
        }
    }

    for section in sections {
        let heading = section
            .get("heading")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = section
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let level = section.get("level").and_then(|v| v.as_i64()).unwrap_or(1);

        if !heading.is_empty() {
            let style = match level {
                1 => "Heading1",
                2 => "Heading2",
                3 => "Heading3",
                _ => "Heading1",
            };
            doc.add_paragraph(heading).style(style);
        }

        if !content.is_empty() {
            for line in content.split('\n') {
                doc.add_paragraph(line).space_after(Length::twips(120));
            }
        }
    }

    doc.save(file_path)
        .map_err(|e| format!("Failed to save document: {}", e))?;

    fix_docx_rels(file_path)?;
    docx_enricher::enrich_docx(
        file_path,
        title,
        author,
        include_page_numbers,
        include_header,
    )?;

    Ok(PathBuf::from(file_path))
}

pub fn write_word_mailmerge(
    output_dir: &str,
    template_text: &str,
    data_source: &[serde_json::Value],
    filename_prefix: &str,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();

    for (idx, record) in data_source.iter().enumerate() {
        let mut doc = Document::new();
        set_margins_one_inch(&mut doc)?;

        let mut body = template_text.to_string();
        if let Some(obj) = record.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{{{}}}}}", key.to_uppercase());
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => value.to_string(),
                };
                body = body.replace(&placeholder, &replacement);
            }
        }

        for line in body.split('\n') {
            doc.add_paragraph(line.trim())
                .space_after(Length::twips(120));
        }

        let filename = format!("{}_{}.docx", filename_prefix, idx + 1);
        let path = std::path::Path::new(output_dir).join(&filename);
        let path_str = path.to_string_lossy().to_string();
        doc.save(&path)
            .map_err(|e| format!("Failed to save document: {}", e))?;

        fix_docx_rels(&path_str)?;
        docx_enricher::enrich_docx(&path_str, "Document", "", false, false)?;

        paths.push(path);
    }

    Ok(paths)
}

pub fn write_word_invoice(
    file_path: &str,
    params: &serde_json::Map<String, serde_json::Value>,
) -> Result<PathBuf, String> {
    let mut doc = Document::new();
    set_margins_one_inch(&mut doc)?;

    let invoice_number = params
        .get("invoice_number")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let date = params.get("date").and_then(|v| v.as_str()).unwrap_or("");
    let due_date = params
        .get("due_date")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let from_company = params
        .get("from_company")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let from_address = params
        .get("from_address")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let from_email = params
        .get("from_email")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let to_company = params
        .get("to_company")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let to_address = params
        .get("to_address")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let to_email = params
        .get("to_email")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let line_items = params
        .get("line_items")
        .and_then(|v| v.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[]);
    let tax_rate = params
        .get("tax_rate")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let tax_label = params
        .get("tax_label")
        .and_then(|v| v.as_str())
        .unwrap_or("Tax");
    let currency = params
        .get("currency")
        .and_then(|v| v.as_str())
        .unwrap_or("USD");
    let payment_terms = params
        .get("payment_terms")
        .and_then(|v| v.as_str())
        .unwrap_or("Net 30");
    let notes = params.get("notes").and_then(|v| v.as_str()).unwrap_or("");
    let cur_sym = currency_symbol(currency);

    // ── Title: INVOICE ──
    {
        let mut p = doc.add_paragraph("");
        p = p
            .alignment(Alignment::Center)
            .space_after(Length::twips(200));
        p.add_run("INVOICE")
            .bold(true)
            .size(28.0)
            .font("Calibri Light")
            .color("1F3864");
    }

    // ── Info table (Invoice #, Date, Due Date) ──
    {
        let mut tbl = doc.add_table(3, 2).width_pct(100.0).layout_fixed();

        // Right-align the info table
        tbl = tbl.alignment(Alignment::Right);

        // Row 0: Invoice Number
        {
            let mut cell = tbl.cell(0, 0).unwrap();
            cell = cell.width(Length::inches(1.5));
            cell_set_text(
                &mut cell,
                "Invoice Number:",
                true,
                "1F3864",
                10.0,
                "Calibri",
            );
        }
        {
            let mut cell = tbl.cell(0, 1).unwrap();
            cell = cell.width(Length::inches(2.0));
            cell_set_text(&mut cell, invoice_number, false, "333333", 10.0, "Calibri");
        }

        // Row 1: Date
        {
            let mut cell = tbl.cell(1, 0).unwrap();
            cell_set_text(&mut cell, "Date:", true, "1F3864", 10.0, "Calibri");
        }
        {
            let mut cell = tbl.cell(1, 1).unwrap();
            let date_text = if date.is_empty() { "—" } else { date };
            cell_set_text(&mut cell, date_text, false, "333333", 10.0, "Calibri");
        }

        // Row 2: Due Date
        {
            let mut cell = tbl.cell(2, 0).unwrap();
            cell_set_text(&mut cell, "Due Date:", true, "1F3864", 10.0, "Calibri");
        }
        {
            let mut cell = tbl.cell(2, 1).unwrap();
            let due_text = if due_date.is_empty() { "—" } else { due_date };
            cell_set_text(&mut cell, due_text, false, "333333", 10.0, "Calibri");
        }
    }

    doc.add_paragraph("");

    // ── FROM section ──
    {
        doc.add_paragraph("")
            .style("Heading2")
            .space_before(Length::twips(120));
        // Heading2 style provides bold text; add a formatted paragraph for the label
        let mut label = doc.add_paragraph("");
        label.add_run("FROM").bold(true).color("1F3864").size(11.0);
    }
    doc.add_paragraph(from_company)
        .space_after(Length::twips(40));
    doc.add_paragraph(from_address)
        .space_after(Length::twips(40));
    if !from_email.is_empty() {
        doc.add_paragraph(from_email).space_after(Length::twips(40));
    }

    doc.add_paragraph("");

    // ── TO section ──
    {
        doc.add_paragraph("")
            .style("Heading2")
            .space_before(Length::twips(120));
        let mut label = doc.add_paragraph("");
        label.add_run("TO").bold(true).color("1F3864").size(11.0);
    }
    doc.add_paragraph(to_company).space_after(Length::twips(40));
    doc.add_paragraph(to_address).space_after(Length::twips(40));
    if !to_email.is_empty() {
        doc.add_paragraph(to_email).space_after(Length::twips(40));
    }

    doc.add_paragraph("");

    // ── Line Items Table ──
    // Columns: # (0.5in), Description (2.5in), Qty (1in), Unit Price (1.2in), Amount (1.3in)
    let col_widths = [0.5, 2.5, 1.0, 1.2, 1.3];
    let item_count = line_items.len();
    let data_rows = item_count;
    let total_rows = 1 + data_rows + 4; // header + items + subtotal + tax + empty + total

    {
        let mut tbl = doc
            .add_table(total_rows, 5)
            .width_pct(100.0)
            .borders(BorderStyle::Single, 4, "AAAAAA")
            .layout_fixed();

        // Helper to format a cell in the table
        let set_header_cell =
            |tbl: &mut rdocx::table::Table, row: usize, col: usize, text: &str, w: f64| {
                let mut cell = tbl.cell(row, col).unwrap();
                cell = cell.width(Length::inches(w)).shading("1F3864");
                cell_set_text(&mut cell, text, true, "FFFFFF", 10.0, "Calibri");
            };

        // Header row
        tbl.row(0).unwrap().header();
        set_header_cell(&mut tbl, 0, 0, "#", col_widths[0]);
        set_header_cell(&mut tbl, 0, 1, "Description", col_widths[1]);
        set_header_cell(&mut tbl, 0, 2, "Qty", col_widths[2]);
        set_header_cell(&mut tbl, 0, 3, "Unit Price", col_widths[3]);
        set_header_cell(&mut tbl, 0, 4, "Amount", col_widths[4]);

        // Data rows
        let mut grand_total = 0.0_f64;
        for (i, item) in line_items.iter().enumerate() {
            let row = 1 + i;
            let seq = i + 1;
            let desc = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let qty = item.get("quantity").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let price = item
                .get("unit_price")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let total = qty * price;
            grand_total += total;

            let row_bg = if i % 2 == 0 { "F2F2F2" } else { "FFFFFF" };

            // # column
            {
                let mut cell = tbl.cell(row, 0).unwrap();
                cell = cell.width(Length::inches(col_widths[0])).shading(row_bg);
                cell_set_text(
                    &mut cell,
                    &seq.to_string(),
                    false,
                    "333333",
                    10.0,
                    "Calibri",
                );
            }
            // Description
            {
                let mut cell = tbl.cell(row, 1).unwrap();
                cell = cell.width(Length::inches(col_widths[1])).shading(row_bg);
                cell_set_text(&mut cell, desc, false, "333333", 10.0, "Calibri");
            }
            // Qty
            {
                let mut cell = tbl.cell(row, 2).unwrap();
                cell = cell.width(Length::inches(col_widths[2])).shading(row_bg);
                cell_set_text(
                    &mut cell,
                    &format!("{}", qty),
                    false,
                    "333333",
                    10.0,
                    "Calibri",
                );
            }
            // Unit Price
            {
                let mut cell = tbl.cell(row, 3).unwrap();
                cell = cell.width(Length::inches(col_widths[3])).shading(row_bg);
                cell_set_text(
                    &mut cell,
                    &format!("{}{:.2}", cur_sym, price),
                    false,
                    "333333",
                    10.0,
                    "Calibri",
                );
            }
            // Amount
            {
                let mut cell = tbl.cell(row, 4).unwrap();
                cell = cell.width(Length::inches(col_widths[4])).shading(row_bg);
                cell_set_text(
                    &mut cell,
                    &format!("{}{:.2}", cur_sym, total),
                    false,
                    "333333",
                    10.0,
                    "Calibri",
                );
            }
        }

        // Summary rows
        let subtotal_row = 1 + item_count;
        let subtotal = grand_total;
        let tax_amount = subtotal * tax_rate;
        let total = subtotal + tax_amount;

        // Subtotal row
        for col in 0..3usize {
            let mut cell = tbl.cell(subtotal_row, col).unwrap();
            cell = cell.shading("FFFFFF");
            cell_set_text(&mut cell, "", false, "", 0.0, "");
        }
        {
            let mut cell = tbl.cell(subtotal_row, 3).unwrap();
            cell = cell.shading("FFFFFF");
            cell_set_text(&mut cell, "Subtotal:", true, "333333", 10.0, "Calibri");
        }
        {
            let mut cell = tbl.cell(subtotal_row, 4).unwrap();
            cell = cell.shading("FFFFFF");
            cell_set_text(
                &mut cell,
                &format!("{}{:.2}", cur_sym, subtotal),
                true,
                "333333",
                10.0,
                "Calibri",
            );
        }

        // Tax row
        let tax_row = subtotal_row + 1;
        for col in 0..3usize {
            let mut cell = tbl.cell(tax_row, col).unwrap();
            cell = cell.shading("FFFFFF");
            cell_set_text(&mut cell, "", false, "", 0.0, "");
        }
        {
            let mut cell = tbl.cell(tax_row, 3).unwrap();
            cell = cell.shading("FFFFFF");
            let tax_pct = (tax_rate * 100.0) as i32;
            cell_set_text(
                &mut cell,
                &format!("{} ({}%)", tax_label, tax_pct),
                true,
                "333333",
                10.0,
                "Calibri",
            );
        }
        {
            let mut cell = tbl.cell(tax_row, 4).unwrap();
            cell = cell.shading("FFFFFF");
            cell_set_text(
                &mut cell,
                &format!("{}{:.2}", cur_sym, tax_amount),
                true,
                "333333",
                10.0,
                "Calibri",
            );
        }

        // Empty spacer row
        let spacer_row = tax_row + 1;
        for col in 0..5usize {
            let mut cell = tbl.cell(spacer_row, col).unwrap();
            cell = cell.shading("FFFFFF");
            cell_set_text(&mut cell, "", false, "", 0.0, "");
        }

        // Total Due row (highlighted)
        let total_row = spacer_row + 1;
        for col in 0..3usize {
            let mut cell = tbl.cell(total_row, col).unwrap();
            cell = cell.shading("E8F0FE");
            cell_set_text(&mut cell, "", false, "", 0.0, "");
        }
        {
            let mut cell = tbl.cell(total_row, 3).unwrap();
            cell = cell.shading("E8F0FE");
            cell_set_text(&mut cell, "Total Due:", true, "1F3864", 12.0, "Calibri");
        }
        {
            let mut cell = tbl.cell(total_row, 4).unwrap();
            cell = cell.shading("E8F0FE");
            cell_set_text(
                &mut cell,
                &format!("{}{:.2}", cur_sym, total),
                true,
                "1F3864",
                12.0,
                "Calibri",
            );
        }
    }

    // ── Payment Terms ──
    doc.add_paragraph("");
    {
        doc.add_paragraph("")
            .style("Heading2")
            .space_before(Length::twips(120));
        let mut label = doc.add_paragraph("");
        label
            .add_run("Payment Terms")
            .bold(true)
            .color("1F3864")
            .size(11.0);
    }
    doc.add_paragraph(payment_terms)
        .space_after(Length::twips(120));

    // ── Notes ──
    if !notes.is_empty() {
        {
            doc.add_paragraph("")
                .style("Heading2")
                .space_before(Length::twips(120));
            let mut label = doc.add_paragraph("");
            label.add_run("Notes").bold(true).color("1F3864").size(11.0);
        }
        doc.add_paragraph(notes).space_after(Length::twips(120));
    }

    doc.save(file_path)
        .map_err(|e| format!("Failed to save invoice document: {}", e))?;

    fix_docx_rels(file_path)?;
    docx_enricher::enrich_docx(file_path, "INVOICE", "", true, false)?;

    Ok(PathBuf::from(file_path))
}

pub fn write_word_from_md(
    file_path: &str,
    markdown_text: &str,
    title: &str,
    _theme: &str,
    _include_toc: bool,
    include_page_numbers: bool,
) -> Result<PathBuf, String> {
    let mut doc = Document::new();
    doc.set_title(title);

    set_margins_one_inch(&mut doc)?;

    doc.add_paragraph(title)
        .style("Heading1")
        .space_after(Length::twips(200));

    if include_page_numbers {
        doc.set_footer("Page ");
    }

    for line in markdown_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            doc.add_paragraph("").space_after(Length::twips(60));
            continue;
        }

        if trimmed.starts_with("### ") {
            doc.add_paragraph(&trimmed[4..]).style("Heading3");
        } else if trimmed.starts_with("## ") {
            doc.add_paragraph(&trimmed[3..]).style("Heading2");
        } else if trimmed.starts_with("# ") {
            doc.add_paragraph(&trimmed[2..]).style("Heading1");
        } else if trimmed.starts_with("---") || trimmed.starts_with("***") {
            doc.add_paragraph("————————————————————");
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            doc.add_paragraph(text).indent_left(Length::inches(0.5));
        } else {
            doc.add_paragraph(trimmed).space_after(Length::twips(120));
        }
    }

    doc.save(file_path)
        .map_err(|e| format!("Failed to save document: {}", e))?;

    fix_docx_rels(file_path)?;
    docx_enricher::enrich_docx(file_path, title, "", include_page_numbers, true)?;

    Ok(PathBuf::from(file_path))
}
