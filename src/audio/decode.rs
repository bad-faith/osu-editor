use std::time::Instant;

use symphonia::core::{
    audio::{AudioBufferRef, SampleBuffer, Signal},
    codecs::DecoderOptions,
    errors::Error as SymphoniaError,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

pub struct DecodedAudio {
    pub sample_rate: u32,
    pub channels: usize,
    /// Planar f32 samples: samples[channel][frame].
    pub samples: Vec<Vec<f32>>,
}

pub fn decode_audio_from_bytes(bytes: Vec<u8>, hint_ext: Option<&str>) -> Option<DecodedAudio> {
    let t0 = Instant::now();
    log!(
        "[audio] decode start (bytes={}, hint_ext={:?})",
        bytes.len(),
        hint_ext
    );
    let mut hint = Hint::new();
    if let Some(ext) = hint_ext {
        let ext = ext.trim().trim_start_matches('.');
        if !ext.is_empty() {
            hint.with_extension(ext);
        }
    }

    let probe = symphonia::default::get_probe();
    let probed = match probe.format(
        &hint,
        MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes.clone())), Default::default()),
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ) {
        Ok(p) => p,
        Err(e) => {
            let hint_was_set = hint_ext
                .map(|ext| !ext.trim().trim_start_matches('.').is_empty())
                .unwrap_or(false);
            if hint_was_set {
                log!(
                    "[audio] decode_audio_from_bytes: probe failed with hint {:?}, retrying without hint",
                    hint_ext
                );
                let empty_hint = Hint::new();
                match probe.format(
                    &empty_hint,
                    MediaSourceStream::new(
                        Box::new(std::io::Cursor::new(bytes.clone())),
                        Default::default(),
                    ),
                    &FormatOptions::default(),
                    &MetadataOptions::default(),
                ) {
                    Ok(p) => p,
                    Err(e2) => {
                        println!("[audio] decode_audio_from_bytes: format probe error: {e2}");
                        return None;
                    }
                }
            } else {
                println!("[audio] decode_audio_from_bytes: format probe error: {e}");
                return None;
            }
        }
    };

    let mut format = probed.format;

    let track = match format.default_track() {
        Some(t) => t,
        None => {
            println!("[audio] decode_audio_from_bytes: no default audio track");
            return None;
        }
    };

    let track_id = track.id;
    let codec_params = track.codec_params.clone();

    let sample_rate = match codec_params.sample_rate {
        Some(sr) => sr,
        None => {
            println!("[audio] decode_audio_from_bytes: missing sample rate");
            return None;
        }
    };

    let channels = match codec_params.channels {
        Some(ch) => ch.count(),
        None => {
            println!("[audio] decode_audio_from_bytes: missing channel count");
            return None;
        }
    };

    let mut decoder =
        match symphonia::default::get_codecs().make(&codec_params, &DecoderOptions::default()) {
            Ok(d) => d,
            Err(e) => {
                println!("[audio] decode_audio_from_bytes: failed to create decoder: {e}");
                return None;
            }
        };
    let mut samples: Vec<Vec<f32>> = (0..channels).map(|_| Vec::new()).collect();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(_)) => break,
            Err(e) => {
                println!("[audio] decode_audio_from_bytes: failed reading packet: {e}");
                return None;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(audio_buf) => audio_buf,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(SymphoniaError::IoError(_)) => break,
            Err(e) => {
                println!("[audio] decode_audio_from_bytes: decode error: {e}");
                return None;
            }
        };

        match decoded {
            AudioBufferRef::F32(buf) => {
                let ch = buf.spec().channels.count();
                let frames = buf.frames();
                if ch != channels {
                    println!(
                        "[audio] decode_audio_from_bytes: channel count changed mid-stream: {channels} -> {ch}"
                    );
                    return None;
                }
                for c in 0..channels {
                    samples[c].extend_from_slice(&buf.chan(c)[..frames]);
                }
            }
            other => {
                let spec = *other.spec();
                let frames = other.frames();
                let mut sb = SampleBuffer::<f32>::new(frames as u64, spec);
                sb.copy_interleaved_ref(other);

                let ch = spec.channels.count();
                if ch != channels {
                    println!(
                        "[audio] decode_audio_from_bytes: channel count changed mid-stream: {channels} -> {ch}"
                    );
                    return None;
                }

                let interleaved = sb.samples();
                for frame in 0..frames {
                    for c in 0..channels {
                        samples[c].push(interleaved[frame * channels + c]);
                    }
                }
            }
        }
    }

    if samples.is_empty() || samples[0].is_empty() {
        println!("[audio] decode_audio_from_bytes: decoded audio is empty");
        return None;
    }

    let frames = samples[0].len();
    log!(
        "[audio] decode ok (sr={}, ch={}, frames={}, {:.2}s)",
        sample_rate,
        channels,
        frames,
        t0.elapsed().as_secs_f64()
    );

    Some(DecodedAudio {
        sample_rate,
        channels,
        samples,
    })
}
