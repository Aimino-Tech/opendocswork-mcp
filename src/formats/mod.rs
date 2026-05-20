pub mod detection;
pub mod registry;

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocumentFormat {
    Docx,
    Doc,
    Xlsx,
    Xls,
    Pptx,
    Ppt,
    Unknown,
}

impl DocumentFormat {
    pub fn from_extension(path: &str) -> Self {
        match Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "docx" => Self::Docx,
            "doc" => Self::Doc,
            "xlsx" => Self::Xlsx,
            "xls" => Self::Xls,
            "pptx" => Self::Pptx,
            "ppt" => Self::Ppt,
            _ => Self::Unknown,
        }
    }

    pub fn from_magic_bytes(path: &str) -> Result<Self, anyhow::Error> {
        use std::io::Read;
        let mut buf = [0u8; 8];
        let mut f = std::fs::File::open(path)?;
        f.read_exact(&mut buf)?;
        match &buf[..4] {
            [0x50, 0x4B, 0x03, 0x04] | [0xD0, 0xCF, 0x11, 0xE0] => Ok(Self::from_extension(path)),
            _ => Ok(Self::Unknown),
        }
    }

    pub fn detect(path: &str) -> Result<Self, anyhow::Error> {
        let ext = Self::from_extension(path);
        if ext != Self::Unknown {
            return Ok(ext);
        }
        if !Path::new(path).exists() {
            return Err(anyhow::anyhow!("File not found: {}", path));
        }
        Self::from_magic_bytes(path)
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Doc => "application/msword",
            Self::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::Xls => "application/vnd.ms-excel",
            Self::Pptx => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Self::Ppt => "application/vnd.ms-powerpoint",
            Self::Unknown => "application/octet-stream",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Docx => "Word Document",
            Self::Doc => "Word 97-2003 Document",
            Self::Xlsx => "Excel Workbook",
            Self::Xls => "Excel 97-2003 Workbook",
            Self::Pptx => "PowerPoint Presentation",
            Self::Ppt => "PowerPoint 97-2003 Presentation",
            Self::Unknown => "Unknown Format",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Docx => "docx",
            Self::Doc => "doc",
            Self::Xlsx => "xlsx",
            Self::Xls => "xls",
            Self::Pptx => "pptx",
            Self::Ppt => "ppt",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_zip_based(&self) -> bool {
        matches!(self, Self::Docx | Self::Xlsx | Self::Pptx)
    }
    pub fn is_writable(&self) -> bool {
        matches!(self, Self::Docx | Self::Xlsx | Self::Pptx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_from_extension() {
        assert_eq!(
            DocumentFormat::from_extension("f.docx"),
            DocumentFormat::Docx
        );
        assert_eq!(
            DocumentFormat::from_extension("f.xlsx"),
            DocumentFormat::Xlsx
        );
        assert_eq!(
            DocumentFormat::from_extension("f.pptx"),
            DocumentFormat::Pptx
        );
        assert_eq!(DocumentFormat::from_extension("f.doc"), DocumentFormat::Doc);
        assert_eq!(DocumentFormat::from_extension("f.xls"), DocumentFormat::Xls);
        assert_eq!(DocumentFormat::from_extension("f.ppt"), DocumentFormat::Ppt);
        assert_eq!(
            DocumentFormat::from_extension("f.unknown"),
            DocumentFormat::Unknown
        );
    }
    #[test]
    fn test_case_insensitivity() {
        assert_eq!(
            DocumentFormat::from_extension("f.DOCX"),
            DocumentFormat::Docx
        );
        assert_eq!(
            DocumentFormat::from_extension("f.XLSX"),
            DocumentFormat::Xlsx
        );
    }
    #[test]
    fn test_mime_types() {
        assert!(DocumentFormat::Docx.mime_type().contains("word"));
        assert!(DocumentFormat::Xlsx.mime_type().contains("sheet"));
        assert!(DocumentFormat::Pptx.mime_type().contains("presentation"));
    }
    #[test]
    fn test_is_zip_based() {
        assert!(DocumentFormat::Docx.is_zip_based());
        assert!(!DocumentFormat::Doc.is_zip_based());
        assert!(!DocumentFormat::Unknown.is_zip_based());
    }
}
