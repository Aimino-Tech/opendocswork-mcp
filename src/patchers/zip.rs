use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const DOCX_DOC: &str = "word/document.xml";
const SLIDE_PREF: &str = "ppt/slides/slide";
const SHEET_PREF: &str = "xl/worksheets/sheet";

fn validate(fp: &Path, expected: &[&str]) -> Result<(), String> {
    if !fp.exists() {
        return Err(format!("not found: {}", fp.display()));
    }
    let ext = fp
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !expected.contains(&ext.as_str()) {
        return Err(format!("unsupported format '.{ext}'"));
    }
    Ok(())
}

fn read_entries(fp: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let f = std::fs::File::open(fp).map_err(|e| format!("open: {e}"))?;
    let mut arc = ZipArchive::new(f).map_err(|e| format!("zip: {e}"))?;
    let mut entries = Vec::with_capacity(arc.len());
    for i in 0..arc.len() {
        let mut e = arc.by_index(i).map_err(|e| format!("entry {i}: {e}"))?;
        let name = e.name().to_string();
        let mut data = Vec::new();
        e.read_to_end(&mut data)
            .map_err(|e| format!("read {name}: {e}"))?;
        entries.push((name, data));
    }
    Ok(entries)
}

fn write_entries(
    fp: &Path,
    entries: &[(String, Vec<u8>)],
    tmp_ext: &str,
) -> Result<PathBuf, String> {
    let tmp = fp.with_extension(tmp_ext);
    let f = std::fs::File::create(&tmp).map_err(|e| format!("create: {e}"))?;
    let mut zw = ZipWriter::new(f);
    for (name, data) in entries {
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zw.start_file::<&str, _>(name, opts)
            .map_err(|e| format!("start {name}: {e}"))?;
        zw.write_all(data)
            .map_err(|e| format!("write {name}: {e}"))?;
    }
    zw.finish().map_err(|e| format!("finish: {e}"))?;
    std::fs::rename(&tmp, fp).map_err(|e| format!("rename: {e}"))?;
    Ok(fp.to_path_buf())
}

fn is_content(name: &str, ext: &str) -> bool {
    match ext {
        "docx" => name == DOCX_DOC,
        "pptx" => name.starts_with(SLIDE_PREF) && name.ends_with(".xml"),
        _ => false,
    }
}

fn is_sheet(name: &str, si: usize) -> bool {
    name.starts_with(SHEET_PREF)
        && name.ends_with(".xml")
        && name[SHEET_PREF.len()..]
            .trim_end_matches(".xml")
            .trim_start_matches('0')
            == (si + 1).to_string()
}

fn replace_text_in_xml(
    data: &[u8],
    find: &[String],
    replace: &[String],
    text_tag: &[u8],
) -> Vec<u8> {
    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);
    let mut writer = Writer::new(Vec::new());
    let mut buf = Vec::new();
    let mut in_text = false;
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == text_tag {
                    in_text = true;
                }
                writer.write_event(Event::Start(e)).ok();
            }
            Ok(Event::Empty(e)) => {
                writer.write_event(Event::Empty(e)).ok();
            }
            Ok(Event::Text(e)) => {
                if in_text {
                    let t = std::str::from_utf8(e.as_ref()).unwrap_or_default();
                    let mut nt = t.to_string();
                    for (f, r) in find.iter().zip(replace.iter()) {
                        nt = nt.replace(f.as_str(), r.as_str());
                    }
                    writer.write_event(Event::Text(BytesText::new(&nt))).ok();
                } else {
                    writer.write_event(Event::Text(e)).ok();
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == text_tag {
                    in_text = false;
                }
                writer.write_event(Event::End(e)).ok();
            }
            Ok(Event::Eof) => break,
            Ok(e) => {
                writer.write_event(e).ok();
            }
            Err(_) => return data.to_vec(),
        }
    }
    writer.into_inner()
}

