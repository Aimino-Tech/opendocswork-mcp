use std::io::{Read, Write, Cursor};
use std::path::PathBuf;

use super::types::*;

#[derive(Debug)]
pub struct TemplateEngine {
    template_dir: PathBuf,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let dir = PathBuf::from(
            std::env::var("OFFICE_TEMPLATE_DIR")
                .unwrap_or_else(|_| "templates".to_string())
        );
        Self { template_dir: dir }
    }

    pub async fn execute(
        &self,
        skill: &SkillDefinition,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> anyhow::Result<SkillRunResult> {
        let template_path = self.resolve_template(skill)?;
        let template_bytes = match tokio::fs::read(&template_path).await {
            Ok(bytes) => bytes,
            Err(_) => self.generate_default_template(skill),
        };

        let output_path = self.create_output_path(skill);
        let processed = self.fill_template(&template_bytes, skill, params)?;

        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&output_path, &processed).await?;

        let file_path = output_path.to_string_lossy().to_string();
        let output_value = self.build_output(skill, params);

        Ok(SkillRunResult {
            skill_name: skill.name.clone(),
            success: true,
            output: output_value,
            file_paths: vec![file_path],
            warnings: vec![],
        })
    }

    fn resolve_template(&self, skill: &SkillDefinition) -> anyhow::Result<PathBuf> {
        let template_name = format!("{}.{}", skill.templates.primary, skill.format);
        Ok(self.template_dir.join(&template_name))
    }

    fn generate_default_template(&self, skill: &SkillDefinition) -> Vec<u8> {
        generate_ooxml_template(skill)
    }

    fn create_output_path(&self, skill: &SkillDefinition) -> PathBuf {
        let out_dir = PathBuf::from(
            std::env::var("OFFICE_OUTPUT_DIR")
                .unwrap_or_else(|_| "output".to_string())
        );
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        out_dir.join(format!("{}_{}.{}", skill.name.replace('.', "_"), timestamp, skill.format))
    }

    fn fill_template(
        &self,
        template_bytes: &[u8],
        skill: &SkillDefinition,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> anyhow::Result<Vec<u8>> {
        let mut archive = zip::ZipArchive::new(Cursor::new(template_bytes))
            .map_err(|e| anyhow::anyhow!("Failed to read template ZIP: {}", e))?;

        let mut out_bytes = Vec::new();
        let mut out_zip = zip::ZipWriter::new(Cursor::new(&mut out_bytes));

        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let entry_name = entry.name().to_string();
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;

            let filled = self.replace_placeholders(&content, skill, params);

            out_zip.start_file(&entry_name, options)?;
            out_zip.write_all(&filled)?;
        }

        out_zip.finish()?;
        Ok(out_bytes)
    }

    fn replace_placeholders(
        &self,
        content: &[u8],
        _skill: &SkillDefinition,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> Vec<u8> {
        let mut text = String::from_utf8_lossy(content).to_string();

        for (key, value) in params.iter() {
            let placeholder = format!("{{{{{}}}}}", key.to_uppercase());
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Array(arr) => serde_json::to_string_pretty(arr).unwrap_or_default(),
                serde_json::Value::Object(obj) => serde_json::to_string_pretty(obj).unwrap_or_default(),
                serde_json::Value::Null => String::new(),
            };
            text = text.replace(&placeholder, &replacement);
        }

        text.into_bytes()
    }

    fn build_output(
        &self,
        skill: &SkillDefinition,
        params: &serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        let mut output = serde_json::Map::new();

        for out in &skill.outputs {
            match out.name.as_str() {
                "subtotal" | "tax_amount" | "total" | "rows_written" | "line_item_count"
                | "slide_count" | "record_count" | "columns" | "word_count"
                | "word_count_estimate" | "section_count" => {
                    output.insert(out.name.clone(), self.compute_numeric_output(skill, params, &out.name));
                }
                "series_count" => {
                    let count = params.get("data")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len().saturating_sub(1))
                        .unwrap_or(0);
                    output.insert(out.name.clone(), serde_json::json!(count));
                }
                "file_path" | "chart_type" => {
                    output.insert(out.name.clone(), serde_json::json!("generated_file"));
                }
                "row_fields" | "value_fields" => {
                    output.insert(out.name.clone(), params.get(&out.name).cloned().unwrap_or(serde_json::json!([])));
                }
                "aggregate" => {
                    output.insert(out.name.clone(), params.get("aggregate").cloned().unwrap_or(serde_json::json!("sum")));
                }
                "fields_found" | "headings" => {
                    output.insert(out.name.clone(), serde_json::json!([]));
                }
                "slide_types" => {
                    let types: Vec<&str> = params.get("slides")
                        .and_then(|v| v.as_array())
                        .map(|slides| {
                            slides.iter()
                                .filter_map(|s| s.get("type").and_then(|t| t.as_str()))
                                .collect()
                        })
                        .unwrap_or_default();
                    output.insert(out.name.clone(), serde_json::json!(types));
                }
                "themes_applied" => {
                    let theme = params.get("theme").and_then(|v| v.as_str()).unwrap_or("light");
                    output.insert(out.name.clone(), serde_json::json!({
                        "theme": theme,
                        "background": if theme == "dark" { "#1E1E1E" } else { "#FFFFFF" },
                        "text": if theme == "dark" { "#CCCCCC" } else { "#333333" }
                    }));
                }
                _ => {
                    output.insert(out.name.clone(), serde_json::json!(null));
                }
            }
        }

        serde_json::Value::Object(output)
    }

    fn compute_numeric_output(
        &self,
        skill: &SkillDefinition,
        params: &serde_json::Map<String, serde_json::Value>,
        output_name: &str,
    ) -> serde_json::Value {
        match output_name {
            "subtotal" => {
                let items = params.get("line_items").and_then(|v| v.as_array());
                let subtotal: f64 = items.map(|items| {
                    items.iter().filter_map(|item| {
                        let qty = item.get("quantity").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let price = item.get("unit_price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        Some(qty * price)
                    }).sum()
                }).unwrap_or(0.0);
                serde_json::json!(subtotal)
            }
            "tax_amount" => {
                let subtotal = self.compute_numeric_output(skill, params, "subtotal");
                let tax_rate = params.get("tax_rate").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let amount = subtotal.as_f64().unwrap_or(0.0) * tax_rate;
                serde_json::json!(amount)
            }
            "total" => {
                let subtotal = self.compute_numeric_output(skill, params, "subtotal");
                let tax = self.compute_numeric_output(skill, params, "tax_amount");
                let total = subtotal.as_f64().unwrap_or(0.0) + tax.as_f64().unwrap_or(0.0);
                serde_json::json!(total)
            }
            "rows_written" => {
                let count = params.get("data")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                serde_json::json!(count)
            }
            "line_item_count" => {
                let count = params.get("line_items")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                serde_json::json!(count)
            }
            "slide_count" => {
                let count = params.get("slides")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                serde_json::json!(count)
            }
            "record_count" => {
                let count = params.get("data_source")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                serde_json::json!(count)
            }
            "columns" => {
                let count = params.get("data")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|r| r.as_array())
                    .map(|r| r.len())
                    .unwrap_or(0);
                serde_json::json!(count)
            }
            "section_count" => {
                let count = params.get("sections")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                serde_json::json!(count)
            }
            "word_count" | "word_count_estimate" => {
                let count = params.get("markdown_text")
                    .or_else(|| params.get("template_text"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.split_whitespace().count())
                    .unwrap_or(0);
                serde_json::json!(count)
            }
            _ => serde_json::json!(0),
        }
    }
}

