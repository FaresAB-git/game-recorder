use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};

use crate::config::AppConfig;
use crate::library::models::Replay;
use crate::library::{
    clear_new_flags, delete_replay_by_id, get_all_replays, get_replay_by_id, get_total_size,
    insert_replay, toggle_favorite_by_id,
};
use crate::recording::ffmpeg::{find_ffmpeg, generate_thumbnail, get_duration, trim_to_clip};
use crate::state::{AppState, RecordingStatus};

// ── Config ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.config.read().await.clone())
}

#[tauri::command]
pub async fn save_config(app: AppHandle, state: State<'_, AppState>, config: AppConfig) -> Result<AppConfig, String> {
    let config = config.sanitized();
    crate::config::save_config(&app, &config)?;
    std::fs::create_dir_all(&config.output_dir).map_err(|e| format!("Dossier de sortie invalide : {}", e))?;

    let autostart_changed = {
        let mut cfg = state.config.write().await;
        let changed = cfg.launch_at_startup != config.launch_at_startup;
        *cfg = config.clone();
        changed
    };
    if autostart_changed {
        crate::config::apply_autostart(&app, config.launch_at_startup);
    }
    Ok(config)
}

// ── Library ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_replays(state: State<'_, AppState>) -> Result<Vec<Replay>, String> {
    get_all_replays(&state.db()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_replay(state: State<'_, AppState>, id: String) -> Result<(), String> {
    delete_replay_by_id(&state.db(), &id)
}

#[tauri::command]
pub async fn toggle_favorite(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    toggle_favorite_by_id(&state.db(), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mark_seen(state: State<'_, AppState>, id: Option<String>) -> Result<(), String> {
    clear_new_flags(&state.db(), id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_replay_in_explorer(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let replay = get_replay_by_id(&state.db(), &id)
        .map_err(|e| e.to_string())?
        .ok_or("Replay introuvable")?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // explorer needs `/select,"path"` verbatim; default arg quoting breaks it.
        std::process::Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", replay.path))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn open_output_folder(state: State<'_, AppState>) -> Result<(), String> {
    let dir = state.config.read().await.output_dir.clone();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer").arg(&dir).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_library_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let (max_gb, output_dir) = {
        let c = state.config.read().await;
        (c.max_storage_gb, c.output_dir.clone())
    };
    let (used_bytes, total_replays) = {
        let db = state.db();
        let used = get_total_size(&db).map_err(|e| e.to_string())?;
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM replays WHERE is_clip = 0", [], |r| r.get(0))
            .unwrap_or(0);
        (used, count)
    };
    Ok(serde_json::json!({
        "storage": {
            "usedBytes": used_bytes,
            "maxBytes": (max_gb * 1024.0 * 1024.0 * 1024.0) as u64,
            "freeBytes": free_space_for(Path::new(&output_dir)),
        },
        "totalReplays": total_replays,
    }))
}

// ── Recording ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_recording_status(state: State<'_, AppState>) -> Result<RecordingStatus, String> {
    Ok(state.recording_status.read().await.clone())
}

// ── System ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(crate::recording::audio::list_mic_devices)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pick_output_directory(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |dir| {
        let _ = tx.send(dir);
    });
    let result = rx.await.map_err(|e| e.to_string())?;
    Ok(result.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().to_string()))
}

// ── Clips ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_clip(state: State<'_, AppState>, id: String, start_sec: f64, end_sec: f64) -> Result<Replay, String> {
    if !start_sec.is_finite() || !end_sec.is_finite() || start_sec < 0.0 || end_sec - start_sec < 1.0 {
        return Err("Le clip doit durer au moins 1 seconde".into());
    }

    let source = get_replay_by_id(&state.db(), &id)
        .map_err(|e| e.to_string())?
        .ok_or("Replay introuvable")?;
    // A clip of a clip still points to the original game.
    let source_id = source.source_id.clone().unwrap_or_else(|| source.id.clone());
    let source_path = PathBuf::from(&source.path);

    let clips_dir = PathBuf::from(&state.config.read().await.output_dir).join("clips");
    std::fs::create_dir_all(&clips_dir).map_err(|e| e.to_string())?;

    let ffmpeg = find_ffmpeg().ok_or("FFmpeg introuvable")?;
    let clip_id = uuid::Uuid::new_v4().to_string();
    let stem = source_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let clip_path = clips_dir.join(format!("{}_clip_{}.mp4", stem, &clip_id[..8]));

    trim_to_clip(&ffmpeg, &source_path, &clip_path, start_sec, end_sec)
        .await
        .map_err(|e| e.to_string())?;

    let duration = get_duration(&clip_path).await.unwrap_or(end_sec - start_sec);
    let thumb = clip_path.with_extension("jpg");
    let thumbnail_path = generate_thumbnail(&ffmpeg, &clip_path, &thumb, duration)
        .await
        .ok()
        .map(|_| thumb.to_string_lossy().to_string());

    let clip = Replay {
        id: clip_id,
        game: source.game,
        date: chrono::Utc::now().to_rfc3339(),
        duration: duration.round() as i64,
        size: std::fs::metadata(&clip_path).map(|m| m.len() as i64).unwrap_or(0),
        path: clip_path.to_string_lossy().to_string(),
        thumbnail_path,
        favorite: false,
        is_new: false,
        is_clip: true,
        source_id: Some(source_id),
    };
    insert_replay(&state.db(), &clip).map_err(|e| e.to_string())?;
    Ok(clip)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Free space on the disk that holds `dir` (not just the first disk).
fn free_space_for(dir: &Path) -> u64 {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    let dir = dir.to_string_lossy().to_lowercase();
    disks
        .iter()
        .filter(|d| dir.starts_with(&d.mount_point().to_string_lossy().to_lowercase()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space())
        .unwrap_or(0)
}