fn set_cell_in_sheet(data: &[u8], cr: &str, val: &str) -> Vec<u8> {
    let s = std::str::from_utf8(data).unwrap_or_default();
    let cell_attr = format!("r=\"{cr}\"");
    let is_num = val.parse::<f64>().is_ok();
    let is_bool = val == "true" || val == "false";

    if !s.contains(&cell_attr) {
        let cell = if is_num || is_bool {
            format!("<c r=\"{cr}\"><v>{val}</v></c>")
        } else {
            format!("<c r=\"{cr}\" t=\"inlineStr\"><is><t>{val}</t></is></c>")
        };
        return if let Some(pos) = s.rfind("</row>") {
            let mut r = s[..pos].to_string();
            r.push_str(&cell);
            r.push_str("</row>");
            r.push_str(&s[pos + 7..]);
            r.into_bytes()
        } else {
            data.to_vec()
        };
    }

    if let Some(cs) = s.find(&cell_attr) {
        let tag_start = s[..cs].rfind('<').unwrap_or(0);
        if s[tag_start..].starts_with("<c") && s[cs..].contains("/>") {
            let end = tag_start + s[tag_start..].find("/>").unwrap() + 2;
            let cell = if is_num || is_bool {
                format!("<c r=\"{cr}\"><v>{val}</v></c>")
            } else {
                format!("<c r=\"{cr}\" t=\"inlineStr\"><is><t>{val}</t></is></c>")
            };
            let mut r = s[..tag_start].to_string();
            r.push_str(&cell);
            r.push_str(&s[end..]);
            return r.into_bytes();
        }
        if let Some(ce) = s[cs..].find("</c>") {
            let end = cs + ce + 4;
            let tag_end = tag_start + s[tag_start..].find('>').unwrap_or(0) + 1;
            let content = if is_num || is_bool {
                format!("<v>{val}</v>")
            } else {
                format!("<is><t>{val}</t></is>")
            };
            let mut r = s[..tag_end].to_string();
            r.push_str(&content);
            r.push_str("</c>");
            r.push_str(&s[end..]);
            return r.into_bytes();
        }
    }
    data.to_vec()
}

fn style_rpr(xml: &[u8], ch: &StyleChanges) -> Vec<u8> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut writer = Writer::new(Vec::new());
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let n = e.name().as_ref().to_vec();
                let name = std::str::from_utf8(&n).unwrap_or("rPr");
                let mut out = BytesStart::new(name);
                for attr in e.attributes().filter_map(|a| a.ok()) {
                    let k = attr.key.as_ref();
                    let skip = (ch.bold.is_some() && k == b"b")
                        || (ch.italic.is_some() && k == b"i")
                        || (ch.underline.is_some() && k == b"u")
                        || (ch.strikethrough.is_some() && k == b"strike")
                        || (ch.font_size_half_pt.is_some() && k == b"sz");
                    if !skip {
                        let k = std::str::from_utf8(k).unwrap_or("");
                        let v = std::str::from_utf8(&attr.value).unwrap_or_default();
                        out.push_attribute((k, v));
                    }
                }
                if ch.bold == Some(true) {
                    out.push_attribute(("b", "1"));
                }
                if ch.italic == Some(true) {
                    out.push_attribute(("i", "1"));
                }
                if ch.underline == Some(true) {
                    out.push_attribute(("u", "sng"));
                }
                if ch.strikethrough == Some(true) {
                    out.push_attribute(("strike", "sng"));
                }
                if let Some(ref sz) = ch.font_size_half_pt {
                    let s = sz.to_string();
                    out.push_attribute(("sz", s.as_str()));
                }
                writer.write_event(Event::Start(out)).ok();
            }
            Ok(Event::Empty(e)) => {
                let n = e.name().as_ref().to_vec();
                let name = std::str::from_utf8(&n).unwrap_or("rPr");
                let mut out = BytesStart::new(name);
                for attr in e.attributes().filter_map(|a| a.ok()) {
                    let k = attr.key.as_ref();
                    let skip = (ch.bold.is_some() && k == b"b")
                        || (ch.italic.is_some() && k == b"i")
                        || (ch.underline.is_some() && k == b"u")
                        || (ch.strikethrough.is_some() && k == b"strike")
                        || (ch.font_size_half_pt.is_some() && k == b"sz");
                    if !skip {
                        let k = std::str::from_utf8(k).unwrap_or("");
                        let v = std::str::from_utf8(&attr.value).unwrap_or_default();
                        out.push_attribute((k, v));
                    }
                }
                if ch.bold == Some(true) {
                    out.push_attribute(("b", "1"));
                }
                if ch.italic == Some(true) {
                    out.push_attribute(("i", "1"));
                }
                if ch.underline == Some(true) {
                    out.push_attribute(("u", "sng"));
                }
                if ch.strikethrough == Some(true) {
                    out.push_attribute(("strike", "sng"));
                }
                if let Some(ref sz) = ch.font_size_half_pt {
                    let s = sz.to_string();
                    out.push_attribute(("sz", s.as_str()));
                }
                writer.write_event(Event::Empty(out)).ok();
            }
            Ok(Event::End(e)) => {
                writer.write_event(Event::End(e)).ok();
            }
            Ok(Event::Text(e)) => {
                writer.write_event(Event::Text(e)).ok();
            }
            Ok(Event::Eof) => break,
            Ok(e) => {
                writer.write_event(e).ok();
            }
            Err(_) => return xml.to_vec(),
        }
    }
    writer.into_inner()
}