pub fn generate_ooxml_template(skill: &SkillDefinition) -> Vec<u8> {
    match skill.format.as_str() {
        "xlsx" => generate_xlsx_template(skill),
        "docx" => generate_docx_template(skill),
        "pptx" => generate_pptx_template(skill),
        _ => Vec::new(),
    }
}

fn generate_xlsx_template(_skill: &SkillDefinition) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
        let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
  <Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
  <Override PartName="/xl/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

        let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

        let wb = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="{{SHEET_NAME}}" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#;

        let wb_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>"#;

        let sheet = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetViews>
    <sheetView tabSelected="1" workbookViewId="0"/>
  </sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <cols>
    <col min="1" max="16" width="15" customWidth="1"/>
  </cols>
  <sheetData>
    <row r="1">
      <c r="A1" t="s"><v>0</v></c>
    </row>
  </sheetData>
  <autoFilter ref="A1:A1"/>
</worksheet>"#;

        let ss = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1" uniqueCount="1">
  <si><t>{{DATA}}</t></si>
</sst>"#;

        let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="2">
    <font><sz val="11"/><name val="Calibri"/></font>
    <font><b/><sz val="11"/><color rgb="FFFFFFFF"/><name val="Calibri"/></font>
  </fonts>
  <fills count="3">
    <fill><patternFill patternType="none"/></fill>
    <fill><patternFill patternType="gray125"/></fill>
    <fill><patternFill><fgColor rgb="FF4472C4"/></patternFill></fill>
  </fills>
  <borders count="2">
    <border><left/><right/><top/><bottom/><diagonal/></border>
    <border><left style="thin"><color auto="1"/></left><right style="thin"><color auto="1"/></right><top style="thin"><color auto="1"/></top><bottom style="thin"><color auto="1"/></bottom><diagonal/></border>
  </borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="2">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="0" fontId="1" fillId="2" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1"/>
  </cellXfs>
