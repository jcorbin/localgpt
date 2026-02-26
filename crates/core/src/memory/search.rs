//! Memory search types and utilities

use serde::{Deserialize, Serialize};

/// A chunk of memory content returned from search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryChunk {
    /// File path relative to workspace
    pub file: String,

    /// Starting line number (1-indexed)
    pub line_start: i32,

    /// Ending line number (1-indexed)
    pub line_end: i32,

    /// The actual content
    pub content: String,

    /// Relevance score (higher is better)
    pub score: f64,

    /// Unix timestamp when the chunk was last updated (for temporal decay)
    #[serde(default)]
    pub updated_at: i64,
}

impl MemoryChunk {
    /// Create a new memory chunk
    pub fn new(file: String, line_start: i32, line_end: i32, content: String, score: f64) -> Self {
        Self {
            file,
            line_start,
            line_end,
            content,
            score,
            updated_at: 0,
        }
    }

    /// Create a new memory chunk with timestamp
    pub fn with_timestamp(mut self, updated_at: i64) -> Self {
        self.updated_at = updated_at;
        self
    }

    /// Apply temporal decay to the score based on age.
    /// decay_factor = exp(-lambda * age_days)
    /// Returns the decayed score.
    pub fn apply_temporal_decay(&mut self, lambda: f64, now_unix: i64) -> f64 {
        if lambda <= 0.0 || self.updated_at <= 0 {
            return self.score;
        }

        let age_secs = (now_unix - self.updated_at).max(0) as f64;
        let age_days = age_secs / (24.0 * 60.0 * 60.0);
        let decay_factor = (-lambda * age_days).exp();

        self.score *= decay_factor;
        self.score
    }

    /// Get a preview of the content (first N characters)
    pub fn preview(&self, max_len: usize) -> String {
        if self.content.len() <= max_len {
            self.content.clone()
        } else {
            format!(
                "{}...",
                &self.content[..self.content.floor_char_boundary(max_len)]
            )
        }
    }

    /// Get the location string (file:line)
    pub fn location(&self) -> String {
        if self.line_start == self.line_end {
            format!("{}:{}", self.file, self.line_start)
        } else {
            format!("{}:{}-{}", self.file, self.line_start, self.line_end)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_chunk_preview() {
        let chunk = MemoryChunk::new(
            "test.md".to_string(),
            1,
            5,
            "This is a long content string that should be truncated".to_string(),
            0.9,
        );

        assert_eq!(chunk.preview(20), "This is a long conte...");
        assert_eq!(chunk.location(), "test.md:1-5");
    }

    #[test]
    fn test_memory_chunk_single_line_location() {
        let chunk = MemoryChunk::new(
            "test.md".to_string(),
            10,
            10,
            "Single line".to_string(),
            0.5,
        );

        assert_eq!(chunk.location(), "test.md:10");
    }

    #[test]
    fn test_memory_chunk_preview_multibyte() {
        // Emoji are 4 bytes each in UTF-8
        let chunk = MemoryChunk::new(
            "test.md".to_string(),
            1,
            1,
            "Hello 🌍🌎🌏 world".to_string(),
            1.0,
        );

        // max_len=8 lands inside the first emoji (bytes 6-9), should not panic
        let preview = chunk.preview(8);
        assert!(preview.ends_with("..."));
        // Should truncate to "Hello " (6 bytes) since byte 8 is mid-emoji
        assert_eq!(preview, "Hello ...");
    }

    #[test]
    fn test_memory_chunk_preview_emdash() {
        // Em-dash (—) is 3 bytes in UTF-8
        let chunk = MemoryChunk::new(
            "test.md".to_string(),
            1,
            1,
            "one—two—three—four—five".to_string(),
            1.0,
        );

        // "one—" is 3 + 3 = 6 bytes; max_len=5 lands mid-emdash
        let preview = chunk.preview(5);
        assert!(preview.ends_with("..."));
        assert_eq!(preview, "one...");
    }

    #[test]
    fn test_temporal_decay_no_decay() {
        // Lambda = 0 means no decay
        let mut chunk = MemoryChunk::new("test.md".to_string(), 1, 1, "content".to_string(), 1.0);
        chunk.updated_at = 1_700_000_000; // Some old timestamp

        let decayed = chunk.apply_temporal_decay(0.0, 1_710_000_000);
        assert!((decayed - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_temporal_decay_seven_days() {
        // Lambda = 0.1: 7-day old memory should get ~50% penalty
        let mut chunk = MemoryChunk::new("test.md".to_string(), 1, 1, "content".to_string(), 1.0);
        let now = 1_710_000_000i64;
        chunk.updated_at = now - (7 * 24 * 60 * 60); // 7 days ago

        let decayed = chunk.apply_temporal_decay(0.1, now);
        // exp(-0.1 * 7) ≈ 0.496
        assert!((decayed - 0.496).abs() < 0.01);
    }

    #[test]
    fn test_temporal_decay_fresh() {
        // Fresh memory (just updated) should have no penalty
        let mut chunk = MemoryChunk::new("test.md".to_string(), 1, 1, "content".to_string(), 1.0);
        let now = 1_710_000_000i64;
        chunk.updated_at = now;

        let decayed = chunk.apply_temporal_decay(0.1, now);
        assert!((decayed - 1.0).abs() < 0.001);
    }
}
