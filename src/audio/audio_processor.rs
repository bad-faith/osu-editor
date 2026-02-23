use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

use soundtouch::{Setting, SoundTouch};

use crate::audio::{decode::decode_audio_from_bytes, sample::RenderedAudio};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct VariantKey {
    speed_bits: u64,
    fix_pitch: bool,
}

impl VariantKey {
    fn new(speed: f64, fix_pitch: bool) -> Self {
        Self {
            speed_bits: speed.to_bits(),
            fix_pitch,
        }
    }
}

pub struct AudioProcessor {
    base: Option<RenderedAudio>,
    cache: HashMap<VariantKey, Option<RenderedAudio>>,
    cache_dir: Option<PathBuf>,
}

impl AudioProcessor {
    pub fn new() -> Self {
        Self {
            base: None,
            cache: HashMap::new(),
            cache_dir: None,
        }
    }

    pub fn set_cache_dir(&mut self, dir: Option<PathBuf>) {
        self.cache_dir = dir;
        self.cache.clear();
    }

    pub fn set_base(&mut self, base: RenderedAudio) {
        self.base = Some(base);
        self.cache.clear();
    }

    pub fn base(&self) -> Option<&RenderedAudio> {
        self.base.as_ref()
    }

    pub fn clear(&mut self) {
        self.base = None;
        self.cache.clear();
    }

    pub fn get_or_render(
        &mut self,
        speed: f64,
        fix_pitch: bool,
        expected_sr: u32,
        expected_ch: usize,
    ) -> Option<RenderedAudio> {
        let speed = normalize_speed(speed);
        let key = VariantKey::new(speed, fix_pitch);
        if let Some(hit) = self.cache.get(&key) {
            return hit.clone();
        }

        if let Some(cached) = self.load_cached_variant(&key, expected_sr, expected_ch) {
            self.cache.insert(key, Some(cached.clone()));
            return Some(cached);
        }

        let Some(base) = self.base.as_ref() else {
            return None;
        };

        let rendered = render_from_base(base, speed, fix_pitch);
        if let Some(rendered) = rendered.as_ref() {
            self.store_cached_variant(&key, rendered);
        }
        self.cache.insert(key, rendered.clone());
        return rendered;
    }

    pub fn get_cached_only(
        &mut self,
        speed: f64,
        fix_pitch: bool,
        expected_sr: u32,
        expected_ch: usize,
    ) -> Option<RenderedAudio> {
        let speed = normalize_speed(speed);
        let key = VariantKey::new(speed, fix_pitch);
        if let Some(hit) = self.cache.get(&key) {
            return hit.clone();
        }
        if let Some(cached) = self.load_cached_variant(&key, expected_sr, expected_ch) {
            self.cache.insert(key, Some(cached.clone()));
            return Some(cached);
        }
        None
    }

    fn cache_path_for(&self, key: &VariantKey) -> Option<PathBuf> {
        let dir = self.cache_dir.as_ref()?;
        let filename = format!(
            "music_speed{}_fix{}.bin",
            key.speed_bits,
            if key.fix_pitch { 1 } else { 0 }
        );
        Some(dir.join(filename))
    }

    fn load_cached_variant(
        &self,
        key: &VariantKey,
        expected_sr: u32,
        expected_ch: usize,
    ) -> Option<RenderedAudio> {
        let path = self.cache_path_for(key)?;
        let data = fs::read(&path).ok()?;
        let decoded = decode_cached_audio(&data)?;
        if decoded.sample_rate != expected_sr || decoded.channels != expected_ch {
            return None;
        }
        Some(decoded)
    }

    fn store_cached_variant(&self, key: &VariantKey, audio: &RenderedAudio) {
        let Some(path) = self.cache_path_for(key) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let bytes = encode_cached_audio(audio);
        let _ = fs::write(path, bytes);
    }
}

const CACHE_MAGIC: [u8; 4] = *b"OEAU";
const CACHE_VERSION: u32 = 1;

