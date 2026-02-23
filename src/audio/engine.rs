use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, anyhow};
use cpal::{traits::DeviceTrait, traits::HostTrait, traits::StreamTrait};
use crossbeam_channel::{Receiver, Sender};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Producer, Split},
};

use crate::audio::{
    audio_processor::{AudioProcessor, render_music},
    sample::RenderedAudio,
};

#[derive(Clone)]
pub struct AudioEngineConfig {
    /// How much audio to keep queued, in milliseconds.
    pub queue_ms: u32,
    /// Preferred callback buffer size (frames). Will clamp to supported range.
    pub preferred_buffer_frames: u32,
    /// If true, preserve pitch when changing playback rate.
    /// If false, pitch changes with playback rate (resample-speedup behavior).
    pub fix_pitch: bool,
}

impl Default for AudioEngineConfig {
    fn default() -> Self {
        Self {
            queue_ms: 60,
            preferred_buffer_frames: 128,
            fix_pitch: false,
        }
    }
}

enum Command {
    LoadMusic {
        bytes: Vec<u8>,
        map_dir_name: String,
        hint_ext: Option<String>,
    },
    SetHitsoundSample {
        bytes: Vec<u8>,
        index: usize,
        filename: String,
        hint_ext: Option<String>,
    },
    RemoveAllHitsoundSamples,
    RemoveAllHitsounds,
    SetHitsoundVolume(f64),
    SetSpacialAudio(f64),
    Play,
    Pause,
    Stop,
    SetSpeed(f64),
    SetVolume(f64),
    SetMapTimeOffset(f64),
    SetHitsoundsOffset(f64),
    SeekMapTime(f64),
    SetFixPitch(bool),
}

enum HitsoundEditCommand {
    Add {
        map_time_ms: f64,
        index: usize,
        volume: f64,
        position_x: f64,
    },
    Remove {
        map_time_ms: f64,
        index: usize,
        volume: f64,
        position_x: f64,
    },
}

struct Shared {
    start_instant: Instant,
    sample_rate: u32,
    channels: usize,

    // True when the engine intends to advance time and consume audio.
    playing: AtomicBool,

    // True once the CPAL output callback has run at least once.
    callback_started: AtomicBool,

    // Absolute output frame index that the device has just finished writing (exclusive).
    played_frames_abs: AtomicU64,

    // Last observed played frame count and timestamp from the audio callback.
    last_callback_played_frames_abs: AtomicU64,
    last_callback_time_ns: AtomicU64,

    // Absolute output frame index that corresponds to map time = 0.
    origin_frame_abs: AtomicU64,

    // Version counter for origin/speed updates (even = stable, odd = updating).
    time_params_version: AtomicU64,

    // Map time to report while paused (ms).
    paused_map_time_ms_bits: AtomicU64,

    // Total rendered music length in output frames (at `sample_rate`).
    music_frames_len: AtomicU64,

    // Map time offset (ms). Typically -AudioLeadIn.
    map_time_offset_ms_bits: AtomicU64,

    // Hitsound timing offset (ms). Applied on scheduling.
    hitsounds_offset_ms_bits: AtomicU64,

    // f32 bits.
    speed_bits: AtomicU32,

    // f32 bits.
    volume_bits: AtomicU32,
    hitsound_volume_bits: AtomicU32,
    spacial_audio_bits: AtomicU32,

    flush_requested: AtomicBool,
    loading: AtomicBool,

    underruns: AtomicU64,
}

impl Shared {
    fn now_ns(&self) -> u64 {
        self.start_instant
            .elapsed()
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64
    }

    fn speed(&self) -> f64 {
        return f32::from_bits(self.speed_bits.load(Ordering::Relaxed)) as f64;
    }

    fn volume(&self) -> f32 {
        return f32::from_bits(self.volume_bits.load(Ordering::Relaxed));
    }

    fn hitsound_volume(&self) -> f32 {
        return f32::from_bits(self.hitsound_volume_bits.load(Ordering::Relaxed));
    }

    fn spacial_audio(&self) -> f32 {
        return f32::from_bits(self.spacial_audio_bits.load(Ordering::Relaxed));
    }

    fn map_time_offset_ms(&self) -> f64 {
        return f64::from_bits(self.map_time_offset_ms_bits.load(Ordering::Relaxed));
    }

    fn paused_map_time_ms(&self) -> f64 {
        return f64::from_bits(self.paused_map_time_ms_bits.load(Ordering::Relaxed));
    }

    fn hitsounds_offset_ms(&self) -> f64 {
        return f64::from_bits(self.hitsounds_offset_ms_bits.load(Ordering::Relaxed));
    }

    fn current_map_time_ms(&self) -> f64 {
        if !self.playing.load(Ordering::Acquire) {
            return self.paused_map_time_ms();
        }
        loop {
            let v1 = self.time_params_version.load(Ordering::Acquire);
            if v1 & 1 == 1 {
                continue;
            }

            let played = self.played_frames_abs.load(Ordering::Acquire);
            let mut played_interp = played;
            if self.playing.load(Ordering::Acquire) && self.callback_started.load(Ordering::Acquire)
            {
                let last_cb_frames = self.last_callback_played_frames_abs.load(Ordering::Acquire);
                let last_cb_ns = self.last_callback_time_ns.load(Ordering::Acquire);
                let now_ns = self.now_ns();
                if now_ns >= last_cb_ns {
                    let dt_ns = now_ns - last_cb_ns;
                    let sr = self.sample_rate as u64;
                    let add_frames = dt_ns.saturating_mul(sr) / 1_000_000_000u64;
                    played_interp = played_interp.max(last_cb_frames.saturating_add(add_frames));
                }
            }

            let origin = self.origin_frame_abs.load(Ordering::Acquire);
            let speed = f32::from_bits(self.speed_bits.load(Ordering::Acquire)) as f64;
            let offset = self.map_time_offset_ms();

            let v2 = self.time_params_version.load(Ordering::Acquire);
            if v1 != v2 {
                continue;
            }

            let sr = self.sample_rate as f64;
            let rel = played_interp.saturating_sub(origin) as f64;
            return (rel / sr) * 1000.0 * speed + offset;
        }
    }

