use super::super::helpers::get_key_value_pairs;

pub struct MetadataSection {
    pub title: String,
    pub title_unicode: String,
    pub artist: String,
    pub artist_unicode: String,
    pub creator: String,
    pub version: String,
    pub source: String,
    pub tags: String,
    pub beatmap_id: i64,
    pub beatmapset_id: i64,
}

impl MetadataSection {
    pub fn to_osu_text(&self) -> String {
        format!(
            "Title:{}\nTitleUnicode:{}\nArtist:{}\nArtistUnicode:{}\nCreator:{}\nVersion:{}\nSource:{}\nTags:{}\nBeatmapID:{}\nBeatmapSetID:{}\n",
            self.title,
            self.title_unicode,
            self.artist,
            self.artist_unicode,
            self.creator,
            self.version,
            self.source,
            self.tags,
            self.beatmap_id,
            self.beatmapset_id,
        )
    }
}

fn prompt_i64(
    file_name: &str,
    field_name: &str,
    prompt_missing_value: &mut dyn FnMut(&str) -> Option<String>,
) -> Option<i64> {
    let prompt = format!(
        "beatmap file {} is missing {}, type it manually",
        file_name, field_name
    );
    loop {
        let input = prompt_missing_value(prompt.as_str())?;
        let value = input.trim();
        match value.parse::<i64>() {
            Ok(parsed) => return Some(parsed),
            Err(err) => {
                println!(
                    "Metadata parsing error: '{}'={} is not a valid i64: {}",
                    field_name, value, err
                );
            }
        }
    }
}

pub fn parse_metadata_section(
    file_name: String,
    section: &str,
    prompt_missing_value: &mut dyn FnMut(&str) -> Option<String>,
) -> Option<MetadataSection> {
    let pairs = get_key_value_pairs(section);
    let pairs = match pairs {
        Some(p) => p,
        None => {
            println!("Failed to parse metadata section due to duplicate keys.");
            return None;
        }
    };
    let title = match pairs.get("Title") {
        None => {
            println!("Metadata parsing error: Missing 'Title' field.");
            return None;
        }
        Some(title) => title.to_string(),
    };
    let title_unicode = match pairs.get("TitleUnicode") {
        None => {
            println!("Metadata parsing error: Missing 'TitleUnicode' field.");
            return None;
        }
        Some(title_unicode) => title_unicode.to_string(),
    };
    let artist = match pairs.get("Artist") {
        None => {
            println!("Metadata parsing error: Missing 'Artist' field.");
            return None;
        }
        Some(artist) => artist.to_string(),
    };
    let artist_unicode = match pairs.get("ArtistUnicode") {
        None => {
            println!("Metadata parsing error: Missing 'ArtistUnicode' field.");
            return None;
        }
        Some(artist_unicode) => artist_unicode.to_string(),
    };
    let creator = match pairs.get("Creator") {
        None => {
            println!("Metadata parsing error: Missing 'Creator' field.");
            return None;
        }
        Some(creator) => creator.to_string(),
    };
    let version = match pairs.get("Version") {
        None => {
            println!("Metadata parsing error: Missing 'Version' field.");
            return None;
        }
        Some(version) => version.to_string(),
    };
    let source = match pairs.get("Source") {
        None => "".to_string(),
        Some(source) => source.to_string(),
    };
    let tags = match pairs.get("Tags") {
        None => {
            println!("Metadata parsing error: Missing 'Tags' field.");
            "".to_string()
        }
        Some(tags) => tags.to_string(),
    };
    let beatmap_id = match pairs.get("BeatmapID") {
        None => prompt_i64(file_name.as_str(), "beatmapID", prompt_missing_value)?,
        Some(beatmap_id_str) => match beatmap_id_str.parse::<i64>() {
            Ok(id) => id,
            Err(err) => {
                println!(
                    "Metadata parsing error: 'BeatmapID'={} is not a valid i64: {}",
                    beatmap_id_str, err
                );
                return None;
            }
        },
    };
    let beatmapset_id = match pairs.get("BeatmapSetID") {
        None => prompt_i64(file_name.as_str(), "beatmapSetID", prompt_missing_value)?,
        Some(beatmapset_id_str) => match beatmapset_id_str.parse::<i64>() {
            Ok(id) => id,
            Err(err) => {
                println!(
                    "Metadata parsing error: 'BeatmapSetID'={} is not a valid i64: {}",
                    beatmapset_id_str, err
                );
                return None;
            }
        },
    };
    return Some(MetadataSection {
        title,
        title_unicode,
        artist,
        artist_unicode,
        creator,
        version,
        source,
        tags,
        beatmap_id,
        beatmapset_id,
    });
}
