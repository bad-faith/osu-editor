use serde::{Deserialize, Serialize};

use crate::{
    dotosu::osu_file::OsuFile,
    map_format::{
        colors::Colors, diff_settings::DiffSettings, events::Events, general::General,
        objects::Objects, timing::Timing,
    },
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Beatmap {
    pub id: i64,
    pub version: String,
    pub general: General,
    pub diff_settings: DiffSettings,
    pub colors: Colors,
    pub events: Events,
    pub objects: Objects,
    pub timing: Timing,
}

impl Beatmap {
    pub fn from_osu_format(beatmap: &OsuFile) -> Option<Self> {
        let timing = Timing::from_osu_format(&beatmap.timing, &beatmap.general);
        let diff_settings = DiffSettings::from_osu_format(&beatmap.difficulty, &beatmap.general);
        Some(Beatmap {
            general: General::from_osu_format(&beatmap.general),
            diff_settings: diff_settings.clone(),
            colors: Colors::from_osu_format(&beatmap.colours),
            events: Events::from_osu_format(&beatmap.events),
            id: beatmap.metadata.beatmap_id,
            version: beatmap.metadata.version.clone(),
            objects: Objects::from_osu_format(&beatmap.objects, &timing, &diff_settings)?,
            timing: timing,
        })
    }
}