    fn is_loading(&self) -> bool {
        self.loading.load(Ordering::Acquire)
    }
}

pub struct AudioEngine {
    tx: Sender<Command>,
    hitsound_edits_tx: Sender<HitsoundEditCommand>,
    shared: Arc<Shared>,
}

impl AudioEngine {
    pub fn new(cfg: AudioEngineConfig) -> anyhow::Result<Self> {
        let (tx, rx) = crossbeam_channel::unbounded();
        let (hitsound_edits_tx, hitsound_edits_rx) = crossbeam_channel::unbounded();

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("no default output device"))?;

        log!(
            "[audio] output device: {}",
            match device.description() {
                Ok(desc) => format!("{desc:?}"),
                Err(_) => "<unknown>".to_string(),
            }
        );

        // Choose a low-latency-ish config. On Windows CPAL uses WASAPI; smaller buffers
        // generally reduce latency (at the risk of underruns).
        let mut supported = device
            .supported_output_configs()
            .context("supported_output_configs")?;

        let score_format = |sf: cpal::SampleFormat| match sf {
            cpal::SampleFormat::F32 => 0,
            cpal::SampleFormat::I16 => 1,
            cpal::SampleFormat::U16 => 2,
            _ => 3,
        };

        let preferred_sample_rate: u32 = 44_100;
        let supports_preferred_sr = |range: &cpal::SupportedStreamConfigRange| {
            range.min_sample_rate() <= preferred_sample_rate
                && preferred_sample_rate <= range.max_sample_rate()
        };

        // Lower is better.
        let score_range = |range: &cpal::SupportedStreamConfigRange| {
            let sr_score: u32 = if supports_preferred_sr(range) { 0 } else { 1 };
            let ch_score: u32 = if range.channels() == 2 { 0 } else { 1 };
            let fmt_score: u32 = score_format(range.sample_format());
            (sr_score, ch_score, fmt_score)
        };

        let mut best: Option<cpal::SupportedStreamConfigRange> = None;
        while let Some(range) = supported.next() {
            let is_better = best
                .as_ref()
                .map(|b| score_range(&range) < score_range(b))
                .unwrap_or(true);

            if is_better {
                best = Some(range);
            }
        }

        let best = best.ok_or_else(|| anyhow!("no supported output configs"))?;

        // Most osu! audio is 44100Hz. Prefer it when supported to avoid expensive
        // offline resampling (44.1k -> 48k).
        let supported_config = if supports_preferred_sr(&best) {
            best.with_sample_rate(preferred_sample_rate)
        } else {
            best.with_max_sample_rate()
        };
        let sample_format = supported_config.sample_format();

        let mut config: cpal::StreamConfig = supported_config.config();

        // Clamp requested buffer size if the backend supports it.
        if let cpal::SupportedBufferSize::Range { min, max } = best.buffer_size() {
            let preferred = cfg.preferred_buffer_frames;
            let fixed = preferred.clamp(*min, *max);
            config.buffer_size = cpal::BufferSize::Fixed(fixed);
        }

        let sample_rate = config.sample_rate;
        let channels = config.channels as usize;

        let shared = Arc::new(Shared {
            start_instant: Instant::now(),
            sample_rate,
            channels,
            playing: AtomicBool::new(false),
            callback_started: AtomicBool::new(false),
            played_frames_abs: AtomicU64::new(0),
            last_callback_played_frames_abs: AtomicU64::new(0),
            last_callback_time_ns: AtomicU64::new(0),
            origin_frame_abs: AtomicU64::new(0),
            time_params_version: AtomicU64::new(0),
            paused_map_time_ms_bits: AtomicU64::new(0f64.to_bits()),
            music_frames_len: AtomicU64::new(0),
            map_time_offset_ms_bits: AtomicU64::new(0f64.to_bits()),
            hitsounds_offset_ms_bits: AtomicU64::new(0f64.to_bits()),
            speed_bits: AtomicU32::new((1.0f32).to_bits()),
            volume_bits: AtomicU32::new((1.0f32).to_bits()),
            hitsound_volume_bits: AtomicU32::new((1.0f32).to_bits()),
            spacial_audio_bits: AtomicU32::new((0.0f32).to_bits()),
            flush_requested: AtomicBool::new(false),
            loading: AtomicBool::new(false),
            underruns: AtomicU64::new(0),
        });

        std::thread::Builder::new()
            .name("audio-engine".to_string())
            .spawn({
                let device = device;
                let config = config;
                let sample_format = sample_format;
                let shared = Arc::clone(&shared);
                move || {
                    audio_thread_main(
                        device,
                        config,
                        sample_format,
                        shared,
                        rx,
                        hitsound_edits_rx,
                        cfg,
                    )
                }
            })
            .context("spawn audio thread")?;

