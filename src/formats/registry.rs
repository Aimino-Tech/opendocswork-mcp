use super::DocumentFormat;
use crate::readers::{Reader, ReaderRegistry};
use crate::writers::{Writer, WriterRegistry};

pub struct FormatRegistry {
    reader_registry: ReaderRegistry,
    writer_registry: WriterRegistry,
}

impl FormatRegistry {
    pub fn new() -> Self {
        Self {
            reader_registry: ReaderRegistry::new(),
            writer_registry: WriterRegistry::new(),
        }
    }
    pub fn get_reader(&self, format: DocumentFormat) -> Option<&dyn Reader> {
        self.reader_registry.get(format)
    }
    pub fn get_writer(&self, format: DocumentFormat) -> Option<&dyn Writer> {
        self.writer_registry.get(format)
    }
}
