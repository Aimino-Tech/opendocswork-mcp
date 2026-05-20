use std::io::{Read, Seek};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub passed: bool,
    pub checks: Vec<IntegrityCheck>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheck {
    pub name: String,
    pub passed: bool,
    pub details: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
}

pub struct IntegrityValidator;

impl IntegrityValidator {
    pub fn verify(path: &str) -> Result<IntegrityReport, anyhow::Error> {
        let start = Instant::now();
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => return Err(anyhow::anyhow!("Cannot open '{}': {}", path, e)),
        };

        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                return Ok(IntegrityReport {
                    passed: false,
                    checks: vec![IntegrityCheck {
                        name: "zip_structure".into(),
                        passed: false,
                        details: format!("Not a valid ZIP archive: {}", e),
                        errors: Some(vec![e.to_string()]),
                    }],
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        };

        let checks = vec![
            Self::check_zip_structure(&mut archive)?,
            Self::check_content_types(&mut archive)?,
            Self::check_relationships(&mut archive)?,
            Self::check_xml_well_formed(&mut archive)?,
        ];

        let duration_ms = start.elapsed().as_millis() as u64;
        let passed = checks.iter().all(|c| c.passed);

        Ok(IntegrityReport { passed, checks, duration_ms })
    }

    fn check_zip_structure(archive: &mut ZipArchive<impl Read + Seek>) -> Result<IntegrityCheck, anyhow::Error> {
        let count = archive.len();
        let mut errors = Vec::new();

        for i in 0..count {
            let entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if entry.is_dir() {
                continue;
            }
            let declared = entry.size();
            let compressed = entry.compressed_size();
            if declared > 0 && compressed == 0 && !name.ends_with('/') {
                errors.push(format!("'{}' is empty but declares {} bytes", name, declared));
            }
            if name.starts_with('/') || name.contains("..") || name.starts_with('\\') {
                errors.push(format!("'{}' has unsafe path", name));
            }
        }

        let passed = errors.is_empty();
        Ok(IntegrityCheck {
            name: "zip_structure".into(),
            passed,
            details: format!("{} entries, {} issues", count, errors.len()),
            errors: if errors.is_empty() { None } else { Some(errors) },
        })
    }

    fn check_content_types(archive: &mut ZipArchive<impl Read + Seek>) -> Result<IntegrityCheck, anyhow::Error> {
        let has_ct = archive.by_name("[Content_Types].xml").is_ok();
        if !has_ct {
            return Ok(IntegrityCheck {
                name: "content_types".into(),
                passed: false,
                details: "Missing [Content_Types].xml — not a valid OOXML document".into(),
                errors: Some(vec!["[Content_Types].xml not found".into()]),
            });
        }

        let mut errors = Vec::new();
        let entry_names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
            .collect();

        let mut ct_content = String::new();
        archive.by_name("[Content_Types].xml")?.read_to_string(&mut ct_content)?;

        if !ct_content.contains("<Types") || !ct_content.contains("</Types>") {
            errors.push("[Content_Types].xml has invalid structure".into());
        }

        let mut covered_count = 0;
        for name in &entry_names {
            if name == "[Content_Types].xml" || name.ends_with('/') {
                continue;
            }
            if ct_content.contains(name) || name.starts_with("_rels/") || name == "docProps/app.xml" || name == "docProps/core.xml" {
                covered_count += 1;
            }
        }

        let passed = errors.is_empty();
        Ok(IntegrityCheck {
            name: "content_types".into(),
            passed,
            details: format!("{} entries, {} covered, {} issues", entry_names.len(), covered_count, errors.len()),
            errors: if errors.is_empty() { None } else { Some(errors) },
        })
    }

    fn resolve_rel_target(rel_file: &str, target: &str) -> String {
        if target.starts_with('/') {
            return target.trim_start_matches('/').to_string();
        }
        let dir = match rel_file.rfind('/') {
            Some(i) => &rel_file[..i],
            None => return target.to_string(),
        };
        let base = if dir.ends_with("_rels") {
            let trimmed = dir.trim_end_matches("_rels").trim_end_matches('/');
            if trimmed.is_empty() { "" } else { trimmed }
        } else {
            dir
        };
        if base.is_empty() {
            return target.to_string();
        }
        let mut parts: Vec<&str> = base.split('/').collect();
        for segment in target.split('/') {
            match segment {
                ".." => { parts.pop(); }
                "." | "" => {}
                seg => { parts.push(seg); }
            }
        }
        parts.join("/")
    }

