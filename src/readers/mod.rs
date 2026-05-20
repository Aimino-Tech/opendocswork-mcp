#![allow(dead_code)]

pub mod excel;
pub mod powerpoint;
pub mod word;

use crate::formats::DocumentFormat;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub index: usize,
    pub text: String,
    pub metadata: HashMap<String, String>,
}

/// Unified reader trait for all Office document formats.
/// Each format-specific reader implements this to provide
/// JSON, Markdown, and chunked reading.
#[async_trait]
pub trait Reader: Send + Sync {
    async fn read_to_json(&self, path: &str) -> Result<serde_json::Value, anyhow::Error>;
    async fn read_to_markdown(&self, path: &str) -> Result<String, anyhow::Error>;
    async fn read_to_chunks(&self, path: &str) -> Result<Vec<DocumentChunk>, anyhow::Error>;
}

/// Registry mapping DocumentFormat to concrete Reader implementations.
pub struct ReaderRegistry {
    readers: HashMap<DocumentFormat, Box<dyn Reader>>,
}

impl ReaderRegistry {
    pub fn new() -> Self {
        Self {
            readers: HashMap::new(),
        }
    }

    pub fn register(&mut self, format: DocumentFormat, reader: Box<dyn Reader>) {
        self.readers.insert(format, reader);
    }

    pub fn get(&self, format: DocumentFormat) -> Option<&dyn Reader> {
        self.readers.get(&format).map(|b| b.as_ref())
    }
}
