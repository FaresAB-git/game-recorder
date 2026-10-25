pub mod valorant;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{interval, Duration, MissedTickBehavior};

use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameType {
    Lol,
    Valorant,
}

impl GameType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GameType::Lol => "lol",
            GameType::Valorant => "valorant",
        }
    }

    /// Short tag used in file names.
    pub fn tag(&self) -> &'static str {
        match self {
            GameType::Lol => "LoL",
            GameType::Valorant => "VALO",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            GameType::Lol => "League of Legends",
            GameType::Valorant => "Valorant",
        }
    }

    /// The in-game process (not the launcher / client).
    pub fn process_name(&self) -> &'static str {
        match self {
            GameType::Lol => "league of legends.exe",
            GameType::Valorant => "valorant-win64-shipping.exe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    NotRunning,
    InMenu,
    InGame,
}

/// Valorant needs this many consecutive "not in game" reads (×2 s) before we
/// stop, so a single failed API call doesn't cut the recording.
const VALORANT_END_CONFIRMATIONS: u32 = 3;

/// Background detection loop, runs for the lifetime of the app.
pub async fn run_detection_loop(app: AppHandle) {
    let mut ticker = interval(Duration::from_secs(2));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let mut sys = System::new();
    let mut recording: Option<GameType> = None;
    let mut valorant_misses = 0u32;

    loop {
        ticker.tick().await;

        let config = app.state::<AppState>().config.read().await.clone();

        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::new());
        let running: HashSet<String> = sys
            .processes()
            .values()
            .map(|p| p.name().to_string_lossy().to_lowercase())
            .collect();

        let lol = if config.enable_lol && running.contains(GameType::Lol.process_name()) {
            GameState::InGame
        } else {
            GameState::NotRunning
        };

        let valorant = if !config.enable_valorant || !running.contains(GameType::Valorant.process_name()) {
            GameState::NotRunning
        } else {
            match valorant::get_valorant_state().await {
                Some(s) => s,
                // Unknown: keep whatever we had.
                None if recording == Some(GameType::Valorant) => GameState::InGame,
                None => GameState::InMenu,
            }
        };

        match recording {
            None => {
                let game = if lol == GameState::InGame {
                    Some(GameType::Lol)
                } else if valorant == GameState::InGame {
                    Some(GameType::Valorant)
                } else {
                    None
                };
                if let Some(game) = game {
                    if handle_game_start(&app, game).await {
                        recording = Some(game);
                        valorant_misses = 0;
                    }
                }
            }
            Some(GameType::Lol) => {
                if lol != GameState::InGame {
                    handle_game_end(&app).await;
                    recording = None;
                }
            }
            Some(GameType::Valorant) => {
                if valorant == GameState::InGame {
                    valorant_misses = 0;
                } else {
                    valorant_misses += 1;
                    // Process gone = game definitely over.
                    if valorant == GameState::NotRunning || valorant_misses >= VALORANT_END_CONFIRMATIONS {
                        handle_game_end(&app).await;
                        recording = None;
                    }
                }
            }
        }
    }
}

async fn set_status(app: &AppHandle, f: impl FnOnce(&mut crate::state::RecordingStatus)) {
    let state = app.state::<AppState>();
    let mut status = state.recording_status.write().await;
    f(&mut status);
    let _ = app.emit("recording_status_changed", status.clone());
}

fn notify(app: &AppHandle, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title("GameRecorder").body(body).show();
}

/// Returns true if the recording actually started.
async fn handle_game_start(app: &AppHandle, game: GameType) -> bool {
    log::info!("Game started: {}", game.as_str());
    let state = app.state::<AppState>();
    let config = state.config.read().await.clone();

    if let Err(e) = state.engine.start_recording(game, &config).await {
        log::error!("Failed to start recording: {}", e);
        if config.show_notifications {
            notify(app, &format!("Impossible d'enregistrer {} : {}", game.display_name(), e));
        }
        return false;
    }

    set_status(app, |s| {
        s.is_recording = true;
        s.game = Some(game.as_str().to_string());
        s.started_at = Some(chrono::Utc::now().to_rfc3339());
    })
    .await;

    if config.show_notifications {
        notify(app, &format!("Enregistrement {} démarré", game.display_name()));
    }
    true
}

/// Stop the current recording, save it to the library and tell the UI.
/// Also used when quitting the app mid-game.
pub async fn handle_game_end(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.engine.is_recording().await {
        return;
    }
    log::info!("Game ended — finalizing recording");

    set_status(app, |s| {
        s.is_recording = false;
        s.is_processing = true;
    })
    .await;

    let config = state.config.read().await.clone();
    let result = state.engine.stop_recording(&config).await;

    set_status(app, |s| {
        s.is_processing = false;
        s.game = None;
        s.started_at = None;
    })
    .await;

    match result {
        Ok(Some(replay)) => {
            log::info!("Recording saved: {} ({})", replay.id, replay.path);
            let deleted = {
                let db = state.db();
                if let Err(e) = crate::library::insert_replay(&db, &replay) {
                    log::error!("Failed to insert replay: {}", e);
                }
                let max_bytes = (config.max_storage_gb * 1024.0 * 1024.0 * 1024.0) as u64;
                crate::library::enforce_limits(&db, max_bytes, config.max_replays).unwrap_or_default()
            };

            let _ = app.emit("new_replay", &replay);
            if !deleted.is_empty() {
                let _ = app.emit("replays_deleted", &deleted);
            }
            if config.auto_open_after_game {
                crate::tray::show_main_window(app);
            }
        }
        Ok(None) => log::info!("Recording discarded (too short or empty)"),
        Err(e) => {
            log::error!("Failed to stop recording: {}", e);
            if config.show_notifications {
                notify(app, &format!("Erreur lors de la sauvegarde : {}", e));
            }
        }
    }
}
