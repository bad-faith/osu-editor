use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Beatmapset {
    pub id: i64,
    pub audio_filename: String,
    pub audio_lead_in: f64,
    pub preview_time: i64,
    pub title: String,
    pub title_unicode: String,
    pub artist: String,
    pub artist_unicode: String,
    pub creator: String,
    pub source: String,
    pub tags: String,
}
