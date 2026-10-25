use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::config::AppConfig;
use crate::recording::engine::RecordingEngine;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatus {
    pub is_recording: bool,
    /// True while the finished game is being saved (concat, audio mux, thumbnail).
    pub is_processing: bool,
    pub game: Option<String>,
    pub started_at: Option<String>,
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub config: RwLock<AppConfig>,
    pub recording_status: RwLock<RecordingStatus>,
    pub engine: Arc<RecordingEngine>,
}

impl AppState {
    pub fn new(db: Connection, config: AppConfig, engine: Arc<RecordingEngine>) -> Self {
        Self {
            db: Mutex::new(db),
            config: RwLock::new(config),
            recording_status: RwLock::new(RecordingStatus::default()),
            engine,
        }
    }

    /// Lock the DB, recovering from a poisoned mutex instead of panicking.
    pub fn db(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }
}