    fn check_relationships(archive: &mut ZipArchive<impl Read + Seek>) -> Result<IntegrityCheck, anyhow::Error> {
        let entry_names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
            .collect();

        let rel_files: Vec<&String> = entry_names.iter().filter(|n| n.ends_with(".rels")).collect();
        let rel_count = rel_files.len();

        let mut errors = Vec::new();
        for rel_file in &rel_files {
            let mut content = String::new();
            let mut entry = match archive.by_name(rel_file) {
                Ok(e) => e,
                Err(_) => { errors.push(format!("Cannot read '{}'", rel_file)); continue; }
            };
            if entry.read_to_string(&mut content).is_err() {
                errors.push(format!("Cannot read '{}'", rel_file));
                continue;
            }
            for line in content.lines() {
                if let Some(rest) = line.trim().split("Target=\"").nth(1) {
                    if let Some(target) = rest.split('"').next() {
                        if target.starts_with("http://") || target.starts_with("https://") || target.starts_with("ftp://") {
                            continue;
                        }
                        let resolved = Self::resolve_rel_target(rel_file, target);
                        if !resolved.is_empty() && !entry_names.contains(&resolved) {
                            errors.push(format!("'{}' references '{}' — resolved to '{}' not found in archive", rel_file, target, resolved));
                        }
                    }
                }
            }
        }

        let passed = errors.is_empty();
        Ok(IntegrityCheck {
            name: "relationships".into(),
            passed,
            details: format!("{} relationship files, {} issues", rel_count, errors.len()),
            errors: if errors.is_empty() { None } else { Some(errors) },
        })
    }

    fn check_xml_well_formed(archive: &mut ZipArchive<impl Read + Seek>) -> Result<IntegrityCheck, anyhow::Error> {
        let mut errors = Vec::new();
        let mut xml_count = 0;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if !name.ends_with(".xml") && !name.ends_with(".rels") {
                continue;
            }
            xml_count += 1;
            let mut content = String::new();
            if entry.read_to_string(&mut content).is_err() {
                errors.push(format!("'{}': cannot read content", name));
                continue;
            }
            if content.trim().is_empty() {
                errors.push(format!("'{}': empty XML file", name));
                continue;
            }
            let mut reader = quick_xml::Reader::from_str(&content);
            let mut parse_errors = Vec::new();
            loop {
                match reader.read_event() {
                    Ok(quick_xml::events::Event::Eof) => break,
                    Err(e) => {
                        parse_errors.push(format!("pos {}: {}", reader.buffer_position(), e));
                        break;
                    }
                    Ok(_) => {}
                }
            }
            if !parse_errors.is_empty() {
                errors.push(format!("'{}': {} parse error(s) — {}", name, parse_errors.len(), parse_errors.join("; ")));
            }
        }

        let passed = errors.is_empty();
        Ok(IntegrityCheck {
            name: "xml_well_formed".into(),
            passed,
            details: format!("{} XML files checked, {} issues", xml_count, errors.len()),
            errors: if errors.is_empty() { None } else { Some(errors) },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use zip::write::FileOptions;

    fn create_test_xlsx(path: &Path) {
        use zip::CompressionMethod;
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: FileOptions<'_, ()> = FileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.add_directory("_rels/", options).unwrap();
        zip.add_directory("docProps/", options).unwrap();
        zip.add_directory("xl/", options).unwrap();
        zip.add_directory("xl/_rels/", options).unwrap();
        zip.add_directory("xl/worksheets/", options).unwrap();
        zip.add_directory("xl/theme/", options).unwrap();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
  <Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
  <Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
  <Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>
</Types>"#).unwrap();

        zip.start_file("_rels/.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/>
</Relationships>"#).unwrap();

        zip.start_file("xl/workbook.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#).unwrap();

        zip.start_file("xl/_rels/workbook.xml.rels", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>"#).unwrap();

        zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row></sheetData>
</worksheet>"#).unwrap();

        zip.start_file("xl/styles.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>
"#).unwrap();

        zip.start_file("xl/sharedStrings.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>Hello</t></si></sst>"#).unwrap();

        zip.start_file("xl/theme/theme1.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:themeElements/></a:theme>"#).unwrap();

        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"/>
"#).unwrap();

        zip.start_file("docProps/app.xml", options).unwrap();
        zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">
  <Application>Microsoft Excel</Application>
</Properties>"#).unwrap();

        zip.finish().unwrap();
    }

    fn create_corrupted_xlsx(path: &Path) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options: FileOptions<'_, ()> = FileOptions::default();

        zip.start_file("not_a_valid_part.xml", options).unwrap();
        zip.write_all(b"this is not valid xml<<broken>>").unwrap();

        zip.finish().unwrap();
    }

    #[test]
    fn test_valid_xlsx_passes_all_checks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.xlsx");
        create_test_xlsx(&path);
        let report = IntegrityValidator::verify(path.to_str().unwrap()).unwrap();
        assert!(report.passed, "Valid xlsx should pass all checks: {:?}", report.checks);
        assert_eq!(report.checks.len(), 4);
        for check in &report.checks {
            assert!(check.passed, "Check '{}' should pass: {}", check.name, check.details);
        }
    }

    #[test]
    fn test_corrupted_file_fails_checks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corrupted.xlsx");
        create_corrupted_xlsx(&path);
        let report = IntegrityValidator::verify(path.to_str().unwrap()).unwrap();
        assert!(!report.passed, "Corrupted file should fail integrity");
        let failed_checks: Vec<&str> = report.checks.iter()
            .filter(|c| !c.passed)
            .map(|c| c.name.as_str())
            .collect();
        assert!(failed_checks.contains(&"content_types"), "Should fail content_types check, got: {:?}", failed_checks);
    }

    #[test]
    fn test_missing_file_returns_error() {
        let result = IntegrityValidator::verify("/tmp/nonexistent_file_12345.xlsx");
        assert!(result.is_err(), "Missing file should return error");
    }
}