        Ok(Self {
            tx,
            hitsound_edits_tx,
            shared,
        })
    }

    pub fn load_music(&self, bytes: Vec<u8>, map_dir_name: &str, filename: &str) {
        let filename = filename.trim().trim_matches('"');
        let hint_ext = std::path::Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());

        let _ = self.tx.send(Command::LoadMusic { bytes, map_dir_name: map_dir_name.to_string(), hint_ext });
        log!("[audio] sent load_music");
    }

    pub fn set_hitsound_sample(
        &self,
        bytes: Vec<u8>,
        index: usize,
        filename: String,
        hint_ext: Option<String>,
    ) -> Option<()> {
        if bytes.is_empty() {
            return None;
        }
        let res = self.tx.send(Command::SetHitsoundSample {
            bytes,
            index,
            filename,
            hint_ext,
        });
        if res.is_ok() {
            log!("[audio] sent set_hitsound_sample index={}", index);
            Some(())
        } else {
            None
        }
    }

    pub fn remove_all_hitsound_samples(&self) {
        let _ = self.tx.send(Command::RemoveAllHitsoundSamples);
        log!("[audio] sent remove_all_hitsound_samples");
    }

    pub fn remove_all_hitsounds(&self) {
        let _ = self.tx.send(Command::RemoveAllHitsounds);
        log!("[audio] sent remove_all_hitsounds");
    }

    pub fn add_hitsound(&self, map_time_ms: f64, index: usize, volume: f64, position_x: f64) {
        if !map_time_ms.is_finite() || !position_x.is_finite() {
            return;
        }
        let volume = volume.clamp(0.0, 1.0);
        let _ = self.hitsound_edits_tx.send(HitsoundEditCommand::Add {
            map_time_ms,
            index,
            volume,
            position_x,
        });
    }

    pub fn remove_hitsound(&self, map_time_ms: f64, index: usize, volume: f64, position_x: f64) {
        if !map_time_ms.is_finite() || !position_x.is_finite() {
            return;
        }
        let volume = volume.clamp(0.0, 1.0);
        let _ = self.hitsound_edits_tx.send(HitsoundEditCommand::Remove {
            map_time_ms,
            index,
            volume,
            position_x,
        });
    }

    pub fn play(&self) {
        let _ = self.tx.send(Command::Play);
        log!("[audio] sent play");
    }

    pub fn pause(&self) {
        let _ = self.tx.send(Command::Pause);
        log!("[audio] sent pause");
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Command::Stop);
        log!("[audio] sent stop");
    }

    pub fn set_speed(&self, speed: f64) {
        if !(0.1..=4.0).contains(&speed) {
            return;
        }
        let _ = self.tx.send(Command::SetSpeed(speed));
        log!("[audio] sent set_speed to {}", speed);
    }

    pub fn get_speed(&self) -> f64 {
        self.shared.speed()
    }

    pub fn get_volume(&self) -> f64 {
        self.shared.volume() as f64
    }

    pub fn get_hitsound_volume(&self) -> f64 {
        self.shared.hitsound_volume() as f64
    }

    pub fn set_volume(&self, volume: f64) {
        if !volume.is_finite() {
            return;
        }
        let volume = volume.clamp(0.0, 1.0);
        let _ = self.tx.send(Command::SetVolume(volume));
        log!("[audio] sent set_volume to {:.3}", volume);
    }

    pub fn set_map_time_offset_ms(&self, offset_ms: f64) {
        if !offset_ms.is_finite() {
            return;
        }
        let _ = self.tx.send(Command::SetMapTimeOffset(offset_ms));
        log!("[audio] sent set_map_time_offset_ms to {:.2}ms", offset_ms);
    }

    pub fn set_hitsounds_offset_ms(&self, offset_ms: f64) {
        if !offset_ms.is_finite() {
            return;
        }
        let _ = self.tx.send(Command::SetHitsoundsOffset(offset_ms));
        log!("[audio] sent set_hitsounds_offset_ms to {:.2}ms", offset_ms);
    }

    pub fn set_hitsound_volume(&self, volume: f64) {
        if !volume.is_finite() {
            return;
        }
        let volume = volume.clamp(0.0, 1.0);
        let _ = self.tx.send(Command::SetHitsoundVolume(volume));
        log!("[audio] sent set_hitsound_volume to {:.3}", volume);
    }

    pub fn set_spacial_audio(&self, spacial_audio: f64) {
        if !spacial_audio.is_finite() {
            return;
        }
        let spacial_audio = spacial_audio.clamp(0.0, 1.0);
        let _ = self.tx.send(Command::SetSpacialAudio(spacial_audio));
        log!("[audio] sent set_spacial_audio to {:.3}", spacial_audio);
    }

    pub fn set_fix_pitch(&self, fix_pitch: bool) {
        let _ = self.tx.send(Command::SetFixPitch(fix_pitch));
        log!("[audio] sent set_fix_pitch={}", fix_pitch);
    }

    pub fn seek_map_time_ms(&self, map_time_ms: f64) {
        if !map_time_ms.is_finite() {
            return;
        }
        let _ = self.tx.send(Command::SeekMapTime(map_time_ms));
        log!("[audio] sent seek_map_time_ms to {:.2}ms", map_time_ms);
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Acquire)
    }

    pub fn song_total_ms(&self) -> f64 {
        let frames = self.shared.music_frames_len.load(Ordering::Acquire);
        if frames == 0 {
            return 0.0;
        }
        let sr = self.shared.sample_rate as f64;
        let speed = self.shared.speed();
        (frames as f64 / sr) * 1000.0 * speed
    }

    pub fn current_time_ms(&self) -> f64 {
        self.shared.current_map_time_ms()
    }

    pub fn is_loading(&self) -> bool {
        self.shared.is_loading()
    }
}

struct Voice {
    audio: RenderedAudio,
    frame_pos: usize,
    gain: f64,
    position_x: f64,
    start_abs_frame: u64,
    sample_index: usize,
    map_time_ms: f64,
}

#[derive(Clone)]
struct HitsoundEvent {
    map_time_ms: f64,
    index: usize,
    volume: f64,
    position_x: f64,
}

struct EngineState {
    music_source_bytes: Option<Vec<u8>>,
    music_hint_ext: Option<String>,
    audio_processor: AudioProcessor,
    music: Option<RenderedAudio>,
    playing: bool,
    fix_pitch: bool,

