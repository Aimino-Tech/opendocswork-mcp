use office_oxide::pptx::shape::{GraphicContent, Shape, TextContent};
use office_oxide::pptx::PptxDocument;
use serde::Serialize;

#[derive(Serialize)] pub struct JsonDocument { pub file: String, pub format: String, pub slide_count: usize, pub slides: Vec<SlideJson> }
#[derive(Serialize)] pub struct SlideJson { pub slide_number: usize, pub name: String, pub notes: Option<String>, pub background_rgb: Option<[u8; 3]>, pub shapes: Vec<ShapeJson> }
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ShapeJson { Text { name: String, text: String, alt_text: Option<String>, placeholder_type: Option<String> }, Image { name: String, alt_text: Option<String>, format: Option<String> }, Table { name: String, rows: Vec<Vec<String>> }, Group { name: String, children: Vec<ShapeJson> }, Connector { name: String }, UnknownGraphic { name: String } }
#[derive(Serialize)] pub struct Chunk { pub index: usize, pub text: String, pub metadata: serde_json::Value }

fn text_body(b: Option<&office_oxide::pptx::shape::TextBody>) -> String {
    let Some(b) = b else { return String::new() };
    let mut out = String::new();
    for (i, p) in b.paragraphs.iter().enumerate() {
        if i > 0 { out.push('\n'); }
        for c in &p.content { match c { TextContent::Run(r) => out.push_str(&r.text), _ => {} } }
    }
    out
}

fn to_json(s: &Shape) -> ShapeJson {
    match s {
        Shape::AutoShape(a) => ShapeJson::Text { name: a.name.clone(), text: text_body(a.text_body.as_ref()), alt_text: a.alt_text.clone(), placeholder_type: a.placeholder.as_ref().and_then(|p| p.ph_type.clone()) },
        Shape::Picture(p) => ShapeJson::Image { name: p.name.clone(), alt_text: p.alt_text.clone(), format: p.format.clone() },
        Shape::GraphicFrame(f) => match &f.content {
            GraphicContent::Table(t) => ShapeJson::Table { name: f.name.clone(), rows: t.rows.iter().map(|r| r.cells.iter().map(|c| text_body(c.text_body.as_ref())).collect()).collect() },
            _ => ShapeJson::UnknownGraphic { name: f.name.clone() },
        },
        Shape::Group(g) => ShapeJson::Group { name: g.name.clone(), children: g.children.iter().map(to_json).collect() },
        _ => ShapeJson::Connector { name: String::new() },
    }
}

pub fn read_ppt_to_json(path: &str) -> Result<JsonDocument, String> {
    let doc = PptxDocument::open(path).map_err(|e| format!("Failed to open PPTX: {e}"))?;
    Ok(JsonDocument { file: path.into(), format: "pptx".into(), slide_count: doc.slides.len(), slides: doc.slides.iter().enumerate().map(|(i, s)| SlideJson {
        slide_number: i + 1, name: s.name.clone(), notes: s.notes.clone(), background_rgb: s.background_rgb, shapes: s.shapes.iter().map(to_json).collect(),
    }).collect() })
}

pub fn read_ppt_to_md(path: &str) -> Result<String, String> {
    let doc = PptxDocument::open(path).map_err(|e| format!("Failed to open PPTX: {e}"))?;
    let mut m = format!("# Presentation\n\n---\n\n");
    for (i, s) in doc.slides.iter().enumerate() {
        m.push_str(&format!("## Slide {}\n\n", i + 1));
        for sh in &s.shapes { if let Shape::AutoShape(a) = sh { let t = text_body(a.text_body.as_ref()); if !t.is_empty() { m.push_str(&t); m.push_str("\n\n"); } } }
        if let Some(n) = &s.notes { m.push_str(&format!("> **Speaker Notes:** {n}\n\n")); }
        m.push_str("---\n\n");
    }
    Ok(m)
}

pub fn read_ppt_to_chunks(path: &str) -> Result<Vec<Chunk>, String> {
    let doc = PptxDocument::open(path).map_err(|e| format!("Failed to open PPTX: {e}"))?;
    Ok(doc.slides.iter().enumerate().map(|(i, s)| {
        let mut t = format!("# {}\n\n", if s.name.is_empty() { format!("Slide {}", i + 1) } else { s.name.clone() });
        for sh in &s.shapes { if let Shape::AutoShape(a) = sh { let tx = text_body(a.text_body.as_ref()); if !tx.is_empty() { t.push_str(&tx); t.push('\n'); } } }
        if let Some(n) = &s.notes { t.push_str(&format!("Speaker Notes: {n}\n")); }
        Chunk { index: i, text: t.trim().into(), metadata: serde_json::json!({"slide_number": i + 1, "has_notes": s.notes.is_some()}) }
    }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use office_oxide::pptx::write::PptxWriter;

    fn fixture() -> Vec<u8> {
        let mut w = PptxWriter::new();
        for i in 0..10 { w.add_slide().set_title(&format!("S{}", i + 1)).add_text(&format!("C{}", i + 1)); }
        let d = std::env::temp_dir(); let p = d.join(format!("f{}.pptx", uuid::Uuid::new_v4()));
        w.save(&p).unwrap(); let data = std::fs::read(&p).unwrap(); let _ = std::fs::remove_file(&p);
        data
    }
    fn with_pptx(f: impl FnOnce(&str)) { let data = fixture(); let d = std::env::temp_dir(); let p = d.join(format!("t{}.pptx", uuid::Uuid::new_v4())); std::fs::write(&p, &data).unwrap(); f(p.to_str().unwrap()); let _ = std::fs::remove_file(&p); }
    #[test] fn json_ok() { with_pptx(|p| { let r = read_ppt_to_json(p).unwrap(); assert_eq!(r.slide_count, 10); }); }
    #[test] fn md_ok() { with_pptx(|p| { let r = read_ppt_to_md(p).unwrap(); assert!(r.contains("## Slide 1")); }); }
    #[test] fn chunks_ok() { with_pptx(|p| { let r = read_ppt_to_chunks(p).unwrap(); assert_eq!(r.len(), 10); }); }
    #[test] fn missing_err() { assert!(read_ppt_to_json("/nope.pptx").is_err()); }
}
