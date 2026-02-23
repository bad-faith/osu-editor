use std::sync::Arc;

use crate::audio::AudioEngine;

use super::hitsound_routing::HitsoundRouting;

#[derive(Clone)]
pub struct HitsoundThreadConfig {
    pub audio: Arc<AudioEngine>,
    pub routing: HitsoundRouting,
}
