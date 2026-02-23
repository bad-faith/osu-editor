use std::sync::Arc;

#[derive(Clone)]
pub struct RenderedAudio {
    pub sample_rate: u32,
    pub channels: usize,
    /// Interleaved f32 samples at `sample_rate`.
    pub data: Arc<Vec<f32>>,
}

impl RenderedAudio {
    pub fn frames_len(&self) -> usize {
        self.data.len() / self.channels
    }
}