fn encode_cached_audio(audio: &RenderedAudio) -> Vec<u8> {
    let samples_len = audio.data.len() as u64;
    let mut out = Vec::with_capacity(4 + 4 + 4 + 4 + 8 + audio.data.len() * 4);
    out.extend_from_slice(&CACHE_MAGIC);
    out.extend_from_slice(&CACHE_VERSION.to_le_bytes());
    out.extend_from_slice(&audio.sample_rate.to_le_bytes());
    out.extend_from_slice(&(audio.channels as u32).to_le_bytes());
    out.extend_from_slice(&samples_len.to_le_bytes());
    for s in audio.data.iter() {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

fn decode_cached_audio(bytes: &[u8]) -> Option<RenderedAudio> {
    if bytes.len() < 4 + 4 + 4 + 4 + 8 {
        return None;
    }
    if bytes[0..4] != CACHE_MAGIC {
        return None;
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
    if version != CACHE_VERSION {
        return None;
    }
    let sample_rate = u32::from_le_bytes(bytes[8..12].try_into().ok()?);
    let channels = u32::from_le_bytes(bytes[12..16].try_into().ok()?) as usize;
    let samples_len = u64::from_le_bytes(bytes[16..24].try_into().ok()?) as usize;
    let expected_bytes = 4 + 4 + 4 + 4 + 8 + samples_len * 4;
    if bytes.len() < expected_bytes || channels == 0 {
        return None;
    }
    let mut data = Vec::with_capacity(samples_len);
    let mut offset = 24;
    for _ in 0..samples_len {
        let f = f32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?);
        data.push(f);
        offset += 4;
    }
    Some(RenderedAudio {
        sample_rate,
        channels,
        data: Arc::new(data),
    })
}

pub fn render_music(
    bytes: Vec<u8>,
    target_sr: u32,
    target_channels: usize,
    speed: f64,
    fix_pitch: bool,
    filename: String,
    hint_ext: Option<&str>,
) -> Option<RenderedAudio> {
    let t0 = std::time::Instant::now();
    log!(
        "[audio] render_music start (target_sr={}, target_ch={}, speed={}, fix_pitch={}, filename={}, hint_ext={:?})",
        target_sr,
        target_channels,
        speed,
        fix_pitch,
        filename,
        hint_ext
    );
    let decoded = match decode_audio_from_bytes(bytes, hint_ext) {
        Some(d) => d,
        None => {
            println!("[audio] render_music: failed to decode audio from bytes: {}", filename);
            return None;
        }
    };

    let rendered = match render_decoded(decoded, target_sr, target_channels, speed, fix_pitch) {
        Some(r) => r,
        None => {
            println!("[audio] render_music: failed to render decoded audio: {}", filename);
            return None;
        }
    };
    log!(
        "[audio] render_music ok ({:.2}s)",
        t0.elapsed().as_secs_f64()
    );
    return Some(rendered);
}

fn normalize_speed(speed: f64) -> f64 {
    let speed = if speed.is_finite() { speed } else { 1.0 };
    return speed.clamp(0.01, 100.0);
}

fn render_from_base(base: &RenderedAudio, speed: f64, fix_pitch: bool) -> Option<RenderedAudio> {
    if base.channels == 0 {
        println!("[audio] render_from_base: invalid channels=0");
        return None;
    }

    let speed = normalize_speed(speed);

    if (speed - 1.0).abs() <= 1e-9 {
        return Some(base.clone());
    }

    let frames_in = base.frames_len();
    if frames_in == 0 {
        println!("[audio] render_from_base: no frames");
        return None;
    }

    let out = if fix_pitch {
        // Tempo change without pitch shift.
        match time_stretch_interleaved_soundtouch(
            base.data.as_slice(),
            base.channels,
            base.sample_rate,
            speed,
        ) {
            Some(out) => out,
            None => {
                println!("[audio] render_from_base: time_stretch_interleaved_soundtouch failed");
                return None;
            }
        }
    } else {
        // Classic resample-speedup/down (pitch changes with speed).
        match resample_interleaved_linear(base.data.as_slice(), base.channels, 1.0 / speed) {
            Some(out) => out,
            None => {
                println!("[audio] render_from_base: resample_interleaved_linear failed");
                return None;
            }
        }
    };

    return Some(RenderedAudio {
        sample_rate: base.sample_rate,
        channels: base.channels,
        data: Arc::new(out),
    });
}

fn render_decoded(
    decoded: crate::audio::decode::DecodedAudio,
    target_sr: u32,
    target_channels: usize,
    speed: f64,
    fix_pitch: bool,
) -> Option<RenderedAudio> {
    let t0 = std::time::Instant::now();
    let src_sr = decoded.sample_rate;
    let src_ch = decoded.channels;

    // Convert to target channel count (2 preferred).
    let mut planar: Vec<Vec<f32>> = Vec::with_capacity(target_channels);
    match (src_ch, target_channels) {
        (1, 1) => planar = decoded.samples,
        (1, 2) => {
            planar.push(decoded.samples[0].clone());
            planar.push(decoded.samples[0].clone());
        }
        (2, 1) => {
            let mut mono = Vec::with_capacity(decoded.samples[0].len());
            for i in 0..decoded.samples[0].len() {
                mono.push((decoded.samples[0][i] + decoded.samples[1][i]) * 0.5);
            }
            planar.push(mono);
        }
        (2, 2) => planar = decoded.samples,
        _ => {
            // Fallback: take first channel(s).
            for c in 0..target_channels {
                planar.push(decoded.samples[c.min(src_ch - 1)].clone());
            }
        }
    }

    let speed = normalize_speed(speed);

    let (planar, ratio) = if fix_pitch {
        // Tempo change without pitch shift via time-stretch.
        // SoundTouch works on interleaved samples and supports multichannel processing.
        let stretched = if (speed - 1.0).abs() <= 1e-9 {
            planar
        } else {
            let interleaved = match interleave_planar(&planar, target_channels) {
                Some(v) => v,
                None => {
                    println!("[audio] render_decoded: interleave_planar failed");
                    return None;
                }
            };
            let stretched_interleaved = match time_stretch_interleaved_soundtouch(
                &interleaved,
                target_channels,
                src_sr,
                speed,
            ) {
                Some(v) => v,
                None => {
                    println!("[audio] render_decoded: time_stretch_interleaved_soundtouch failed");
                    return None;
                }
            };
            match deinterleave_to_planar(&stretched_interleaved, target_channels) {
                Some(v) => v,
                None => {
                    println!("[audio] render_decoded: deinterleave_to_planar failed");
                    return None;
                }
            }
        };

        // Only sample-rate convert (pitch preserved).
        let ratio = (target_sr as f64) / (src_sr as f64).max(1.0);
        (stretched, ratio)
    } else {
        // Classic speed-up/down by resampling (pitch changes with speed).
        let effective_src_sr = (src_sr as f64 * speed).max(1.0);
        let ratio = (target_sr as f64) / effective_src_sr;
        (planar, ratio)
    };

    let ratio_is_identity = (ratio - 1.0).abs() <= 1e-9;
    if ratio_is_identity {
        log!(
            "[audio] resample bypass (src_sr={}, target_sr={}, speed={}, ratio={:.6})",
            src_sr,
            target_sr,
            speed,
            ratio
        );
    } else {
        log!(
            "[audio] resample start (src_sr={}, src_ch={}, frames_in={}, ratio={:.6})",
            src_sr,
            src_ch,
            planar.get(0).map(|v| v.len()).unwrap_or(0),
            ratio
        );
    }

    let resampled = if ratio_is_identity {
        planar
    } else {
        let out = match resample_planar(&planar, ratio) {
            Some(v) => v,
            None => {
                println!("[audio] render_decoded: resample_planar failed");
                return None;
            }
        };
        log!(
            "[audio] resample ok (frames_out={}, {:.2}s)",
            out.get(0).map(|v| v.len()).unwrap_or(0),
            t0.elapsed().as_secs_f64()
        );
        out
    };

    // Interleave.
    let frames = resampled[0].len();
    let mut interleaved = Vec::with_capacity(frames * target_channels);
    for i in 0..frames {
        for c in 0..target_channels {
            interleaved.push(resampled[c][i]);
        }
    }

    return Some(RenderedAudio {
        sample_rate: target_sr,
        channels: target_channels,
        data: Arc::new(interleaved),
    });
}

fn resample_interleaved_linear(input: &[f32], channels: usize, ratio: f64) -> Option<Vec<f32>> {
    if channels == 0 {
        println!("[audio] resample_interleaved_linear: channels=0");
        return None;
    }
    if !ratio.is_finite() || ratio <= 0.0 {
        println!("[audio] resample_interleaved_linear: invalid resample ratio: {ratio}");
        return None;
    }
    if input.len() % channels != 0 {
        println!(
            "[audio] resample_interleaved_linear: interleaved buffer not divisible by channels"
        );
        return None;
    }

    let frames_in = input.len() / channels;
    if frames_in == 0 {
        println!("[audio] resample_interleaved_linear: no frames");
        return None;
    }

    let frames_out = ((frames_in as f64) * ratio).ceil().max(1.0) as usize;
    let inv_ratio = 1.0 / ratio;
    let mut out = vec![0.0f32; frames_out * channels];

    for i in 0..frames_out {
        let src_pos = (i as f64) * inv_ratio;
        let idx0 = (src_pos.floor() as usize).min(frames_in - 1);
        let idx1 = (idx0 + 1).min(frames_in - 1);
        let frac = (src_pos - (idx0 as f64)) as f32;

        let a0 = idx0 * channels;
        let a1 = idx1 * channels;
        let dst = i * channels;
        for c in 0..channels {
            let s0 = input[a0 + c];
            let s1 = input[a1 + c];
            out[dst + c] = s0 + (s1 - s0) * frac;
        }
    }

    return Some(out);
}

fn time_stretch_interleaved_soundtouch(
    input: &[f32],
    channels: usize,
    sample_rate: u32,
    speed: f64,
) -> Option<Vec<f32>> {
    if channels == 0 {
        println!("[audio] time_stretch_interleaved_soundtouch: channels=0");
        return None;
    }
    if input.len() % channels != 0 {
        println!(
            "[audio] time_stretch_interleaved_soundtouch: interleaved buffer not divisible by channels"
        );
        return None;
    }
    if !speed.is_finite() || speed <= 0.0 {
        println!("[audio] time_stretch_interleaved_soundtouch: invalid speed: {speed}");
        return None;
    }

    let frames_in = input.len() / channels;
    if frames_in == 0 {
        println!("[audio] time_stretch_interleaved_soundtouch: no frames");
        return None;
    }

    let speed = normalize_speed(speed);

    let mut st = SoundTouch::new();
    st.set_channels(channels as u32)
        .set_sample_rate(sample_rate)
        .set_tempo(speed)
        // Better quality; quickseek trades quality for CPU.
        .set_setting(Setting::UseQuickseek, 0);

    // Heuristic tuning to reduce reverby artifacts at very slow speeds.
    let (seq_ms, seek_ms, overlap_ms) = if speed < 0.5 {
        (90, 35, 14)
    } else if speed < 0.75 {
        (70, 25, 12)
    } else {
        (40, 15, 8)
    };
    let _ = seek_ms;
    st.set_setting(Setting::SequenceMs, seq_ms)
        .set_setting(Setting::OverlapMs, overlap_ms);

    let mut out = st.generate_audio(input);

    // Enforce exact duration to prevent editor clock drift.
    let expected_frames = ((frames_in as f64) / speed).round().max(1.0) as usize;
    let expected_samples = expected_frames * channels;
    if out.len() > expected_samples {
        out.truncate(expected_samples);
    } else if out.len() < expected_samples {
        out.resize(expected_samples, 0.0);
    }
    return Some(out);
}

fn interleave_planar(input: &[Vec<f32>], channels: usize) -> Option<Vec<f32>> {
    if input.is_empty() {
        println!("[audio] interleave_planar: no channels");
        return None;
    }
    if input.len() != channels {
        println!("[audio] interleave_planar: channel count mismatch");
        return None;
    }

    let frames = input[0].len();
    for c in 1..channels {
        if input[c].len() != frames {
            println!("[audio] interleave_planar: channel length mismatch");
            return None;
        }
    }

    let mut out = Vec::with_capacity(frames * channels);
    for i in 0..frames {
        for c in 0..channels {
            out.push(input[c][i]);
        }
    }
    return Some(out);
}

fn deinterleave_to_planar(input: &[f32], channels: usize) -> Option<Vec<Vec<f32>>> {
    if channels == 0 {
        println!("[audio] deinterleave_to_planar: channels=0");
        return None;
    }
    if input.len() % channels != 0 {
        println!("[audio] deinterleave_to_planar: interleaved buffer not divisible by channels");
        return None;
    }

    let frames = input.len() / channels;
    let mut planar: Vec<Vec<f32>> = (0..channels).map(|_| Vec::with_capacity(frames)).collect();
    for i in 0..frames {
        for c in 0..channels {
            planar[c].push(input[i * channels + c]);
        }
    }
    return Some(planar);
}

fn resample_planar(input: &[Vec<f32>], ratio: f64) -> Option<Vec<Vec<f32>>> {
    if input.is_empty() {
        println!("[audio] resample_planar: no channels");
        return None;
    }

    if !ratio.is_finite() || ratio <= 0.0 {
        println!("[audio] resample_planar: invalid resample ratio: {ratio}");
        return None;
    }

    let channels = input.len();
    let frames_in = input[0].len();
    for c in 1..channels {
        if input[c].len() != frames_in {
            println!("[audio] resample_planar: channel length mismatch");
            return None;
        }
    }

    if frames_in == 0 {
        println!("[audio] resample_planar: no frames");
        return None;
    }

    // Fast linear resampling. Quality is lower than sinc but avoids multi-second stalls
    // from offline high-quality resampling.
    let frames_out = ((frames_in as f64) * ratio).ceil().max(1.0) as usize;
    let inv_ratio = 1.0 / ratio;

    let mut out: Vec<Vec<f32>> = Vec::with_capacity(channels);
    for c in 0..channels {
        let src = &input[c];
        let mut dst = Vec::with_capacity(frames_out);

        for i in 0..frames_out {
            let src_pos = (i as f64) * inv_ratio;
            let idx0 = src_pos.floor() as usize;
            let frac = (src_pos - (idx0 as f64)) as f32;

            let idx0 = idx0.min(frames_in - 1);
            let idx1 = (idx0 + 1).min(frames_in - 1);

            let s0 = src[idx0];
            let s1 = src[idx1];
            dst.push(s0 + (s1 - s0) * frac);
        }

        out.push(dst);
    }

    return Some(out);
}
