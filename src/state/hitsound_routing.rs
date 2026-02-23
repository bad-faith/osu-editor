use crate::map_format::{objects::HitsoundInfo, timing::SampleSet};

use super::hitsound_sampleset_indices::HitsoundSamplesetIndices;

#[derive(Clone)]
pub struct HitsoundRouting {
    pub normal: HitsoundSamplesetIndices,
    pub soft: HitsoundSamplesetIndices,
    pub drum: HitsoundSamplesetIndices,
}

impl HitsoundRouting {
    pub fn resolve_sampleset(&self, sample_set: &SampleSet) -> &HitsoundSamplesetIndices {
        match sample_set {
            SampleSet::Normal => &self.normal,
            SampleSet::Soft => &self.soft,
            SampleSet::Drum => &self.drum,
        }
    }

    pub fn resolve_audio_events(
        &self,
        hitsound_info: &HitsoundInfo,
        position_x: f64,
    ) -> Vec<(usize, f64, f64)> {
        let hit_sampleset = self.resolve_sampleset(&hitsound_info.hit_sampleset);
        let addition_sampleset = self.resolve_sampleset(&hitsound_info.additions_sampleset);

        let mut events = vec![(hit_sampleset.hitnormal, hitsound_info.volume, position_x)];
        if hitsound_info.play_whistle {
            events.push((
                addition_sampleset.hitwhistle,
                hitsound_info.volume,
                position_x,
            ));
        }
        if hitsound_info.play_finish {
            events.push((
                addition_sampleset.hitfinish,
                hitsound_info.volume,
                position_x,
            ));
        }
        if hitsound_info.play_clap {
            events.push((addition_sampleset.hitclap, hitsound_info.volume, position_x));
        }
        events
    }
}
