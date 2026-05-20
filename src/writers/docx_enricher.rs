use std::io::{Cursor, Read, Write};
use std::path::Path;

/// Comprehensive styles.xml template (349KB) from python-docx — includes all styles
/// that LibreOffice requires. rdocx's built-in styles.xml is too minimal and produces
/// corrupt documents that LibreOffice rejects.
const STYLES_TEMPLATE: &str = include_str!("styles_template.xml");

pub fn enrich_docx(
    file_path: &str,
    title: &str,
    author: &str,
    include_page_numbers: bool,
    include_header: bool,
) -> Result<(), String> {
    let path = Path::new(file_path);
    let original = std::fs::read(path).map_err(|e| format!("Failed to read DOCX: {}", e))?;

    let reader = Cursor::new(&original);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Failed to open DOCX ZIP: {}", e))?;

    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry {}: {}", i, e))?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .map_err(|e| format!("Failed to read entry '{}': {}", name, e))?;
        entries.push((name, data));
    }
    drop(archive);

    let existing: std::collections::HashSet<String> =
        entries.iter().map(|(n, _)| n.clone()).collect();

    let mut content_type_overrides: Vec<(String, String)> = Vec::new();
    let mut document_rels: Vec<(String, String)> = Vec::new();

    if !existing.contains("word/theme/theme1.xml") {
        entries.push(("word/theme/theme1.xml".to_string(), THEME_XML.as_bytes().to_vec()));
        content_type_overrides.push((
            "/word/theme/theme1.xml".to_string(),
            "application/vnd.openxmlformats-officedocument.theme+xml".to_string(),
        ));
        document_rels.push((
            "theme".to_string(),
            "theme/theme1.xml".to_string(),
        ));
    }

    if !existing.contains("word/numbering.xml") {
        entries.push(("word/numbering.xml".to_string(), NUMBERING_XML.as_bytes().to_vec()));
        content_type_overrides.push((
            "/word/numbering.xml".to_string(),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"
                .to_string(),
        ));
        document_rels.push((
            "numbering".to_string(),
            "numbering.xml".to_string(),
        ));
    }

    if !existing.contains("word/fontTable.xml") {
        entries.push(("word/fontTable.xml".to_string(), FONT_TABLE_XML.as_bytes().to_vec()));
        content_type_overrides.push((
            "/word/fontTable.xml".to_string(),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.fontTable+xml"
                .to_string(),
        ));
        document_rels.push((
            "fontTable".to_string(),
            "fontTable.xml".to_string(),
        ));
    }

    if !existing.contains("word/settings.xml") {
        entries.push(("word/settings.xml".to_string(), SETTINGS_XML.as_bytes().to_vec()));
        content_type_overrides.push((
            "/word/settings.xml".to_string(),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml"
                .to_string(),
        ));
        document_rels.push((
            "settings".to_string(),
            "settings.xml".to_string(),
        ));
    }

    if include_page_numbers {
        if !existing.contains("word/footer1.xml") {
            let footer = format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:ftr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:p>
    <w:pPr><w:jc w:val="center"/></w:pPr>
    <w:r><w:rPr><w:sz w:val="18"/><w:color w:val="808080"/></w:rPr><w:t>Page </w:t></w:r>
    <w:r><w:rPr><w:sz w:val="18"/><w:color w:val="808080"/></w:rPr>
      <w:fldChar w:fldCharType="begin"/>
    </w:r>
    <w:r><w:rPr><w:sz w:val="18"/><w:color w:val="808080"/></w:rPr>
      <w:instrText xml:space="preserve"> PAGE </w:instrText>
    </w:r>
    <w:r><w:rPr><w:sz w:val="18"/><w:color w:val="808080"/></w:rPr>
      <w:fldChar w:fldCharType="separate"/>
    </w:r>
    <w:r><w:rPr><w:sz w:val="18"/><w:color w:val="808080"/></w:rPr>
      <w:t>1</w:t>
    </w:r>
    <w:r><w:rPr><w:sz w:val="18"/><w:color w:val="808080"/></w:rPr>
      <w:fldChar w:fldCharType="end"/>
    </w:r>
  </w:p>
</w:ftr>"#
            );
            entries.push(("word/footer1.xml".to_string(), footer.into_bytes()));
            content_type_overrides.push((
                "/word/footer1.xml".to_string(),
                "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"
                    .to_string(),
            ));
        }
        // Always add footer rel — rdocx may create the file without adding rels
        document_rels.push(("footer".to_string(), "footer1.xml".to_string()));
    }

    if include_header {
        if !existing.contains("word/header1.xml") {
        let header = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:hdr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:p>
    <w:pPr><w:jc w:val="right"/></w:pPr>
    <w:r><w:rPr><w:sz w:val="16"/><w:color w:val="808080"/><w:i/></w:rPr><w:t>{}</w:t></w:r>
  </w:p>
</w:hdr>"#,
            title
        );
        entries.push(("word/header1.xml".to_string(), header.into_bytes()));
        content_type_overrides.push((
            "/word/header1.xml".to_string(),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml"
                .to_string(),
        ));
        }
        // Always add header rel — rdocx may create the file without adding rels
        document_rels.push(("header".to_string(), "header1.xml".to_string()));
    }

    if !existing.contains("docProps/core.xml") {
        let core = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/"
                   xmlns:dcmitype="http://purl.org/dc/dcmitype/"
                   xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:title>{}</dc:title>
  <dc:creator>{}</dc:creator>
  <cp:lastModifiedBy>docworks-mcp</cp:lastModifiedBy>
  <cp:revision>1</cp:revision>
  <dcterms:created xsi:type="dcterms:W3CDTF">{}</dcterms:created>
  <dcterms:modified xsi:type="dcterms:W3CDTF">{}</dcterms:modified>
</cp:coreProperties>"#,
            title,
            author,
            now_iso8601(),
            now_iso8601()
        );
        entries.push(("docProps/core.xml".to_string(), core.into_bytes()));
        content_type_overrides.push((
            "/docProps/core.xml".to_string(),
            "application/vnd.openxmlformats-package.core-properties+xml".to_string(),
        ));
    }

    if !existing.contains("docProps/app.xml") {
        let app = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"
            xmlns:vt="http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes">
  <Application>docworks-mcp</Application>
  <DocSecurity>0</DocSecurity>
  <Lines>1</Lines>
  <Paragraphs>1</Paragraphs>
  <ScaleCrop>false</ScaleCrop>
  <HeadingPairs/>
  <TitlesOfParts/>
  <Company>docworks-mcp</Company>
  <LinksUpToDate>false</LinksUpToDate>
  <CharactersWithSpaces>0</CharactersWithSpaces>
  <SharedDoc>false</SharedDoc>
  <HyperlinksChanged>false</HyperlinksChanged>
  <AppVersion>16.0000</AppVersion>
</Properties>"#
        );
        entries.push(("docProps/app.xml".to_string(), app.into_bytes()));
        content_type_overrides.push((
            "/docProps/app.xml".to_string(),
            "application/vnd.openxmlformats-officedocument.extended-properties+xml".to_string(),
        ));
    }

    // Replace rdocx's minimal styles.xml with a comprehensive template (LibreOffice-compatible)
    if let Some(pos) = entries.iter().position(|(n, _)| n == "word/styles.xml") {
        entries[pos].1 = STYLES_TEMPLATE.as_bytes().to_vec();
    }

    let mut ct_changed = false;
    for (name, _ct) in &content_type_overrides {
        if !existing_content_type(&entries, name) {
            ct_changed = true;
        }
    }

    if ct_changed || !document_rels.is_empty() {
        let ct_xml = rebuild_content_types(&entries, &content_type_overrides);
        replace_entry(&mut entries, "[Content_Types].xml", ct_xml.as_bytes().to_vec());

        let mut rels_changed = false;
        if has_entry(&entries, "docProps/core.xml")
            && !existing_relationship(&entries, "_rels/.rels", "core-properties")
        {
            rels_changed = true;
        }
        if has_entry(&entries, "docProps/app.xml")
            && !existing_relationship(&entries, "_rels/.rels", "extended-properties")
        {
            rels_changed = true;
        }
        if rels_changed {
            let rels_xml = rebuild_rels(&entries);
            replace_entry(&mut entries, "_rels/.rels", rels_xml.as_bytes().to_vec());
        }

        if !document_rels.is_empty() {
            let doc_rels_xml = rebuild_document_rels(&entries, &document_rels);
            replace_entry(
                &mut entries,
                "word/_rels/document.xml.rels",
                doc_rels_xml.as_bytes().to_vec(),
            );
        }
    }

    let cursor = Cursor::new(Vec::new());
    let mut zip_writer = zip::ZipWriter::new(cursor);
    let opts = zip::write::FileOptions::<'_, ()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, data) in &entries {
        if name.ends_with('/') {
            zip_writer
                .add_directory(name, opts)
                .map_err(|e| format!("Failed to add directory '{}': {}", name, e))?;
        } else {
            zip_writer
                .start_file(name, opts)
                .map_err(|e| format!("Failed to start file '{}': {}", name, e))?;
            zip_writer
                .write_all(data)
                .map_err(|e| format!("Failed to write '{}': {}", name, e))?;
        }
    }

    let inner = zip_writer
        .finish()
        .map_err(|e| format!("Failed to finish ZIP: {}", e))?;
    let new_content = inner.into_inner();

    std::fs::write(path, &new_content)
        .map_err(|e| format!("Failed to write enriched DOCX: {}", e))?;

    Ok(())
}