    hitsound_samples: Vec<Option<RenderedAudio>>,
    hitsound_events: Vec<HitsoundEvent>,
    last_hitsound_map_time_ms: Option<f64>,

    voices: Vec<Voice>,

    scheduled: Vec<Voice>,

    cfg: AudioEngineConfig,
}

fn ensure_music_base(state: &mut EngineState, sr: u32, channels: usize) -> anyhow::Result<bool> {
    if state.audio_processor.base().is_some() {
        return Ok(true);
    }

    let Some(bytes) = state.music_source_bytes.clone() else {
        return Ok(false);
    };

    let hint_ext = state.music_hint_ext.clone();
    let base = match render_music(
        bytes,
        sr,
        channels,
        1.0,
        false,
        "song".to_string(),
        hint_ext.as_deref(),
    ) {
        Some(b) => b,
        None => {
            return Err(anyhow!("failed to decode/render music base"));
        }
    };
    state.audio_processor.set_base(base);
    Ok(true)
}

fn prime_music_queue(
    shared: &Arc<Shared>,
    state: &EngineState,
    prod: &mut impl Producer<Item = f32>,
    channels: usize,
    target_frames: usize,
) {
    let Some(music) = state.music.as_ref() else {
        return;
    };

    // Fill up to the target queue depth (or until we run out of music).
    while (prod.occupied_len() / channels) < target_frames {
        let occupied_frames = prod.occupied_len() / channels;

        // Absolute frame index of the first frame in this generated block.
        let played_abs = shared.played_frames_abs.load(Ordering::Acquire);
        let abs_cursor = played_abs + occupied_frames as u64;
        let origin_abs = shared.origin_frame_abs.load(Ordering::Acquire);

        let rel = abs_cursor.saturating_sub(origin_abs) as usize;
        let available = music.frames_len().saturating_sub(rel);
        if available == 0 {
            break;
        }

        let frames_to_gen = (target_frames - occupied_frames).min(1024).min(available);
        if frames_to_gen == 0 {
            break;
        }

        let start = rel * channels;
        let end = (rel + frames_to_gen) * channels;
        let slice = &music.data[start..end];

        let pushed = prod.push_slice(slice);
        if pushed < slice.len() {
            // Queue filled earlier than expected.
            break;
        }
    }
}