</styleSheet>"#;

        let theme = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
  </a:themeElements>
</a:theme>"#;

        zip.add_directory("_rels", opts).unwrap();
        zip.add_directory("xl", opts).unwrap();
        zip.add_directory("xl/_rels", opts).unwrap();
        zip.add_directory("xl/worksheets", opts).unwrap();
        zip.add_directory("xl/theme", opts).unwrap();

        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(content_types.as_bytes()).unwrap();
        zip.start_file("_rels/.rels", opts).unwrap();
        zip.write_all(rels.as_bytes()).unwrap();
        zip.start_file("xl/workbook.xml", opts).unwrap();
        zip.write_all(wb.as_bytes()).unwrap();
        zip.start_file("xl/_rels/workbook.xml.rels", opts).unwrap();
        zip.write_all(wb_rels.as_bytes()).unwrap();
        zip.start_file("xl/worksheets/sheet1.xml", opts).unwrap();
        zip.write_all(sheet.as_bytes()).unwrap();
        zip.start_file("xl/styles.xml", opts).unwrap();
        zip.write_all(styles.as_bytes()).unwrap();
        zip.start_file("xl/sharedStrings.xml", opts).unwrap();
        zip.write_all(ss.as_bytes()).unwrap();
        zip.start_file("xl/theme/theme1.xml", opts).unwrap();
        zip.write_all(theme.as_bytes()).unwrap();
    }
    buf
}

fn generate_docx_template(_skill: &SkillDefinition) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
        let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
  <Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
  <Override PartName="/word/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/word/settings.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"/>
  <Override PartName="/word/fontTable.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml"/>
</Types>"#;

        let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

        let doc = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
  <w:body>
    <w:p>
      <w:pPr><w:jc w:val="center"/></w:pPr>
      <w:r><w:rPr><w:b/><w:sz w:val="56"/><w:color w:val="1F3864"/></w:rPr><w:t>{{TITLE}}</w:t></w:r>
    </w:p>
    <w:p><w:r><w:t>{{CONTENT}}</w:t></w:r></w:p>
    <w:sectPr>
      <w:pgSz w:w="12240" w:h="15840"/>
      <w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720"/>
    </w:sectPr>
  </w:body>
</w:document>"#;

        let doc_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings" Target="settings.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable" Target="fontTable.xml"/>
</Relationships>"#;

        let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
    <w:pPr><w:spacing w:after="120" w:line="276" w:lineRule="auto"/></w:pPr>
    <w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri"/><w:sz w:val="22"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:pPr><w:spacing w:before="480" w:after="240"/></w:pPr>
    <w:rPr><w:b/><w:color w:val="1F3864"/><w:sz w:val="36"/><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri"/></w:rPr>
  </w:style>
</w:styles>"#;

        let theme = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
  </a:themeElements>
</a:theme>"#;

        let settings = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:zoom w:percent="100"/>
  <w:defaultTabStop w:val="720"/>
</w:settings>"#;

        let font_table = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:fonts xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:font w:name="Calibri"><w:panose1 w:val="020F0502020204030204"/><w:charset w:val="00"/></w:font>
</w:fonts>"#;

        zip.add_directory("_rels", opts).unwrap();
        zip.add_directory("word", opts).unwrap();
        zip.add_directory("word/_rels", opts).unwrap();
        zip.add_directory("word/theme", opts).unwrap();

        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(content_types.as_bytes()).unwrap();
        zip.start_file("_rels/.rels", opts).unwrap();
        zip.write_all(rels.as_bytes()).unwrap();
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(doc.as_bytes()).unwrap();
        zip.start_file("word/_rels/document.xml.rels", opts).unwrap();
        zip.write_all(doc_rels.as_bytes()).unwrap();
        zip.start_file("word/styles.xml", opts).unwrap();
        zip.write_all(styles.as_bytes()).unwrap();
        zip.start_file("word/theme/theme1.xml", opts).unwrap();
        zip.write_all(theme.as_bytes()).unwrap();
        zip.start_file("word/settings.xml", opts).unwrap();
        zip.write_all(settings.as_bytes()).unwrap();
        zip.start_file("word/fontTable.xml", opts).unwrap();
        zip.write_all(font_table.as_bytes()).unwrap();
    }
    buf
}