fn apply_style_to_xml(data: &[u8], ext: &str, pattern: &str, ch: &StyleChanges) -> Vec<u8> {
    let (run_tag, text_tag, rpr_tag) = match ext {
        "docx" => (b"w:r" as &[u8], b"w:t" as &[u8], b"w:rPr" as &[u8]),
        _ => (b"a:r" as &[u8], b"a:t" as &[u8], b"a:rPr" as &[u8]),
    };
    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut all_events: Vec<Event> = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(e) => all_events.push(e.into_owned()),
            Err(_) => return data.to_vec(),
        }
    }
    let mut writer = Writer::new(Vec::new());
    let mut i = 0;
    while i < all_events.len() {
        let is_run_start = match &all_events[i] {
            Event::Start(s) => s.name().as_ref() == run_tag,
            _ => false,
        };
        if is_run_start {
            let mut depth = 0u32;
            let mut run_events = vec![all_events[i].clone()];
            i += 1;
            while i < all_events.len() {
                let is_start =
                    matches!(&all_events[i], Event::Start(s) if s.name().as_ref() == run_tag);
                let is_end =
                    matches!(&all_events[i], Event::End(s) if s.name().as_ref() == run_tag);
                if is_start {
                    depth += 1;
                }
                run_events.push(all_events[i].clone());
                if is_end && depth == 0 {
                    i += 1;
                    break;
                }
                if is_end {
                    depth -= 1;
                }
                i += 1;
            }
            let mut run_text = String::new();
            let mut in_t = false;
            for ev in &run_events {
                match ev {
                    Event::Start(e) if e.name().as_ref() == text_tag => in_t = true,
                    Event::End(e) if e.name().as_ref() == text_tag => in_t = false,
                    Event::Text(e) if in_t => {
                        let t = std::str::from_utf8(e.as_ref()).unwrap_or_default();
                        run_text.push_str(t);
                    }
                    _ => {}
                }
            }
            let matched = pattern.is_empty() || run_text.contains(pattern);
            if matched && !run_text.is_empty() {
                let tmp_buf;
                {
                    let mut has_rpr = false;
                    for ev in &run_events {
                        if matches!(ev, Event::Start(s) if s.name().as_ref() == rpr_tag)
                            || matches!(ev, Event::Empty(s) if s.name().as_ref() == rpr_tag)
                        {
                            has_rpr = true;
                        }
                    }
                    let mut w2 = Writer::new(Vec::new());
                    let mut rpr_inserted = false;
                    for ev in &run_events {
                        match ev {
                            Event::Start(e) if e.name().as_ref() == rpr_tag => {
                                let rpr_xml = {
                                    let mut w3 = Writer::new(Vec::new());
                                    w3.write_event(Event::Start(e.borrow())).ok();
                                    w3.write_event(Event::End(e.to_end())).ok();
                                    w3.into_inner()
                                };
                                let modified = style_rpr(&rpr_xml, ch);
                                let mut r2 = Reader::from_reader(modified.as_slice());
                                r2.config_mut().trim_text(true);
                                let mut b2 = Vec::new();
                                loop {
                                    b2.clear();
                                    match r2.read_event_into(&mut b2) {
                                        Ok(Event::Eof) => break,
                                        Ok(ev2) => {
                                            w2.write_event(ev2).ok();
                                        }
                                        Err(_) => break,
                                    }
                                }
                            }
                            Event::Empty(e) if e.name().as_ref() == rpr_tag => {
                                let modified = style_rpr(
                                    &{
                                        let mut w3 = Writer::new(Vec::new());
                                        w3.write_event(Event::Empty(e.borrow())).ok();
                                        w3.into_inner()
                                    },
                                    ch,
                                );
                                let mut r2 = Reader::from_reader(modified.as_slice());
                                r2.config_mut().trim_text(true);
                                let mut b2 = Vec::new();
                                loop {
                                    b2.clear();
                                    match r2.read_event_into(&mut b2) {
                                        Ok(Event::Eof) => break,
                                        Ok(ev2) => {
                                            w2.write_event(ev2).ok();
                                        }
                                        Err(_) => break,
                                    }
                                }
                            }
                            _ => {
                                w2.write_event(ev.clone()).ok();
                                if !has_rpr
                                    && !rpr_inserted
                                    && matches!(ev, Event::Start(s) if s.name().as_ref() == run_tag)
                                {
                                    let name = std::str::from_utf8(rpr_tag).unwrap_or("rPr");
                                    let mut rpr = BytesStart::new(name);
                                    if ch.bold == Some(true) {
                                        rpr.push_attribute(("b", "1"));
                                    }
                                    if ch.italic == Some(true) {
                                        rpr.push_attribute(("i", "1"));
                                    }
                                    if ch.underline == Some(true) {
                                        rpr.push_attribute(("u", "sng"));
                                    }
                                    if ch.strikethrough == Some(true) {
                                        rpr.push_attribute(("strike", "sng"));
                                    }
                                    if let Some(ref sz) = ch.font_size_half_pt {
                                        let s = sz.to_string();
                                        rpr.push_attribute(("sz", s.as_str()));
                                    }
                                    if let Some(ref n) = ch.font_name {
                                        rpr.push_attribute(("ascii", n.as_str()));
                                        rpr.push_attribute(("hAnsi", n.as_str()));
                                    }
                                    w2.write_event(Event::Empty(rpr)).ok();
                                    rpr_inserted = true;
                                }
                            }
                        }
                    }
                    tmp_buf = w2.into_inner();
                }
                let mut r2 = Reader::from_reader(tmp_buf.as_slice());
                r2.config_mut().trim_text(true);
                let mut b2 = Vec::new();
                loop {
                    b2.clear();
                    match r2.read_event_into(&mut b2) {
                        Ok(Event::Eof) => break,
                        Ok(ev2) => {
                            writer.write_event(ev2).ok();
                        }
                        Err(_) => break,
                    }
                }
            } else {
                for ev in &run_events {
                    writer.write_event(ev.clone()).ok();
                }
            }
        } else {
            writer.write_event(all_events[i].clone()).ok();
            i += 1;
        }
    }
    writer.into_inner()
}

