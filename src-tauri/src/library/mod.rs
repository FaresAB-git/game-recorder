pub mod models;

use models::Replay;
use rusqlite::{params, Connection, OptionalExtension, Result, Row};
use std::path::Path;

const COLUMNS: &str = "id, game, date, duration, size, path, thumbnail_path, favorite, is_new, is_clip, source_id";

/// Create tables and run migrations.
pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS replays (
            id             TEXT PRIMARY KEY,
            game           TEXT NOT NULL,
            date           TEXT NOT NULL,
            duration       INTEGER NOT NULL,
            size           INTEGER NOT NULL DEFAULT 0,
            path           TEXT NOT NULL,
            thumbnail_path TEXT,
            favorite       INTEGER NOT NULL DEFAULT 0,
            is_new         INTEGER NOT NULL DEFAULT 0,
            is_clip        INTEGER NOT NULL DEFAULT 0,
            source_id      TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_replays_date ON replays(date);",
    )?;
    // Migrations for databases created by older versions (errors = column already exists).
    let _ = conn.execute("ALTER TABLE replays ADD COLUMN is_clip INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE replays ADD COLUMN source_id TEXT", []);
    Ok(())
}

fn row_to_replay(row: &Row) -> Result<Replay> {
    Ok(Replay {
        id: row.get(0)?,
        game: row.get(1)?,
        date: row.get(2)?,
        duration: row.get(3)?,
        size: row.get(4)?,
        path: row.get(5)?,
        thumbnail_path: row.get(6)?,
        favorite: row.get::<_, i64>(7)? != 0,
        is_new: row.get::<_, i64>(8)? != 0,
        is_clip: row.get::<_, i64>(9)? != 0,
        source_id: row.get(10)?,
    })
}

pub fn insert_replay(conn: &Connection, r: &Replay) -> Result<()> {
    conn.execute(
        &format!("INSERT OR REPLACE INTO replays ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"),
        params![
            r.id, r.game, r.date, r.duration, r.size, r.path, r.thumbnail_path,
            r.favorite as i64, r.is_new as i64, r.is_clip as i64, r.source_id,
        ],
    )?;
    Ok(())
}

/// All replays and clips, newest first.
pub fn get_all_replays(conn: &Connection) -> Result<Vec<Replay>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLUMNS} FROM replays ORDER BY date DESC"))?;
    let rows = stmt.query_map([], row_to_replay)?;
    rows.collect()
}

pub fn get_replay_by_id(conn: &Connection, id: &str) -> Result<Option<Replay>> {
    conn.query_row(&format!("SELECT {COLUMNS} FROM replays WHERE id = ?1"), params![id], row_to_replay)
        .optional()
}

/// Delete a replay's files then its DB row. Fails (and keeps the row) if the
/// video file can't be removed, e.g. because it's still open in the player.
pub fn delete_replay_by_id(conn: &Connection, id: &str) -> std::result::Result<(), String> {
    let Some(r) = get_replay_by_id(conn, id).map_err(|e| e.to_string())? else {
        return Ok(());
    };
    let path = Path::new(&r.path);
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| format!("Impossible de supprimer le fichier : {}", e))?;
    }
    if let Some(thumb) = r.thumbnail_path {
        let _ = std::fs::remove_file(thumb);
    }
    conn.execute("DELETE FROM replays WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

/// Toggle favorite and return the new value.
pub fn toggle_favorite_by_id(conn: &Connection, id: &str) -> Result<bool> {
    conn.execute("UPDATE replays SET favorite = 1 - favorite WHERE id = ?1", params![id])?;
    conn.query_row("SELECT favorite FROM replays WHERE id = ?1", params![id], |r| r.get::<_, i64>(0))
        .map(|v| v != 0)
}

/// Clear the "new" badge of one replay, or of all of them when `id` is `None`.
pub fn clear_new_flags(conn: &Connection, id: Option<&str>) -> Result<()> {
    match id {
        Some(id) => conn.execute("UPDATE replays SET is_new = 0 WHERE id = ?1", params![id])?,
        None => conn.execute("UPDATE replays SET is_new = 0 WHERE is_new = 1", [])?,
    };
    Ok(())
}

/// Total size of all files in the library, in bytes.
pub fn get_total_size(conn: &Connection) -> Result<u64> {
    conn.query_row("SELECT COALESCE(SUM(size), 0) FROM replays", [], |r| r.get::<_, i64>(0))
        .map(|v| v.max(0) as u64)
}

/// Delete the oldest game recordings until under the limits.
/// Favorites and clips are never deleted automatically.
pub fn enforce_limits(conn: &Connection, max_storage_bytes: u64, max_replays: usize) -> Result<Vec<String>> {
    let mut deleted = Vec::new();
    loop {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM replays WHERE is_clip = 0",
            [],
            |r| r.get(0),
        )?;
        let total = get_total_size(conn)?;
        if count as usize <= max_replays && total <= max_storage_bytes {
            break;
        }
        let oldest: Option<String> = conn
            .query_row(
                "SELECT id FROM replays WHERE favorite = 0 AND is_clip = 0 ORDER BY date ASC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let Some(id) = oldest else { break };
        if let Err(e) = delete_replay_by_id(conn, &id) {
            log::warn!("Auto-cleanup could not delete {}: {}", id, e);
            break;
        }
        log::info!("Auto-cleanup: deleted {}", id);
        deleted.push(id);
    }
    Ok(deleted)
}

/// Sync the DB with the disk: drop rows whose file is gone, refresh sizes and
/// forget missing thumbnails.
pub fn resync(conn: &Connection) -> Result<()> {
    let replays = get_all_replays(conn)?;
    let mut removed = 0;
    for r in &replays {
        let path = Path::new(&r.path);
        let Ok(meta) = std::fs::metadata(path) else {
            conn.execute("DELETE FROM replays WHERE id = ?1", params![r.id])?;
            removed += 1;
            continue;
        };
        if meta.len() as i64 != r.size {
            conn.execute("UPDATE replays SET size = ?1 WHERE id = ?2", params![meta.len() as i64, r.id])?;
        }
        if let Some(t) = &r.thumbnail_path {
            if !Path::new(t).exists() {
                conn.execute("UPDATE replays SET thumbnail_path = NULL WHERE id = ?1", params![r.id])?;
            }
        }
    }
    log::info!("Library: {} entries ({} missing files removed)", replays.len() - removed, removed);
    Ok(())
}
