// Valorant game-state detection through the Riot Client local API (no API key
// needed), with ShooterGame.log as fallback.

use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::OnceLock;

use base64::Engine as _;

use crate::detection::GameState;

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            // The local Riot API uses a self-signed certificate on 127.0.0.1.
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap_or_default()
    })
}

/// `None` = could not determine (keep the previous state).
pub async fn get_valorant_state() -> Option<GameState> {
    if let Some(s) = try_riot_local_api().await {
        return Some(s);
    }
    try_log_file()
}

fn lockfile_path() -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var("LOCALAPPDATA").ok()?)
        .join("Riot Games")
        .join("Riot Client")
        .join("Config")
        .join("lockfile");
    p.exists().then_some(p)
}

async fn try_riot_local_api() -> Option<GameState> {
    // Format: name:pid:port:password:protocol
    let content = std::fs::read_to_string(lockfile_path()?).ok()?;
    let parts: Vec<&str> = content.trim().split(':').collect();
    if parts.len() < 5 {
        return None;
    }
    let base = format!("https://127.0.0.1:{}", parts[2]);
    let auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", parts[3]))
    );

    let get = |path: &str| {
        client()
            .get(format!("{}{}", base, path))
            .header("Authorization", auth.clone())
            .send()
    };

    // Our own PUUID — the presences list also contains every friend, so
    // without this a friend in game would start a recording.
    let session: serde_json::Value = get("/chat/v1/session").await.ok()?.json().await.ok()?;
    let my_puuid = session.get("puuid")?.as_str()?.to_string();

    let json: serde_json::Value = get("/chat/v4/presences").await.ok()?.json().await.ok()?;
    let me = json
        .get("presences")?
        .as_array()?
        .iter()
        .find(|p| {
            p.get("puuid").and_then(|v| v.as_str()) == Some(my_puuid.as_str())
                && p.get("product").and_then(|v| v.as_str()) == Some("valorant")
        })?;

    parse_session_state(me)
}

fn parse_session_state(presence: &serde_json::Value) -> Option<GameState> {
    let encoded = presence.get("private").and_then(|v| v.as_str())?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).ok()?;
    let data: serde_json::Value = serde_json::from_slice(&decoded).ok()?;

    // Newer clients nest it under matchPresenceData, older ones at the root.
    let state = data
        .pointer("/matchPresenceData/sessionLoopState")
        .or_else(|| data.get("sessionLoopState"))
        .and_then(|v| v.as_str())?;

    Some(if state == "INGAME" { GameState::InGame } else { GameState::InMenu })
}

/// Fallback: look at the last markers in ShooterGame.log.
fn try_log_file() -> Option<GameState> {
    let path = PathBuf::from(std::env::var("LOCALAPPDATA").ok()?)
        .join("VALORANT")
        .join("Saved")
        .join("Logs")
        .join("ShooterGame.log");

    // Only read the tail: the log grows to tens of MB during a session.
    const TAIL: u64 = 64 * 1024;
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    f.seek(SeekFrom::Start(len.saturating_sub(TAIL))).ok()?;
    let mut buf = Vec::with_capacity(TAIL as usize);
    f.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);

    let in_game = text.rfind("GameState_InProgress");
    let in_menu = ["GameState_WaitingToStart", "GameState_LeavingMap", "returned to main menu"]
        .iter()
        .filter_map(|m| text.rfind(m))
        .max();

    match (in_game, in_menu) {
        (Some(g), Some(m)) => Some(if g > m { GameState::InGame } else { GameState::InMenu }),
        (Some(_), None) => Some(GameState::InGame),
        (None, Some(_)) => Some(GameState::InMenu),
        (None, None) => None,
    }
}