pub fn surgical_replace_text(
    fp: impl AsRef<Path>,
    find: &[String],
    replace: &[String],
) -> Result<PathBuf, String> {
    let p = fp.as_ref();
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    validate(p, &["docx", "pptx"])?;
    let tag = if ext == "docx" { b"w:t" } else { b"a:t" };
    let mut entries = read_entries(p)?;
    for (name, data) in &mut entries {
        if is_content(name, &ext) {
            *data = replace_text_in_xml(data, find, replace, tag);
        }
    }
    write_entries(p, &entries, &format!("{ext}.tmp"))
}

pub fn surgical_set_cell(
    fp: impl AsRef<Path>,
    si: usize,
    cr: &str,
    val: &str,
) -> Result<PathBuf, String> {
    let p = fp.as_ref();
    validate(p, &["xlsx"])?;
    let mut entries = read_entries(p)?;
    let mut found = false;
    for (name, data) in &mut entries {
        if is_sheet(name, si) {
            found = true;
            *data = set_cell_in_sheet(data, cr, val);
        }
    }
    if !found {
        return Err(format!("sheet {si} not found in {}", p.display()));
    }
    write_entries(p, &entries, "xlsx.tmp")
}

#[derive(Debug, Clone, Default)]
pub struct StyleChanges {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub font_size_half_pt: Option<u16>,
    pub font_name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PatchOp {
    ReplaceText {
        find: Vec<String>,
        replace: Vec<String>,
    },
    SetCell {
        sheet_index: usize,
        cell_ref: String,
        value: String,
    },
    SetStyle {
        text_pattern: String,
        changes: StyleChanges,
    },
}

pub fn surgical_patch(fp: impl AsRef<Path>, ops: &[PatchOp]) -> Result<PathBuf, String> {
    let p = fp.as_ref();
    if !p.exists() {
        return Err(format!("not found: {}", p.display()));
    }
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mut entries = read_entries(p)?;
    for op in ops {
        match op {
            PatchOp::ReplaceText { find, replace } => {
                let tag = if ext == "docx" { b"w:t" } else { b"a:t" };
                for (name, data) in &mut entries {
                    if is_content(name, &ext) {
                        *data = replace_text_in_xml(data, find, replace, tag);
                    }
                }
            }
            PatchOp::SetCell {
                sheet_index,
                cell_ref,
                value,
            } => {
                let mut found = false;
                for (name, data) in &mut entries {
                    if is_sheet(name, *sheet_index) {
                        found = true;
                        *data = set_cell_in_sheet(data, cell_ref, value);
                    }
                }
                if !found {
                    return Err(format!(
                        "sheet {} not found in {}",
                        sheet_index,
                        p.display()
                    ));
                }
            }
            PatchOp::SetStyle {
                text_pattern,
                changes,
            } => {
                for (name, data) in &mut entries {
                    if is_content(name, &ext) {
                        *data = apply_style_to_xml(data, &ext, text_pattern, changes);
                    }
                }
            }
        }
    }
    write_entries(p, &entries, &format!("{ext}.tmp"))
}

pub fn surgical_set_style(
    fp: impl AsRef<Path>,
    pat: &str,
    ch: &StyleChanges,
) -> Result<PathBuf, String> {
    let p = fp.as_ref();
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    validate(p, &["docx", "pptx"])?;
    let mut entries = read_entries(p)?;
    for (name, data) in &mut entries {
        if is_content(name, &ext) {
            *data = apply_style_to_xml(data, &ext, pat, ch);
        }
    }
    write_entries(p, &entries, &format!("{ext}.tmp"))
}
