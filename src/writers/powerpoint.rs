use std::path::{Path, PathBuf};
use office_oxide::format::DocumentFormat;
use office_oxide::pptx::write::PptxWriter;

pub struct SlideOutline {
    pub title: String,
    pub body_text: Vec<String>,
    pub bullets: Vec<String>,
    pub alignment: Option<office_oxide::ir::ParagraphAlignment>,
}

pub fn write_ppt_deck(slides: &[SlideOutline], output_path: impl AsRef<Path>) -> Result<PathBuf, String> {
    let mut writer = PptxWriter::new();
    for s in slides {
        let slide = writer.add_slide();
        if !s.title.is_empty() { slide.set_title_aligned(&s.title, s.alignment.clone()); }
        for t in &s.body_text { slide.add_text(t); }
        if !s.bullets.is_empty() {
            let items: Vec<&str> = s.bullets.iter().map(|x| x.as_str()).collect();
            slide.add_bullet_list(&items);
        }
    }
    writer.save(output_path.as_ref()).map_err(|e| format!("save: {e}"))?;
    Ok(output_path.as_ref().to_path_buf())
}

pub fn write_ppt_from_md(md: &str, output_path: impl AsRef<Path>) -> Result<PathBuf, String> {
    let p = output_path.as_ref();
    office_oxide::create::create_from_markdown(md, DocumentFormat::Pptx, p).map_err(|e| format!("md: {e}"))?;
    Ok(p.to_path_buf())
}
