//! Custom tracing layer for capturing logs and exposing them to the frontend.
//!
//! This module provides a `LogCapture` layer that stores log entries in a
//! bounded circular buffer, allowing the UI to display recent log messages.

use std::collections::VecDeque;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// Maximum number of log entries to keep in memory
const MAX_LOG_ENTRIES: usize = 500;

/// A single log entry with timestamp, level, and message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// Thread-safe log buffer
#[derive(Debug, Clone, Default)]
pub struct LogBuffer {
    entries: Arc<RwLock<VecDeque<LogEntry>>>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_LOG_ENTRIES))),
        }
    }

    /// Add a new log entry, removing old entries if buffer is full
    pub fn push(&self, entry: LogEntry) {
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

    /// Get the most recent N entries
    pub fn get_recent(&self, count: usize) -> Vec<LogEntry> {
        let entries = self.entries.read();
        entries.iter().rev().take(count).rev().cloned().collect()
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
