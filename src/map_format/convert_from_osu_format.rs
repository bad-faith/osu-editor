use std::collections::HashSet;

use crate::{
    dotosu::{osu_file::OsuFile, sections::metadata::MetadataSection},
    map_format::{beatmap::Beatmap, beatmapset::Beatmapset},
};

pub fn convert_osu_beatmapset_to_internal(
    osu_files: &Vec<OsuFile>,
) -> Option<(Beatmapset, Vec<Beatmap>)> {
    if !prevalidate_osu_files(&osu_files) {
        return None;
    }
    let first_map = &osu_files[0];
    let beatmaps = osu_files
        .iter()
        .map(Beatmap::from_osu_format)
        .collect::<Option<Vec<_>>>()?;

    return Some((
        Beatmapset {
            audio_filename: first_map.general.audio_filename.clone(),
            audio_lead_in: first_map.general.audio_lead_in,
            preview_time: first_map.general.preview_time,
            id: first_map.metadata.beatmapset_id,
            creator: first_map.metadata.creator.clone(),
            title: first_map.metadata.title.clone(),
            title_unicode: first_map.metadata.title_unicode.clone(),
            artist: first_map.metadata.artist.clone(),
            artist_unicode: first_map.metadata.artist_unicode.clone(),
            source: first_map.metadata.source.clone(),
            tags: first_map.metadata.tags.clone(),
        },
        beatmaps,
    ));
}

#[derive(PartialEq)]
struct MatchingMetadata {
    title: String,
    title_unicode: String,
    artist: String,
    artist_unicode: String,
    creator: String,
    source: String,
    tags: String,
}

impl From<&MetadataSection> for MatchingMetadata {
    fn from(metadata: &MetadataSection) -> Self {
        MatchingMetadata {
            title: metadata.title.clone(),
            title_unicode: metadata.title_unicode.clone(),
            artist: metadata.artist.clone(),
            artist_unicode: metadata.artist_unicode.clone(),
            creator: metadata.creator.clone(),
            source: metadata.source.clone(),
            tags: metadata.tags.clone(),
        }
    }
}

fn prevalidate_osu_files(osu_files: &Vec<OsuFile>) -> bool {
    if osu_files.is_empty() {
        println!("No .osu files found.");
        return false;
    }
    let version_0 = MatchingMetadata::from(&osu_files[0].metadata);
    let mut versions: HashSet<String> = HashSet::new();
    for osu_file in osu_files {
        let version_n = MatchingMetadata::from(&osu_file.metadata);
        if version_n != version_0 {
            println!(
                "Inconsistent metadata found between .osu files: '{}' vs '{}'",
                osu_file.metadata.version, osu_files[0].metadata.version
            );
            return false;
        }
        match versions.insert(osu_file.metadata.version.clone()) {
            false => {
                println!(
                    "Duplicate diffs '{}' found in .osu files.",
                    osu_file.metadata.version
                );
                return false;
            }
            true => {}
        }
    }
    return true;
}