fn has_entry(entries: &[(String, Vec<u8>)], name: &str) -> bool {
    entries.iter().any(|(n, _)| n == name)
}

fn replace_entry(entries: &mut Vec<(String, Vec<u8>)>, name: &str, data: Vec<u8>) {
    if let Some(pos) = entries.iter().position(|(n, _)| n == name) {
        entries[pos].1 = data;
    } else {
        entries.push((name.to_string(), data));
    }
}

fn existing_content_type(entries: &[(String, Vec<u8>)], part_name: &str) -> bool {
    let ct_content = entries
        .iter()
        .find(|(n, _)| n == "[Content_Types].xml")
        .map(|(_, d)| String::from_utf8_lossy(d))
        .unwrap_or_default();
    ct_content.contains(&format!("PartName=\"{}\"", part_name))
}

fn existing_relationship(entries: &[(String, Vec<u8>)], rels_file: &str, rel_type: &str) -> bool {
    let rels_content = entries
        .iter()
        .find(|(n, _)| n == rels_file)
        .map(|(_, d)| String::from_utf8_lossy(d))
        .unwrap_or_default();
    rels_content.contains(rel_type)
}

fn rebuild_content_types(
    entries: &[(String, Vec<u8>)],
    new_overrides: &[(String, String)],
) -> String {
    let existing_ct = entries
        .iter()
        .find(|(n, _)| n == "[Content_Types].xml")
        .map(|(_, d)| String::from_utf8_lossy(d))
        .unwrap_or_default();

    let mut overrides: Vec<String> = Vec::new();

    for line in existing_ct.lines() {
        if line.contains("<Override") {
            if let Some(start) = line.find("PartName=\"") {
                if let Some(end) = line[start + 10..].find('"') {
                    let pn = &line[start + 10..start + 10 + end];
                    if let Some((_, c)) = new_overrides.iter().find(|(n, _)| n == pn) {
                        overrides
                            .push(format!(r#"<Override PartName="{pn}" ContentType="{c}"/>"#));
                    } else {
                        if let Some(ct_start) =
                            line[start + 10 + end + 1..].find("ContentType=\"")
                        {
                            let ct_start_abs = start + 10 + end + 1 + ct_start + 13;
                            if let Some(ct_end) = line[ct_start_abs..].find('"') {
                                let ct_val = &line[ct_start_abs..ct_start_abs + ct_end];
                                overrides.push(format!(
                                    r#"<Override PartName="{pn}" ContentType="{ct_val}"/>"#
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    for (pn, ct) in new_overrides {
        if !overrides.iter().any(|o| o.contains(pn)) {
            overrides.push(format!(r#"<Override PartName="{pn}" ContentType="{ct}"/>"#));
        }
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
{}
</Types>"#,
        overrides.join("\n")
    )
}

fn rebuild_rels(entries: &[(String, Vec<u8>)]) -> String {
    let mut rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );

    rels.push_str(
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>"#,
    );

    if has_entry(entries, "docProps/core.xml") {
        rels.push_str(
            r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>"#,
        );
    }
    if has_entry(entries, "docProps/app.xml") {
        rels.push_str(
            r#"<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>"#,
        );
    }

    rels.push_str("</Relationships>");
    rels
}

fn rebuild_document_rels(
    entries: &[(String, Vec<u8>)],
    new_rels: &[(String, String)],
) -> String {
    let existing = entries
        .iter()
        .find(|(n, _)| n == "word/_rels/document.xml.rels")
        .map(|(_, d)| String::from_utf8_lossy(d))
        .unwrap_or_default();

    let mut rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );

    // rdocx hardcodes rId1 for header and rId2 for footer in the document's sectPr.
    // We must use the same rIds in the relationships for LibreOffice to find them.
    const HEADER_RID: &str = "rId1";
    const FOOTER_RID: &str = "rId2";

    let mut rid_counter: u32 = 3;  // Start at 3 since 1=header, 2=footer
    let mut found_header = false;
    let mut found_footer = false;
    for line in existing.lines() {
        if line.contains("<Relationship") && !line.contains("numbering")
            && !line.contains("fontTable") && !line.contains("theme")
            && !line.contains("settings")
        {
            // Skip existing header/footer rels — we replace them
            if line.contains("header") { found_header = true; continue; }
            if line.contains("footer") { found_footer = true; continue; }

            if let Some(start) = line.find("Id=\"") {
                if let Some(end) = line[start + 4..].find('"') {
                    let id = &line[start + 4..start + 4 + end];
                    let num: u32 = id[3..].parse().unwrap_or(0);
                    if num > rid_counter {
                        rid_counter = num;
                    }
                }
            }
            rels.push_str(&line);
            rels.push('\n');
        }
    }
    rid_counter += 1;

    let rel_type_map: std::collections::HashMap<&str, &str> = [
        ("styles", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles"),
        ("numbering", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering"),
        ("fontTable", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/fontTable"),
        ("settings", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings"),
        ("theme", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme"),
        ("footer", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer"),
        ("header", "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header"),
    ]
    .iter()
    .cloned()
    .collect();

    for (rel_name, target) in new_rels {
        let rel_type = rel_type_map.get(rel_name.as_str()).unwrap_or(&"");
        // Use fixed rIds that match rdocx's sectPr expectations
        let rid = match rel_name.as_str() {
            "header" => HEADER_RID.to_string(),
            "footer" => FOOTER_RID.to_string(),
            _ => format!("rId{}", rid_counter),
        };
        if rel_name != "header" && rel_name != "footer" {
            rid_counter += 1;
        }
        rels.push_str(&format!(
            r#"<Relationship Id="{rid}" Type="{rel_type}" Target="{target}"/>"#,
        ));
    }

    rels.push_str("</Relationships>");
    rels
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();

    let days_since_epoch = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    let mut y = 1970i64;
    let mut remaining = days_since_epoch as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }

    let months_days: [i64; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 0;
    for (i, &md) in months_days.iter().enumerate() {
        if remaining < md {
            m = i + 1;
            break;
        }
        remaining -= md;
    }
    if m == 0 {
        m = 12;
    }
    let d = remaining + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

const THEME_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="1F497D"/></a:dk2>
      <a:lt2><a:srgbClr val="EEECE1"/></a:lt2>
      <a:accent1><a:srgbClr val="4F81BD"/></a:accent1>
      <a:accent2><a:srgbClr val="C0504D"/></a:accent2>
      <a:accent3><a:srgbClr val="9BBB59"/></a:accent3>
      <a:accent4><a:srgbClr val="8064A2"/></a:accent4>
      <a:accent5><a:srgbClr val="4BACC6"/></a:accent5>
      <a:accent6><a:srgbClr val="F79646"/></a:accent6>
      <a:hlink><a:srgbClr val="0000FF"/></a:hlink>
      <a:folHlink><a:srgbClr val="800080"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="Office">
      <a:majorFont>
        <a:latin typeface="Calibri Light"/>
        <a:ea typeface=""/>
        <a:cs typeface=""/>
      </a:majorFont>
      <a:minorFont>
        <a:latin typeface="Calibri"/>
        <a:ea typeface=""/>
        <a:cs typeface=""/>
      </a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="Office">
      <a:fillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:gradFill rotWithShape="1">
          <a:gsLst>
            <a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="50000"/><a:satMod val="300000"/></a:schemeClr></a:gs>
            <a:gs pos="35000"><a:schemeClr val="phClr"><a:tint val="37000"/><a:satMod val="300000"/></a:schemeClr></a:gs>
            <a:gs pos="100000"><a:schemeClr val="phClr"><a:tint val="15000"/><a:satMod val="350000"/></a:schemeClr></a:gs>
          </a:gsLst>
          <a:ln ang="5400000" scaled="0"/>
        </a:gradFill>
        <a:gradFill rotWithShape="1">
          <a:gsLst>
            <a:gs pos="0"><a:schemeClr val="phClr"><a:shade val="51000"/><a:satMod val="130000"/></a:schemeClr></a:gs>
            <a:gs pos="80000"><a:schemeClr val="phClr"><a:shade val="93000"/><a:satMod val="130000"/></a:schemeClr></a:gs>
            <a:gs pos="100000"><a:schemeClr val="phClr"><a:shade val="94000"/><a:satMod val="135000"/></a:schemeClr></a:gs>
          </a:gsLst>
          <a:ln ang="5400000" scaled="0"/>
        </a:gradFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="9525" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"><a:shade val="95000"/><a:satMod val="100000"/></a:schemeClr></a:solidFill><a:prstDash val="solid"/></a:ln>
        <a:ln w="25400" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln>
        <a:ln w="38100" cap="flat" cmpd="sng" algn="ctr"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="20000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="38000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle>
        <a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="23000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="35000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle>
        <a:effectStyle><a:effectLst><a:outerShdw blurRad="40000" dist="23000" dir="5400000" rotWithShape="0"><a:srgbClr val="000000"><a:alpha val="35000"/></a:srgbClr></a:outerShdw></a:effectLst></a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:gradFill rotWithShape="1">
          <a:gsLst>
            <a:gs pos="0"><a:schemeClr val="phClr"><a:tint val="40000"/><a:satMod val="350000"/></a:schemeClr></a:gs>
            <a:gs pos="100000"><a:schemeClr val="phClr"><a:tint val="45000"/><a:satMod val="350000"/></a:schemeClr></a:gs>
          </a:gsLst>
          <a:ln ang="5400000" scaled="0"/>
        </a:gradFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;

const NUMBERING_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
             xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas"
             xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"
             xmlns:v="urn:schemas-microsoft-com:vml"
             xmlns:wp14="http://schemas.microsoft.com/office/word/2010/wordprocessingDrawing"
             xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
             xmlns:w10="urn:schemas-microsoft-com:office:word"
             xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"
             xmlns:wpg="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup"
             xmlns:wpi="http://schemas.microsoft.com/office/word/2010/wordprocessingInk"
             xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml"
             xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape"
             mc:Ignorable="w14 wp14">
  <w:abstractNum w:abstractNumId="0">
    <w:nsid w:val="AAAAAAAA"/>
    <w:multiLevelType w:val="hybridMultilevel"/>
    <w:tmpl w:val="AAAAAAAA"/>
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="bullet"/>
      <w:lvlText w:val="▪"/>
      <w:lvlJc w:val="left"/>
      <w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr>
      <w:rPr><w:rFonts w:ascii="Symbol" w:hAnsi="Symbol" w:hint="default"/><w:sz w:val="24"/></w:rPr>
    </w:lvl>
    <w:lvl w:ilvl="1">
      <w:start w:val="1"/>
      <w:numFmt w:val="bullet"/>
      <w:lvlText w:val="●"/>
      <w:lvlJc w:val="left"/>
      <w:pPr><w:ind w:left="1440" w:hanging="360"/></w:pPr>
      <w:rPr><w:rFonts w:ascii="Courier New" w:hAnsi="Courier New" w:hint="default"/><w:sz w:val="24"/></w:rPr>
    </w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="1">
    <w:nsid w:val="BBBBBBBB"/>
    <w:multiLevelType w:val="hybridMultilevel"/>
    <w:tmpl w:val="BBBBBBBB"/>
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1."/>
      <w:lvlJc w:val="left"/>
      <w:pPr><w:ind w:left="360" w:hanging="360"/></w:pPr>
      <w:rPr><w:sz w:val="24"/></w:rPr>
    </w:lvl>
    <w:lvl w:ilvl="1">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1.%2."/>
      <w:lvlJc w:val="left"/>
      <w:pPr><w:ind w:left="792" w:hanging="432"/></w:pPr>
      <w:rPr><w:sz w:val="24"/></w:rPr>
    </w:lvl>
    <w:lvl w:ilvl="2">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1.%2.%3."/>
      <w:lvlJc w:val="left"/>
      <w:pPr><w:ind w:left="1224" w:hanging="504"/></w:pPr>
      <w:rPr><w:sz w:val="24"/></w:rPr>
    </w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="2">
    <w:nsid w:val="CCCCCCCC"/>
    <w:multiLevelType w:val="hybridMultilevel"/>
    <w:tmpl w:val="CCCCCCCC"/>
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1)"/>
      <w:lvlJc w:val="left"/>
      <w:pPr><w:ind w:left="360" w:hanging="360"/></w:pPr>
      <w:rPr><w:sz w:val="24"/></w:rPr>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
  <w:num w:numId="2">
    <w:abstractNumId w:val="1"/>
  </w:num>
  <w:num w:numId="3">
    <w:abstractNumId w:val="2"/>
  </w:num>
</w:numbering>"#;

const FONT_TABLE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:fonts xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
         xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
         xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
         xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"
         mc:Ignorable="w14">
  <w:font w:name="Calibri">
    <w:panose1 w:val="020F0502020204030204"/>
    <w:charset w:val="00"/>
    <w:family w:val="auto"/>
    <w:pitch w:val="variable"/>
    <w:sig w:usb0="E10002FF" w:usb1="4000ACFF" w:usb2="00000009" w:usb3="00000000" w:csb0="0000019F" w:csb1="00000000"/>
  </w:font>
  <w:font w:name="Calibri Light">
    <w:panose1 w:val="020F0302020204030204"/>
    <w:charset w:val="00"/>
    <w:family w:val="auto"/>
    <w:pitch w:val="variable"/>
    <w:sig w:usb0="A00002EF" w:usb1="4000207B" w:usb2="00000000" w:usb3="00000000" w:csb0="0000019F" w:csb1="00000000"/>
  </w:font>
  <w:font w:name="Times New Roman">
    <w:panose1 w:val="02020603050405020304"/>
    <w:charset w:val="00"/>
    <w:family w:val="auto"/>
    <w:pitch w:val="variable"/>
    <w:sig w:usb0="E0002AFF" w:usb1="C0007841" w:usb2="00000009" w:usb3="00000000" w:csb0="000001FF" w:csb1="00000000"/>
  </w:font>
  <w:font w:name="Arial">
    <w:panose1 w:val="020B0604020202020204"/>
    <w:charset w:val="00"/>
    <w:family w:val="auto"/>
    <w:pitch w:val="variable"/>
    <w:sig w:usb0="E0002AFF" w:usb1="C0007843" w:usb2="00000009" w:usb3="00000000" w:csb0="000001FF" w:csb1="00000000"/>
  </w:font>
  <w:font w:name="Symbol">
    <w:panose1 w:val="00000000000000000000"/>
    <w:charset w:val="02"/>
    <w:family w:val="auto"/>
    <w:pitch w:val="variable"/>
    <w:sig w:usb0="00000000" w:usb1="10000000" w:usb2="00000000" w:usb3="00000000" w:csb0="80000000" w:csb1="00000000"/>
  </w:font>
</w:fonts>"#;

const SETTINGS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:settings xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
            xmlns:o="urn:schemas-microsoft-com:office:office"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"
            xmlns:v="urn:schemas-microsoft-com:vml"
            xmlns:w10="urn:schemas-microsoft-com:office:word"
            xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml"
            xmlns:sl="http://schemas.openxmlformats.org/schemaLibrary/2006/main"
            mc:Ignorable="w14">
  <w:zoom w:percent="100"/>
  <w:defaultTabStop w:val="720"/>
  <w:characterSpacingControl w:val="doNotCompress"/>
  <w:footnotePr>
    <w:footnote w:id="-1"/>
    <w:footnote w:id="0"/>
  </w:footnotePr>
  <w:endnotePr>
    <w:endnote w:id="-1"/>
    <w:endnote w:id="0"/>
  </w:endnotePr>
  <w:compat>
    <w:useFELayout/>
    <w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/>
    <w:compatSetting w:name="overrideTableStyleFontSizeAndJustification" w:uri="http://schemas.microsoft.com/office/word" w:val="1"/>
    <w:compatSetting w:name="enableOpenTypeFeatures" w:uri="http://schemas.microsoft.com/office/word" w:val="1"/>
    <w:compatSetting w:name="doNotFlipMirrorIndents" w:uri="http://schemas.microsoft.com/office/word" w:val="1"/>
  </w:compat>
</w:settings>"#;