fn generate_pptx_template(_skill: &SkillDefinition) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
        let opts: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/ppt/presProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presProps+xml"/>
  <Override PartName="/ppt/tableStyles.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"/>
  <Override PartName="/ppt/viewProps.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.viewProps+xml"/>
</Types>"#;

        let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#;

        let pres = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationshps">
  <p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#;

        let pres_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/presProps" Target="presProps.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles" Target="tableStyles.xml"/>
  <Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/viewProps" Target="viewProps.xml"/>
  <Relationship Id="rId6" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#;

        let slide = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:nvPr/><p:cNvPr id="1" name=""/><p:nvGrpSpPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="457200" y="457200"/><a:ext cx="8229600" cy="1371600"/></a:xfrm></p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:p><a:r><a:rPr lang="en-US" sz="4400" b="1" solidFill="1F3864"/><a:t>{{TITLE}}</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="3" name="Content"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr/></p:nvSpPr>
        <p:spPr><a:xfrm><a:off x="457200" y="1828800"/><a:ext cx="8229600 cy="4572000"/></a:xfrm></p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:p><a:r><a:rPr lang="en-US" sz="1800"/><a:t>{{CONTENT}}</a:t></a:r></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"#;

        let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#;

        let master = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:nvPr/><p:cNvPr id="1" name=""/><p:nvGrpSpPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

        let layout = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             type="blank">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:nvPr/><p:cNvPr id="1" name=""/><p:nvGrpSpPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sldLayout>"#;

        let theme = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?">
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
  </a:themeElements>
</a:theme>"#;

        let pres_props = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presProps xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"/>
"#;
        let table_styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def="{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}"/>
"#;
        let view_props = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:viewPr xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:slideViewPr><p:cSldViewPr><p:cViewPr varScale="1"/></p:cSldViewPr></p:slideViewPr>
  <p:notesTextViewPr><p:cViewPr/></p:notesTextViewPr>
</p:viewPr>"#;

        zip.add_directory("_rels", opts).unwrap();
        zip.add_directory("ppt", opts).unwrap();
        zip.add_directory("ppt/_rels", opts).unwrap();
        zip.add_directory("ppt/slides", opts).unwrap();
        zip.add_directory("ppt/slides/_rels", opts).unwrap();
        zip.add_directory("ppt/slideMasters", opts).unwrap();
        zip.add_directory("ppt/slideLayouts", opts).unwrap();
        zip.add_directory("ppt/theme", opts).unwrap();

        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(content_types.as_bytes()).unwrap();
        zip.start_file("_rels/.rels", opts).unwrap();
        zip.write_all(rels.as_bytes()).unwrap();
        zip.start_file("ppt/presentation.xml", opts).unwrap();
        zip.write_all(pres.as_bytes()).unwrap();
        zip.start_file("ppt/_rels/presentation.xml.rels", opts).unwrap();
        zip.write_all(pres_rels.as_bytes()).unwrap();
        zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
        zip.write_all(slide.as_bytes()).unwrap();
        zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts).unwrap();
        zip.write_all(slide_rels.as_bytes()).unwrap();
        zip.start_file("ppt/slideMasters/slideMaster1.xml", opts).unwrap();
        zip.write_all(master.as_bytes()).unwrap();
        zip.start_file("ppt/slideLayouts/slideLayout1.xml", opts).unwrap();
        zip.write_all(layout.as_bytes()).unwrap();
        zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
        zip.write_all(theme.as_bytes()).unwrap();
        zip.start_file("ppt/presProps.xml", opts).unwrap();
        zip.write_all(pres_props.as_bytes()).unwrap();
        zip.start_file("ppt/tableStyles.xml", opts).unwrap();
        zip.write_all(table_styles.as_bytes()).unwrap();
        zip.start_file("ppt/viewProps.xml", opts).unwrap();
        zip.write_all(view_props.as_bytes()).unwrap();
    }
    buf
}
