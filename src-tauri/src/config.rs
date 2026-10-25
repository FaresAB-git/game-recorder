use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub output_dir: String,
    pub max_storage_gb: f64,
    pub max_replays: usize,
    pub min_game_duration_minutes: u32,
    pub resolution: String,
    pub fps: u32,
    pub bitrate: u32,
    pub encoder: String,
    pub capture_game_audio: bool,
    pub capture_mic: bool,
    pub mic_device: Option<String>,
    pub enable_lol: bool,
    pub enable_valorant: bool,
    pub launch_at_startup: bool,
    /// When launched by Windows at login, stay in the tray instead of opening the window.
    pub start_minimized: bool,
    pub auto_open_after_game: bool,
    pub show_notifications: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        let output_dir = dirs::video_dir()
            .unwrap_or_else(|| PathBuf::from("C:/Users/Public/Videos"))
            .join("GameRecorder")
            .to_string_lossy()
            .to_string();

        Self {
            output_dir,
            max_storage_gb: 50.0,
            max_replays: 30,
            min_game_duration_minutes: 3,
            resolution: "native".to_string(),
            fps: 60,
            bitrate: 20000,
            encoder: "auto".to_string(),
            capture_game_audio: true,
            capture_mic: false,
            mic_device: None,
            enable_lol: true,
            enable_valorant: true,
            launch_at_startup: false,
            start_minimized: false,
            auto_open_after_game: true,
            show_notifications: true,
        }
    }
}

impl AppConfig {
    /// Clamp values coming from the UI into sane ranges.
    pub fn sanitized(mut self) -> Self {
        if self.output_dir.trim().is_empty() {
            self.output_dir = AppConfig::default().output_dir;
        }
        self.max_storage_gb = self.max_storage_gb.clamp(1.0, 10_000.0);
        self.max_replays = self.max_replays.clamp(1, 10_000);
        self.min_game_duration_minutes = self.min_game_duration_minutes.min(60);
        self.fps = if self.fps >= 60 { 60 } else { 30 };
        self.bitrate = self.bitrate.clamp(1000, 150_000);
        if !matches!(self.resolution.as_str(), "native" | "1080p" | "720p") {
            self.resolution = "native".into();
        }
        if !matches!(self.encoder.as_str(), "auto" | "nvenc" | "amf" | "qsv" | "x264") {
            self.encoder = "auto".into();
        }
        self
    }
}

fn config_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config.json")
}

pub fn load_config(app: &AppHandle) -> AppConfig {
    let path = config_path(app);
    match std::fs::read_to_string(&path) {
        // `#[serde(default)]` keeps the user's values when new fields are added.
        Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
            Ok(c) => c.sanitized(),
            Err(e) => {
                log::error!("Invalid config.json ({}), using defaults", e);
                AppConfig::default()
            }
        },
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// Register / unregister the app in the Windows "Run" key.
///
/// Debug builds are never registered: they have a console window, which is
/// what showed up at login instead of the app.
pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    if cfg!(debug_assertions) {
        if enabled {
            log::warn!("Autostart ignored in debug builds — build the release version to use it");
        }
        // Clean up an entry left by an older debug build.
        if autostart.is_enabled().unwrap_or(false) {
            let _ = autostart.disable();
        }
        return;
    }
    let res = if enabled { autostart.enable() } else { autostart.disable() };
    if let Err(e) = res {
        log::error!("Autostart update failed: {}", e);
    }
}
