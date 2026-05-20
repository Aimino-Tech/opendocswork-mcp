#![allow(dead_code)]

pub mod docx_enricher;
pub mod excel;
pub mod powerpoint;
pub mod word;

use crate::formats::DocumentFormat;
use async_trait::async_trait;
use std::collections::HashMap;

/// Unified writer trait for all Office document formats.
/// Supports create, write/update, and edit operations.
#[async_trait]
pub trait Writer: Send + Sync {
    async fn create(&self, path: &str, data: serde_json::Value) -> Result<(), anyhow::Error>;
    async fn write(
        &self,
        path: &str,
        data: serde_json::Value,
        target: &Option<String>,
    ) -> Result<(), anyhow::Error>;
    async fn edit(
        &self,
        path: &str,
        operation: &str,
        target: &str,
        value: &serde_json::Value,
    ) -> Result<(), anyhow::Error>;
}

/// Registry mapping DocumentFormat to concrete Writer implementations.
pub struct WriterRegistry {
    writers: HashMap<DocumentFormat, Box<dyn Writer>>,
}

impl WriterRegistry {
    pub fn new() -> Self {
        Self {
            writers: HashMap::new(),
        }
    }

    pub fn register(&mut self, format: DocumentFormat, writer: Box<dyn Writer>) {
        self.writers.insert(format, writer);
    }

    pub fn get(&self, format: DocumentFormat) -> Option<&dyn Writer> {
        self.writers.get(&format).map(|b| b.as_ref())
    }
}