fn audio_thread_main(
    device: cpal::Device,
    config: cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    shared: Arc<Shared>,
    rx: Receiver<Command>,
    hitsound_edits_rx: Receiver<HitsoundEditCommand>,
    cfg: AudioEngineConfig,
) {
    let sr = shared.sample_rate;
    let channels = shared.channels;

    let queue_frames = ((sr as u64) * (cfg.queue_ms as u64) / 1000).max(256) as usize;

    log!(
        "[audio] stream config: sr={} ch={} fmt={:?} buffer={:?} queue_ms={} queue_frames={}",
        config.sample_rate,
        config.channels,
        sample_format,
        config.buffer_size,
        cfg.queue_ms,
        queue_frames
    );
    let rb = HeapRb::<f32>::new(queue_frames * channels);
    let (mut prod, cons) = rb.split();

    let mut state = EngineState {
        music_source_bytes: None,
        music_hint_ext: None,
        audio_processor: AudioProcessor::new(),
        music: None,
        playing: false,
        fix_pitch: cfg.fix_pitch,
        hitsound_samples: Vec::new(),
        hitsound_events: Vec::new(),
        last_hitsound_map_time_ms: None,
        voices: Vec::new(),
        scheduled: Vec::new(),
        cfg,
    };

    let stream = match build_stream(&device, &config, sample_format, &shared, cons) {
        Some(s) => s,
        None => {
            println!("Failed to build audio stream");
            return;
        }
    };

    // Start paused; we only start the callback once the editor actually plays.
    // This avoids the editor switching to the audio clock while we're still loading.
    if let Err(err) = stream.pause() {
        log!("Failed to pause audio stream: {err:?}");
    }

    loop {
        // Drain high-priority control commands first.
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                Command::LoadMusic { bytes, map_dir_name, hint_ext } => {
                    // New map: avoid reusing base from previous song.
                    state.audio_processor.clear();
                    state.music_source_bytes = Some(bytes.clone());
                    state.music_hint_ext = hint_ext.clone();
                    let cache_dir = Path::new("saves").join(&map_dir_name).join("cache");
                    state.audio_processor.set_cache_dir(Some(cache_dir));

                    // Try to load cached base (1.0x, original pitch) to avoid decoding when possible.
                    if let Some(cached_base) = state
                        .audio_processor
                        .get_cached_only(1.0, false, sr, channels)
                    {
                        state.audio_processor.set_base(cached_base);
                    }

                    let speed = shared.speed();
                    let mut rendered =
                        state
                            .audio_processor
                            .get_or_render(speed, state.fix_pitch, sr, channels);

                    if rendered.is_none() {
                        // Always render a 1.0x reference (original pitch) and derive playback-rate
                        // variants from it. This avoids accumulating artifacts and improves pitch stability.
                        match render_music(
                            bytes,
                            sr,
                            channels,
                            1.0,
                            false,
                            "song".to_string(),
                            hint_ext.as_deref(),
                        ) {
                            Some(base) => {
                                state.audio_processor.set_base(base);
                                rendered = state
                                    .audio_processor
                                    .get_or_render(speed, state.fix_pitch, sr, channels);
                            }
                            None => {
                                println!("Failed to decode/render music");
                                shared.music_frames_len.store(0, Ordering::Release);
                                state.audio_processor.clear();
                                state.music = None;
                            }
                        }
                    }

                    match rendered {
                        Some(rendered) => {
                            shared
                                .music_frames_len
                                .store(rendered.frames_len() as u64, Ordering::Release);
                            state.music = Some(rendered);
                        }
                        None => {
                            shared.music_frames_len.store(0, Ordering::Release);
                            state.music = None;
                        }
                    }

                    // Keep origin at current absolute frame.
                    let now = shared.played_frames_abs.load(Ordering::Acquire);
                    shared.origin_frame_abs.store(now, Ordering::Release);
                    state.hitsound_events.clear();
                    state.voices.clear();
                    state.scheduled.clear();
                    state.last_hitsound_map_time_ms = None;
                    shared.flush_requested.store(true, Ordering::Release);
                }
                Command::SetHitsoundSample {
                    bytes,
                    index,
                    filename,
                    hint_ext,
                } => {
                    if bytes.is_empty() {
                        continue;
                    }

                    let rendered = render_music(
                        bytes,
                        sr,
                        channels,
                        1.0,
                        false,
                        filename,
                        hint_ext.as_deref(),
                    );

                    if state.hitsound_samples.len() <= index {
                        state.hitsound_samples.resize(index + 1, None);
                    }

                    match rendered {
                        Some(sample) => {
                            state.hitsound_samples[index] = Some(sample);
                            log!("[audio] set hitsound sample index={}", index);
                        }
                        None => {
                            state.hitsound_samples[index] = None;
                            log!("[audio] failed to decode hitsound index={}", index);
                        }
                    }
                }
                Command::RemoveAllHitsoundSamples => {
                    state.hitsound_samples.clear();
                }
                Command::RemoveAllHitsounds => {
                    state.hitsound_events.clear();
                    state.scheduled.clear();
                    state.voices.clear();
                    state.last_hitsound_map_time_ms = None;
                }
                Command::Play => {
                    state.playing = true;
                    shared.playing.store(true, Ordering::Release);

                    // Prime the queue before starting the stream callback to avoid an immediate
                    // underrun (especially when fix_pitch rendering was slow).
                    let target_frames = ((sr as u64) * (state.cfg.queue_ms as u64) / 1000) as usize;
                    prime_music_queue(&shared, &state, &mut prod, channels, target_frames);

                    if let Err(err) = stream.play() {
                        log!("Failed to play audio stream: {err:?}");
                        state.playing = false;
                        shared.playing.store(false, Ordering::Release);
                    }
                    log!("[audio] cmd play (playing=true)");
                }
                Command::Pause => {
                    let t_ms = shared.current_map_time_ms();
                    shared
                        .paused_map_time_ms_bits
                        .store(t_ms.to_bits(), Ordering::Release);
                    state.playing = false;
                    shared.playing.store(false, Ordering::Release);
                    shared.flush_requested.store(true, Ordering::Release);
                    if let Err(err) = stream.pause() {
                        log!("Failed to pause audio stream: {err:?}");
                    }
                    log!("[audio] cmd pause (playing=false)");
                }
                Command::Stop => {
                    state.playing = false;
                    shared.playing.store(false, Ordering::Release);
                    shared
                        .paused_map_time_ms_bits
                        .store(0f64.to_bits(), Ordering::Release);
                    state.voices.clear();
                    state.scheduled.clear();
                    state.last_hitsound_map_time_ms = None;
                    shared.music_frames_len.store(0, Ordering::Release);
                    shared.flush_requested.store(true, Ordering::Release);
                    if let Err(err) = stream.pause() {
                        log!("Failed to pause audio stream: {err:?}");
                    }
                    log!("[audio] cmd stop (playing=false)");
                }
                Command::SetSpeed(new_speed) => {
                    if !(0.1..=4.0).contains(&new_speed) {
                        continue;
                    }

                    // Capture current map time based on actual played frames (no interpolation),
                    // so changing speed doesn't jump forward.
                    let played_abs = shared.played_frames_abs.load(Ordering::Acquire);
                    let origin_abs = shared.origin_frame_abs.load(Ordering::Acquire);
                    let offset_ms = shared.map_time_offset_ms();
                    let speed_old = shared.speed();
                    let sr_f = sr as f64;
                    let rel_frames = played_abs.saturating_sub(origin_abs) as f64;
                    let t_old_ms = (rel_frames / sr_f) * 1000.0 * speed_old + offset_ms;
                    shared
                        .paused_map_time_ms_bits
                        .store(t_old_ms.to_bits(), Ordering::Release);

                    let was_playing = state.playing;
                    if was_playing {
                        state.playing = false;
                        shared.playing.store(false, Ordering::Release);
                        if let Err(err) = stream.pause() {
                            log!("Failed to pause audio stream: {err:?}");
                        }
                        log!("[audio] cmd pause (playing=false)");
                    }

                    if state.fix_pitch {
                        shared.loading.store(true, Ordering::Release);
                    }

                    // Preserve the current reported map time across rate changes.
                    // We keep `played_frames_abs` as "last played sample" and adjust `origin_frame_abs`
                    // so the mapping remains continuous.
                    // Update speed + origin together to keep map time continuous.
                    shared.time_params_version.fetch_add(1, Ordering::AcqRel);

                    // origin_new = played_abs - ((t_old - offset)/speed_new) * sr
                    let rel_new_frames_f = ((t_old_ms - offset_ms) / (1000.0 * new_speed)) * sr_f;
                    let rel_new_frames = if rel_new_frames_f.is_finite() {
                        rel_new_frames_f.round().max(0.0) as u64
                    } else {
                        0
                    };
                    let origin_abs_new = played_abs.saturating_sub(rel_new_frames);
                    shared
                        .origin_frame_abs
                        .store(origin_abs_new, Ordering::Release);
                    shared
                        .speed_bits
                        .store((new_speed as f32).to_bits(), Ordering::Release);

                    let desired_played_abs = origin_abs_new.saturating_add(rel_new_frames);
                    shared
                        .played_frames_abs
                        .store(desired_played_abs, Ordering::Release);
                    shared
                        .last_callback_played_frames_abs
                        .store(desired_played_abs, Ordering::Release);
                    shared
                        .last_callback_time_ns
                        .store(shared.now_ns(), Ordering::Release);

                    shared.time_params_version.fetch_add(1, Ordering::Release);

                    // Derive from 1.0x base when available; fallback to decoding otherwise.
                    match ensure_music_base(&mut state, sr, channels) {
                        Ok(true) => {
                            match state
                                .audio_processor
                                .get_or_render(new_speed, state.fix_pitch, sr, channels)
                            {
                                Some(rendered) => {
                                    shared
                                        .music_frames_len
                                        .store(rendered.frames_len() as u64, Ordering::Release);
                                    state.music = Some(rendered);
                                }
                                None => {
                                    shared.music_frames_len.store(0, Ordering::Release);
                                    state.music = None;
                                }
                            }
                        }
                        Ok(false) => {}
                        Err(err) => {
                            log!("Failed to decode/render music base: {err:?}");
                        }
                    }

                    state.voices.clear();
                    state.scheduled.clear();
                    state.last_hitsound_map_time_ms = None;
                    shared.flush_requested.store(true, Ordering::Release);
                    if state.fix_pitch {
                        shared.loading.store(false, Ordering::Release);
                    }

                    if was_playing {
                        state.playing = true;
                        shared.playing.store(true, Ordering::Release);

                        let target_frames =
                            ((sr as u64) * (state.cfg.queue_ms as u64) / 1000) as usize;
                        prime_music_queue(&shared, &state, &mut prod, channels, target_frames);

                        if let Err(err) = stream.play() {
                            log!("Failed to play audio stream: {err:?}");
                            state.playing = false;
                            shared.playing.store(false, Ordering::Release);
                        }
                        log!("[audio] cmd play (playing=true)");
                    }
                }
                Command::SetVolume(new_volume) => {
                    if !new_volume.is_finite() {
                        continue;
                    }
                    let v = (new_volume as f32).clamp(0.0, 1.0);
                    shared.volume_bits.store(v.to_bits(), Ordering::Release);
                }
                Command::SetMapTimeOffset(new_offset) => {
                    if !new_offset.is_finite() {
                        continue;
                    }
                    shared
                        .map_time_offset_ms_bits
                        .store(new_offset.to_bits(), Ordering::Release);
                }
                Command::SetHitsoundsOffset(new_offset) => {
                    if !new_offset.is_finite() {
                        continue;
                    }
                    shared
                        .hitsounds_offset_ms_bits
                        .store(new_offset.to_bits(), Ordering::Release);
                }
                Command::SetHitsoundVolume(new_volume) => {
                    if !new_volume.is_finite() {
                        continue;
                    }
                    let v = (new_volume as f32).clamp(0.0, 1.0);
                    shared
                        .hitsound_volume_bits
                        .store(v.to_bits(), Ordering::Release);
                }
                Command::SetSpacialAudio(new_spacial_audio) => {
                    if !new_spacial_audio.is_finite() {
                        continue;
                    }
                    let v = (new_spacial_audio as f32).clamp(0.0, 1.0);
                    shared
                        .spacial_audio_bits
                        .store(v.to_bits(), Ordering::Release);
                }
                Command::SetFixPitch(fix_pitch) => {
                    if state.playing {
                        state.playing = false;
                        shared.playing.store(false, Ordering::Release);
                        if let Err(err) = stream.pause() {
                            log!("Failed to pause audio stream: {err:?}");
                        }
                        log!("[audio] cmd pause (playing=false)");
                    }
                    shared.loading.store(true, Ordering::Release);
                    state.fix_pitch = fix_pitch;

                    let speed = shared.speed();
                    match ensure_music_base(&mut state, sr, channels) {
                        Ok(true) => {
                            match state
                                .audio_processor
                                .get_or_render(speed, state.fix_pitch, sr, channels)
                            {
                                Some(rendered) => {
                                    shared
                                        .music_frames_len
                                        .store(rendered.frames_len() as u64, Ordering::Release);
                                    state.music = Some(rendered);
                                }
                                None => {
                                    shared.music_frames_len.store(0, Ordering::Release);
                                    state.music = None;
                                }
                            }
                        }
                        Ok(false) => {}
                        Err(err) => {
                            log!("Failed to decode/render music base: {err:?}");
                        }
                    }

                    state.voices.clear();
                    state.scheduled.clear();
                    state.last_hitsound_map_time_ms = None;
                    shared.flush_requested.store(true, Ordering::Release);
                    shared.loading.store(false, Ordering::Release);
                }
                Command::SeekMapTime(map_time_ms) => {
                    let Some(music) = state.music.as_ref() else {
                        continue;
                    };

                    // Convert desired beatmap map time -> relative music time (ms).
                    // current_map_time_ms = rel_ms * speed + offset
                    // => rel_ms = (map_time_ms - offset) / speed
                    let offset_ms = shared.map_time_offset_ms();
                    let speed = shared.speed();
                    if !speed.is_finite() || speed <= 1e-9 {
                        continue;
                    }

                    let rel_ms = ((map_time_ms - offset_ms) / speed).max(0.0);
                    let mut rel_frames = ((rel_ms / 1000.0) * (sr as f64)).round() as i64;
                    rel_frames = rel_frames.clamp(0, music.frames_len() as i64);

                    let origin_abs = shared.origin_frame_abs.load(Ordering::Acquire);
                    let new_played_abs = origin_abs.saturating_add(rel_frames as u64);
                    shared
                        .played_frames_abs
                        .store(new_played_abs, Ordering::Release);

                    if !state.playing {
                        shared
                            .paused_map_time_ms_bits
                            .store(map_time_ms.to_bits(), Ordering::Release);
                    }

                    state.voices.clear();
                    state.scheduled.clear();
                    state.last_hitsound_map_time_ms = None;
                    shared.flush_requested.store(true, Ordering::Release);
                    log!(
                        "[audio] cmd seek map_time_ms={:.2} => rel_ms={:.2} rel_frames={} played_abs={}",
                        map_time_ms,
                        rel_ms,
                        rel_frames,
                        new_played_abs
                    );
                }
            }
        }

        // Apply hitsound edits in bounded chunks so control commands stay responsive.
        const MAX_HITSOUND_EDITS_PER_TICK: usize = 256;
        let mut edits_applied = 0usize;
        while edits_applied < MAX_HITSOUND_EDITS_PER_TICK {
            let Ok(cmd) = hitsound_edits_rx.try_recv() else {
                break;
            };

            match cmd {
                HitsoundEditCommand::Add {
                    map_time_ms,
                    index,
                    volume,
                    position_x,
                } => {
                    if !map_time_ms.is_finite() || !position_x.is_finite() {
                        continue;
                    }
                    let volume = volume.clamp(0.0, 1.0);
                    state.hitsound_events.push(HitsoundEvent {
                        map_time_ms,
                        index,
                        volume,
                        position_x,
                    });
                    state.hitsound_events.sort_by(|a, b| {
                        a.map_time_ms
                            .partial_cmp(&b.map_time_ms)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                HitsoundEditCommand::Remove {
                    map_time_ms,
                    index,
                    volume,
                    position_x,
                } => {
                    let target_gain = volume.clamp(0.0, 1.0);
                    let target_position_x = position_x;
                    let gain_match = |g: f64| (g - target_gain).abs() <= 1e-3;
                    let position_x_match = |x: f64| (x - target_position_x).abs() <= 1e-3;

                    let time_match = |t: f64| (t - map_time_ms).abs() <= 0.5;

                    state.hitsound_events.retain(|e| {
                        !(e.index == index
                            && gain_match(e.volume)
                            && position_x_match(e.position_x)
                            && time_match(e.map_time_ms))
                    });

                    state.scheduled.retain(|v| {
                        !(v.sample_index == index
                            && gain_match(v.gain)
                            && position_x_match(v.position_x)
                            && time_match(v.map_time_ms))
                    });
                    state.voices.retain(|v| {
                        !(v.sample_index == index
                            && gain_match(v.gain)
                            && position_x_match(v.position_x)
                            && time_match(v.map_time_ms))
                    });
                }
            }

            edits_applied += 1;
        }

        if !state.playing {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }

        let occupied = prod.occupied_len();
        let occupied_frames = occupied / channels;
        let target_frames = ((sr as u64) * (state.cfg.queue_ms as u64) / 1000) as usize;

        if occupied_frames >= target_frames {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }

        let frames_to_gen = (target_frames - occupied_frames).min(1024);
        if frames_to_gen == 0 {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }

        let mut out = vec![0.0f32; frames_to_gen * channels];

        // Absolute frame index of the first frame in this generated block.
        let played_abs = shared.played_frames_abs.load(Ordering::Acquire);
        let abs_cursor = played_abs + occupied_frames as u64;
        let origin_abs = shared.origin_frame_abs.load(Ordering::Acquire);

        // Mix music (apply music volume only).
        if let Some(music) = state.music.as_ref() {
            let rel = abs_cursor.saturating_sub(origin_abs) as usize;
            let available = music.frames_len().saturating_sub(rel);
            let frames = frames_to_gen.min(available);
            if frames > 0 {
                let start = rel * channels;
                let end = (rel + frames) * channels;
                let music_volume = shared.volume();
                if (music_volume - 1.0).abs() > f32::EPSILON {
                    for (dst, src) in out[..(frames * channels)]
                        .iter_mut()
                        .zip(music.data[start..end].iter())
                    {
                        *dst = *src * music_volume;
                    }
                } else {
                    out[..(frames * channels)].copy_from_slice(&music.data[start..end]);
                }
            }
        }

        // Activate any scheduled hitsounds whose start is within (or before) this block's end.
        let abs_end = abs_cursor + frames_to_gen as u64;
        let mut i = 0;
        while i < state.scheduled.len() {
            if state.scheduled[i].start_abs_frame <= abs_end {
                state.voices.push(state.scheduled.swap_remove(i));
            } else {
                i += 1;
            }
        }

        // Schedule hitsounds for this block based on map time window.
        let speed = shared.speed();
        if speed.is_finite() && speed > 1e-9 {
            let sr_f = sr as f64;
            let offset_ms = shared.map_time_offset_ms();
            let hitsounds_offset_ms = shared.hitsounds_offset_ms();
            let rel_start = abs_cursor.saturating_sub(origin_abs) as f64;
            let rel_end = rel_start + frames_to_gen as f64;
            let map_start = (rel_start / sr_f) * 1000.0 * speed + offset_ms;
            let map_end = (rel_end / sr_f) * 1000.0 * speed + offset_ms;

            let mut last_end = state.last_hitsound_map_time_ms.unwrap_or(map_start - 1e-3);
            if map_start + 1.0 < last_end || (map_start - last_end).abs() > 200.0 {
                last_end = map_start - 1e-3;
            }

            if map_end >= map_start {
                for ev in &state.hitsound_events {
                    let ev_time_ms = ev.map_time_ms + hitsounds_offset_ms;
                    if ev_time_ms > last_end && ev_time_ms <= map_end + 1e-6 {
                        let sample = state.hitsound_samples.get(ev.index).and_then(|s| s.clone());
                        let Some(sample) = sample else {
                            continue;
                        };

                        let rel_ms = ((ev_time_ms - offset_ms) / speed).max(0.0);
                        let rel_frames_f = (rel_ms / 1000.0) * sr_f;
                        if !rel_frames_f.is_finite() {
                            continue;
                        }
                        let rel_frames = rel_frames_f.round().max(0.0) as u64;
                        let start_abs = origin_abs.saturating_add(rel_frames);

                        state.voices.push(Voice {
                            audio: sample,
                            frame_pos: 0,
                            gain: ev.volume.clamp(0.0, 1.0),
                            position_x: ev.position_x,
                            start_abs_frame: start_abs,
                            sample_index: ev.index,
                            map_time_ms: ev.map_time_ms,
                        });
                    }
                }
            }

            state.last_hitsound_map_time_ms = Some(map_end);
        }

        // Mix voices (hitsounds).
        let hitsound_volume = shared.hitsound_volume();
        let spacial_audio = shared.spacial_audio().clamp(0.0, 1.0);
        for voice in &mut state.voices {
            // If we're already past the scheduled start (e.g. due to a big block), catch up.
            let desired_pos = abs_cursor.saturating_sub(voice.start_abs_frame) as usize;
            if desired_pos > voice.frame_pos {
                voice.frame_pos = desired_pos;
            }

            let start_off_frames = if voice.start_abs_frame > abs_cursor {
                (voice.start_abs_frame - abs_cursor) as usize
            } else {
                0
            };

            if start_off_frames >= frames_to_gen {
                continue;
            }

            let frames_in_block = frames_to_gen - start_off_frames;
            let available = voice.audio.frames_len().saturating_sub(voice.frame_pos);
            let frames = frames_in_block.min(available);
            if frames == 0 {
                continue;
            }

            let src_start = voice.frame_pos * channels;
            let src_end = (voice.frame_pos + frames) * channels;
            let dst_start = start_off_frames * channels;

            let src = &voice.audio.data[src_start..src_end];
            let base_gain = (voice.gain as f32) * hitsound_volume;

            if channels >= 2 {
                let x = voice.position_x as f32;
                let left_factor = ((1.0 - spacial_audio) + spacial_audio * (1.0 - x)).clamp(0.0, 1.0);
                let right_factor = ((1.0 - spacial_audio) + spacial_audio * x).clamp(0.0, 1.0);
                let left_gain = base_gain * left_factor;
                let right_gain = base_gain * right_factor;

                for frame in 0..frames {
                    let frame_base = frame * channels;
                    out[dst_start + frame_base] += src[frame_base] * left_gain;
                    out[dst_start + frame_base + 1] += src[frame_base + 1] * right_gain;

                    for ch in 2..channels {
                        out[dst_start + frame_base + ch] += src[frame_base + ch] * base_gain;
                    }
                }
            } else {
                for j in 0..(frames * channels) {
                    out[dst_start + j] += src[j] * base_gain;
                }
            }

            voice.frame_pos += frames;
        }
        state.voices.retain(|v| v.frame_pos < v.audio.frames_len());

        // Soft clip.
        for s in &mut out {
            *s = s.clamp(-1.0, 1.0);
        }

        let pushed = prod.push_slice(&out);
        if pushed < out.len() {
            // Should be rare; we sized queue to avoid this.
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    shared: &Arc<Shared>,
    mut cons: impl Consumer<Item = f32> + Send + 'static,
) -> Option<cpal::Stream> {
    let channels = shared.channels;
    let err_fn = |err| log!("audio stream error: {err}");
    let shared = Arc::clone(shared);

    let mut scratch: Vec<f32> = Vec::new();

    let stream = match device.build_output_stream_raw(
        config,
        sample_format,
        move |data: &mut cpal::Data, _info| {
            use cpal::Sample;

            if shared
                .callback_started
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                log!("[audio] output callback started");
            }

            if shared.flush_requested.swap(false, Ordering::AcqRel) {
                let _ = cons.clear();
            }

            let len = data.len();
            scratch.resize(len, 0.0);
            let got = cons.pop_slice(&mut scratch);
            if got < len {
                for s in &mut scratch[got..] {
                    *s = 0.0;
                }
                let prev = shared.underruns.fetch_add(1, Ordering::Relaxed);
                if prev == 0 {
                    log!("[audio] underrun (queue starved)");
                }
            }

            match data.sample_format() {
                cpal::SampleFormat::F32 => {
                    if let Some(out) = data.as_slice_mut::<f32>() {
                        out.copy_from_slice(&scratch);
                    }
                }
                cpal::SampleFormat::I16 => {
                    if let Some(out) = data.as_slice_mut::<i16>() {
                        for (dst, src) in out.iter_mut().zip(scratch.iter()) {
                            *dst = i16::from_sample(*src);
                        }
                    }
                }
                cpal::SampleFormat::U16 => {
                    if let Some(out) = data.as_slice_mut::<u16>() {
                        for (dst, src) in out.iter_mut().zip(scratch.iter()) {
                            *dst = u16::from_sample(*src);
                        }
                    }
                }
                _ => {
                    // Unsupported format: output silence.
                    if let Some(out) = data.as_slice_mut::<f32>() {
                        out.fill(0.0);
                    }
                }
            }

            // Advance time only by frames that came from the queued audio.
            // (If we had to pad with zeros, the editor timeline should not run ahead.)
            let frames = got / channels;
            if frames > 0 && shared.playing.load(Ordering::Acquire) {
                let prev = shared
                    .played_frames_abs
                    .fetch_add(frames as u64, Ordering::AcqRel);
                let new_played = prev + frames as u64;
                shared
                    .last_callback_played_frames_abs
                    .store(new_played, Ordering::Release);
                shared
                    .last_callback_time_ns
                    .store(shared.now_ns(), Ordering::Release);
            }
        },
        err_fn,
        None,
    ) {
        Ok(s) => s,
        Err(err) => {
            println!("Failed to build output stream: {err:?}");
            return None;
        }
    };

    return Some(stream);
}
