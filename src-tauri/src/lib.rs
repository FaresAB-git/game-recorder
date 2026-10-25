mod commands;
mod config;
mod detection;
mod library;
mod recording;
mod state;
mod tray;

use rusqlite::Connection;
use tauri::{Manager, WindowEvent};

use commands::*;
use config::load_config;
use library::{init_db, resync};
use recording::engine::RecordingEngine;
use state::AppState;

/// Arg added to the Windows "Run" entry so we know we were started at login.
const AUTOSTART_ARG: &str = "--autostart";

/// Log to `%LOCALAPPDATA%\com.gamerecorder.app\logs\gamerecorder.log` (release
/// builds have no console, so without this errors are invisible).
fn init_logging() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(log::LevelFilter::Info);

    if let Some(dir) = dirs::data_local_dir().map(|d| d.join("com.gamerecorder.app").join("logs")) {
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("gamerecorder.log");
        // Keep one previous session for comparison.
        if path.exists() {
            let _ = std::fs::rename(&path, dir.join("gamerecorder.old.log"));
        }
        if let Ok(file) = std::fs::File::create(&path) {
            builder.target(env_logger::Target::Pipe(Box::new(file)));
            builder.format(|buf, record| {
                use std::io::Write;
                writeln!(
                    buf,
                    "{} {:5} {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    record.level(),
                    record.args()
                )
            });
        }
    }
    builder.init();
    log::info!("GameRecorder {} started", env!("CARGO_PKG_VERSION"));
}

pub fn run() {
    init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // A second launch just brings the existing window forward.
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let config = load_config(&handle);

            let db_path = app.path().app_data_dir()?.join("library.db");
            std::fs::create_dir_all(db_path.parent().unwrap())?;
            let conn = Connection::open(&db_path)?;
            init_db(&conn)?;
            if let Err(e) = resync(&conn) {
                log::error!("Library resync error: {}", e);
            }
            std::fs::create_dir_all(&config.output_dir).ok();

            // Re-register autostart so the Run entry always points at this exe
            // (an old entry pointed at the debug build → console at login).
            config::apply_autostart(&handle, config.launch_at_startup);

            let engine = RecordingEngine::new();
            app.manage(AppState::new(conn, config.clone(), engine.clone()));
            tray::setup_tray(&handle)?;

            // Show the window, except when Windows started us and the user
            // asked to stay in the tray.
            let from_autostart = std::env::args().any(|a| a == AUTOSTART_ARG || a == "--hidden");
            if !(from_autostart && config.start_minimized) {
                tray::show_main_window(&handle);
            }

            // Probe the encoder now so no test process runs during a game.
            tauri::async_runtime::spawn(async move { engine.warmup_encoder().await });
            tauri::async_runtime::spawn(detection::run_detection_loop(handle));
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps the app running in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            get_replays,
            delete_replay,
            toggle_favorite,
            mark_seen,
            open_replay_in_explorer,
            open_output_folder,
            get_library_stats,
            get_recording_status,
            get_audio_devices,
            pick_output_directory,
            create_clip,
        ])
        .run(tauri::generate_context!())
        .expect("Error running GameRecorder");
}
