//! Tracing layer for capturing logs and exposing them to the frontend.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

const MAX_LOG_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: u64,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// Thread-safe log buffer with atomic ID generation for differential updates
#[derive(Debug, Clone)]
pub struct LogBuffer {
    entries: Arc<RwLock<VecDeque<LogEntry>>>,
    /// Atomic counter for generating unique, monotonically increasing log IDs
    next_id: Arc<AtomicU64>,
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_LOG_ENTRIES))),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Generate the next unique log ID atomically
    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Add a new log entry, removing old entries if buffer is full.
    /// The entry's ID will be set automatically.
    pub fn push(&self, mut entry: LogEntry) {
        entry.id = self.next_id();
        let mut entries = self.entries.write();
        if entries.len() >= MAX_LOG_ENTRIES {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    /// Get all log entries as a vector
    pub fn get_all(&self) -> Vec<LogEntry> {
        self.entries.read().iter().cloned().collect()
    }

    /// Get the most recent N entries efficiently using skip/take
    pub fn get_recent(&self, count: usize) -> Vec<LogEntry> {
        let entries = self.entries.read();
        let len = entries.len();
        let skip = len.saturating_sub(count);
        entries.iter().skip(skip).cloned().collect()
    }

    /// Get all log entries with ID greater than `last_id`.
    /// This enables differential updates - the frontend can track the last
    /// received ID and only fetch new logs.
    pub fn get_logs_since(&self, last_id: u64) -> Vec<LogEntry> {
        let entries = self.entries.read();
        entries.iter().filter(|e| e.id > last_id).cloned().collect()
    }

    /// Get the ID of the most recent log entry, or 0 if buffer is empty.
    /// Useful for the frontend to initialize its tracking state.
    pub fn get_latest_id(&self) -> u64 {
        self.entries.read().back().map(|e| e.id).unwrap_or(0)
    }

    /// Clear all log entries
    pub fn clear(&self) {
        self.entries.write().clear();
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

/// Visitor to extract the message from a tracing event
struct MessageVisitor {
    message: String,
}

impl MessageVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
        }
    }
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // Remove surrounding quotes if present
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        } else if self.message.is_empty() {
            // Fallback: use the first field as the message
            self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" || self.message.is_empty() {
            self.message = value.to_string();
        }
    }
}

/// A tracing layer that captures log events to a buffer
pub struct LogCaptureLayer {
    buffer: LogBuffer,
}

impl LogCaptureLayer {
    pub fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

impl<S> Layer<S> for LogCaptureLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = *metadata.level();

        // Get current timestamp
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();

        // Extract message from the event
        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);

        let entry = LogEntry {
            id: 0, // Will be assigned by LogBuffer::push()
            timestamp,
            level: level_to_string(level),
            target: metadata.target().to_string(),
            message: visitor.message,
        };

        self.buffer.push(entry);
    }
}

fn level_to_string(level: Level) -> String {
    match level {
        Level::TRACE => "TRACE".to_string(),
        Level::DEBUG => "DEBUG".to_string(),
        Level::INFO => "INFO".to_string(),
        Level::WARN => "WARN".to_string(),
        Level::ERROR => "ERROR".to_string(),
    }
}
