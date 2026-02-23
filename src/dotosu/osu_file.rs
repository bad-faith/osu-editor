use crate::dotosu::{
    helpers::get_section,
    sections::{
        colours::{ColoursSection, parse_colours_section},
        difficulty::{DifficultySection, parse_difficulty_section},
        events::{EventsSection, parse_events_section},
        general::{GeneralSection, parse_general_section},
        metadata::{MetadataSection, parse_metadata_section},
        objects::{HitObjectsSection, parse_objects_section},
        timing::{TimingSection, parse_timing_section},
    },
};

pub struct OsuFile {
    pub general: GeneralSection,
    pub metadata: MetadataSection,
    pub difficulty: DifficultySection,
    pub events: EventsSection,
    pub timing: TimingSection,
    pub colours: ColoursSection,
    pub objects: HitObjectsSection,
}

impl OsuFile {
    pub fn to_osu_text(&self) -> String {
        // Minimal round-trip: we keep the parsed raw section bodies and re-emit them.
        // This avoids needing temp files or a separate serializer format.
        format!(
            "osu file format v14\n\n[General]\n{}\n[Metadata]\n{}\n[Difficulty]\n{}\n[Events]\n{}\n[TimingPoints]\n{}\n[Colours]\n{}\n[HitObjects]\n{}\n",
            self.general.to_osu_text(),
            self.metadata.to_osu_text(),
            self.difficulty.to_osu_text(),
            self.events.to_osu_text(),
            self.timing.to_osu_text(),
            self.colours.to_osu_text(),
            self.objects.to_osu_text()
        )
    }
}

pub fn parse_osu_file(
    file_name: String,
    osu_data: &[u8],
    prompt_missing_value: &mut dyn FnMut(&str) -> Option<String>,
) -> Option<OsuFile> {
    let osu_text = match std::str::from_utf8(osu_data) {
        Ok(text) => text,
        Err(err) => {
            println!("Failed to parse .osu file: {}", err);
            return None;
        }
    };
    let general = match get_section(osu_text, "General") {
        Some(section) => section,
        None => {
            println!("No [General] section found in .osu file.");
            return None;
        }
    };
    let metadata = match get_section(osu_text, "Metadata") {
        Some(section) => section,
        None => {
            println!("No [Metadata] section found in .osu file.");
            return None;
        }
    };
    let difficulty = match get_section(osu_text, "Difficulty") {
        Some(section) => section,
        None => {
            println!("No [Difficulty] section found in .osu file.");
            return None;
        }
    };
    let events = match get_section(osu_text, "Events") {
        Some(section) => section,
        None => {
            println!("No [Events] section found in .osu file.");
            return None;
        }
    };
    let timing = match get_section(osu_text, "TimingPoints") {
        Some(section) => section,
        None => {
            println!("No [TimingPoints] section found in .osu file.");
            return None;
        }
    };
    let colours = match get_section(osu_text, "Colours") {
        Some(section) => section,
        None => {
            println!("No [Colours] section found in .osu file.");
            return None;
        }
    };
    let objects = match get_section(osu_text, "HitObjects") {
        Some(section) => section,
        None => {
            println!("No [HitObjects] section found in .osu file.");
            return None;
        }
    };

    let metadata = match parse_metadata_section(file_name, metadata, prompt_missing_value) {
        Some(meta) => meta,
        None => {
            println!("Failed to parse metadata section in .osu file.");
            return None;
        }
    };

    let general = match parse_general_section(general) {
        Some(general) => general,
        None => {
            println!("Failed to parse general section in .osu file.");
            return None;
        }
    };

    let difficulty = match parse_difficulty_section(difficulty) {
        Some(diff) => diff,
        None => {
            println!("Failed to parse difficulty section in .osu file.");
            return None;
        }
    };

    let events = match parse_events_section(events) {
        Some(ev) => ev,
        None => {
            println!("Failed to parse events section in .osu file.");
            return None;
        }
    };

    let timing = match parse_timing_section(timing) {
        Some(tp) => tp,
        None => {
            println!("Failed to parse timing section in .osu file.");
            return None;
        }
    };

    let colours = match parse_colours_section(colours) {
        Some(col) => col,
        None => {
            println!("Failed to parse colours section in .osu file.");
            return None;
        }
    };

    let objects = match parse_objects_section(objects) {
        Some(ho) => ho,
        None => {
            println!("Failed to parse hit objects section in .osu file.");
            return None;
        }
    };

    return Some(OsuFile {
        general,
        metadata,
        difficulty,
        events,
        timing,
        colours,
        objects,
    });
}
