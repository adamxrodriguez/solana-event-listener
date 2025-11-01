//! JSONL file storage for events

use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use tracing::{error, trace};

/// JSONL file writer for append-only event storage
pub struct JsonlWriter {
    /// File path for writing events
    file_path: String,
}

impl JsonlWriter {
    /// Create a new JSONL writer
    pub fn new(file_path: impl AsRef<str>) -> Self {
        Self {
            file_path: file_path.as_ref().to_string(),
        }
    }

    /// Write an event to the JSONL file (appends to file)
    pub async fn write<T: Serialize>(&self, event: &T) -> Result<()> {
        // Serialize event to JSON
        let json = serde_json::to_string(event)
            .context("Failed to serialize event to JSON")?;

        // Append to file (create if it doesn't exist)
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .with_context(|| format!("Failed to open file for writing: {}", self.file_path))?;

        // Write JSON line with newline
        let mut writer = file;
        writer
            .write_all(json.as_bytes())
            .with_context(|| format!("Failed to write to file: {}", self.file_path))?;
        writer.write_all(b"\n")?;
        
        // Flush to ensure data is written
        writer.flush()?;

        trace!("Wrote event to {}: {} bytes", self.file_path, json.len());

        Ok(())
    }

    /// Get the file path
    pub fn file_path(&self) -> &str {
        &self.file_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use tempfile::NamedTempFile;
    use std::fs;
    use std::io::{BufRead, BufReader};

    #[derive(Serialize)]
    struct TestEvent {
        id: u64,
        message: String,
    }

    #[tokio::test]
    async fn test_write_single_event() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let writer = JsonlWriter::new(path);

        let event = TestEvent {
            id: 1,
            message: "test message".to_string(),
        };

        writer.write(&event).await.unwrap();

        // Read back and verify
        let content = fs::read_to_string(path).unwrap();
        let deserialized: TestEvent = serde_json::from_str(content.trim()).unwrap();
        
        assert_eq!(deserialized.id, 1);
        assert_eq!(deserialized.message, "test message");
    }

    #[tokio::test]
    async fn test_write_multiple_events() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let writer = JsonlWriter::new(path);

        // Write first event
        let event1 = TestEvent {
            id: 1,
            message: "first".to_string(),
        };
        writer.write(&event1).await.unwrap();

        // Write second event
        let event2 = TestEvent {
            id: 2,
            message: "second".to_string(),
        };
        writer.write(&event2).await.unwrap();

        // Read back and verify
        let file = fs::File::open(path).unwrap();
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>().unwrap();
        
        assert_eq!(lines.len(), 2);
        
        let deserialized1: TestEvent = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(deserialized1.id, 1);
        assert_eq!(deserialized1.message, "first");
        
        let deserialized2: TestEvent = serde_json::from_str(&lines[1]).unwrap();
        assert_eq!(deserialized2.id, 2);
        assert_eq!(deserialized2.message, "second");
    }

    #[tokio::test]
    async fn test_write_large_event() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let writer = JsonlWriter::new(path);

        let large_message = "x".repeat(10000);
        let event = TestEvent {
            id: 100,
            message: large_message.clone(),
        };

        writer.write(&event).await.unwrap();

        // Read back and verify
        let content = fs::read_to_string(path).unwrap();
        let deserialized: TestEvent = serde_json::from_str(content.trim()).unwrap();
        
        assert_eq!(deserialized.id, 100);
        assert_eq!(deserialized.message.len(), 10000);
    }
}
