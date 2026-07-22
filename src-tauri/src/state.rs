use parking_lot::Mutex;
use std::collections::VecDeque;

use crate::models::{LogEntry, TrackpadSettings};
use crate::platform::{create_backend, TrackpadBackend};

const MAX_LOGS: usize = 500;

pub struct AppState {
    pub backend: Box<dyn TrackpadBackend>,
    pub logs: Mutex<VecDeque<LogEntry>>,
    /// Last settings applied from UI (may be ahead of OS until apply succeeds).
    pub settings_cache: Mutex<TrackpadSettings>,
}

impl AppState {
    pub fn new() -> Self {
        let backend = create_backend();
        let settings = backend.get_settings().unwrap_or_default();
        Self {
            backend,
            logs: Mutex::new(VecDeque::with_capacity(128)),
            settings_cache: Mutex::new(settings),
        }
    }

    pub fn push_log(&self, level: &str, source: &str, message: impl Into<String>) {
        let entry = LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            level: level.to_string(),
            source: source.to_string(),
            message: message.into(),
        };
        log::log!(
            match level {
                "error" => log::Level::Error,
                "warn" => log::Level::Warn,
                "debug" => log::Level::Debug,
                _ => log::Level::Info,
            },
            "[{}] {}",
            source,
            entry.message
        );
        let mut logs = self.logs.lock();
        if logs.len() >= MAX_LOGS {
            logs.pop_front();
        }
        logs.push_back(entry);
    }

    pub fn logs_snapshot(&self) -> Vec<LogEntry> {
        self.logs.lock().iter().cloned().collect()
    }

    pub fn clear_logs(&self) {
        self.logs.lock().clear();
    }
}
