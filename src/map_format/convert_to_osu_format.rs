use crate::{
    dotosu::{
        osu_file::OsuFile,
        sections::{general::GeneralSection, metadata::MetadataSection},
    },
    map_format::{beatmap::Beatmap, beatmapset::Beatmapset},
};

pub fn convert_internal_to_osu_format(beatmapset: Beatmapset, beatmap: Beatmap) -> OsuFile {
    OsuFile {
        general: GeneralSection {
            audio_filename: beatmapset.audio_filename,
            audio_lead_in: beatmapset.audio_lead_in,
            preview_time: beatmapset.preview_time,
            countdown: beatmap.general.countdown,
            sample_set: beatmap.general.sample_set,
            stack_leniency: get_stack_leniency(
                beatmap.diff_settings.preempt_period,
                beatmap.diff_settings.stacking_period,
            ),
            mode: beatmap.general.mode,
            letterbox_in_breaks: beatmap.general.letterbox_in_breaks,
            epilepsy_warning: beatmap.general.epilepsy_warning,
            widescreen_storyboard: beatmap.general.widescreen_storyboard,
        },
        metadata: MetadataSection {
            beatmapset_id: beatmapset.id,
            creator: beatmapset.creator,
            title: beatmapset.title,
            title_unicode: beatmapset.title_unicode,
            artist: beatmapset.artist,
            artist_unicode: beatmapset.artist_unicode,
            source: beatmapset.source,
            tags: beatmapset.tags,
            version: beatmap.version,
            beatmap_id: beatmap.id,
        },
        difficulty: beatmap.diff_settings.to_osu_format(),
        events: beatmap.events.to_osu_format(),
        timing: beatmap.timing.to_osu_format(),
        colours: beatmap.colors.to_osu_format(),
        objects: beatmap.objects.to_osu_format(),
    }
}

fn get_stack_leniency(preempt_period: f64, stacking_period: f64) -> f64 {
    (stacking_period / preempt_period).clamp(0.0, 1.0)
}
