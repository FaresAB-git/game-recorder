use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Replay {
    pub id: String,
    pub game: String,   // "lol" | "valorant"
    pub date: String,   // ISO 8601
    pub duration: i64,  // seconds
    pub size: i64,      // bytes
    pub path: String,
    pub thumbnail_path: Option<String>,
    pub favorite: bool,
    pub is_new: bool,
    pub is_clip: bool,
    /// For clips: id of the game recording it was cut from.
    pub source_id: Option<String>,
}
