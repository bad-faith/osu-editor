#[macro_use]
mod logging;

mod audio;
mod config;
mod dotosu;
mod editor;
mod exports;
mod files;
mod geometry;
mod gpu;
mod imports;
mod layout;
mod map_format;
mod dialogue_app;
mod render;
mod skin;
mod gui;
mod hitbox_handlers;
mod kb_mouse_events;
mod state;
mod treap;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    Arc,
};

use winit::event_loop::EventLoop;

use crate::audio::{AudioEngine, AudioEngineConfig};
use crate::config::Config;
use crate::editor::open_editor_window;
use crate::dialogue_app::DialogueApp;

use crate::exports::select_and_export_map;
use crate::files::{BeatmapsetFolder, get_config, open_beatmapset_folder};
use crate::imports::{select_and_import_map, select_and_import_skin};
use crate::skin::Skin;
use crate::files::scan_folder;

const EDITOR_VERSION: &str = "0.0.1";

fn main() {
    let audio = match AudioEngine::new(AudioEngineConfig {
        queue_ms: 60,
        preferred_buffer_frames: 128,
        fix_pitch: false,
    }) {
        Ok(a) => Arc::new(a),
        Err(err) => {
            println!("Audio init failed: {err:?}");
            return;
        }
    };

    // winit only supports creating a single EventLoop per process on some platforms (notably Windows).
    // Keep one around for the entire lifetime of the CLI so you can open/close the editor repeatedly.
    let mut event_loop = EventLoop::new().expect("Failed to create winit EventLoop");
    let mut selector = DialogueApp::new();

    loop {
        let option_strings: Vec<String> = vec![
            "import .osz map from imports/".to_string(),
            "import .osk skin from imports/".to_string(),
            "open a map from saves/".to_string(),
            "export a map from saves/".to_string(),
            "exit".to_string(),
        ];

        let selection = match selector.select(&mut event_loop, "Main menu", &option_strings) {
            Some(idx) => idx,
            None => break,
        };

        match selection {
            0 => select_and_import_map(&mut event_loop, &mut selector),
            1 => select_and_import_skin(&mut event_loop, &mut selector),
            2 => select_and_open_map(&mut event_loop, &mut selector, &audio),
            3 => select_and_export_map(&mut event_loop, &mut selector),
            4 => break,
            _ => unreachable!(),
        }
    }
}

fn select_and_open_map(
    event_loop: &mut EventLoop<()>,
    selector: &mut DialogueApp,
    audio: &Arc<AudioEngine>,
) {
    println!("Opening map...");

    let config = match get_config() {
        Some(cfg) => cfg,
        None => {
            println!("Failed to load config.json, using default config.");
            return;
        }
    };

    let skin = match Skin::load_from_path(&Path::new("skins/").join(&config.appearance.general.skin), &Path::new("skins/default")) {
        Some(skin) => skin,
        None => {
            println!("Failed to load skin.");
            return;
        }
    };

    // --- Step 1: scan saves/ ---
    let saves_path = Path::new("saves");
    if !saves_path.exists() {
        println!("No saves/ directory found.");
        return;
    }

    let entries = scan_folder(saves_path, Some(true), None);

    if entries.is_empty() {
        println!("No maps found in saves/");
        return;
    }

    // --- Step 2: console selection ---
    println!("Available maps:");
    let selection = match selector.select(event_loop, "Select a map", &entries) {
        Some(idx) => idx,
        None => {
            println!("Map selection cancelled.");
            return;
        }
    };
    let map_dir_name = &entries[selection];
    let beatmapset = match open_beatmapset_folder(map_dir_name) {
        Some(beatmapset) => beatmapset,
        None => {
            println!("Failed to open beatmapset folder for {}", map_dir_name);
            return;
        }
    };
    println!("Launching: {}", map_dir_name);

    match load_beatmapset_audio(&beatmapset, &config, map_dir_name, audio) {
        Some(()) => {}
        None => {
            println!("Failed to load beatmap audio.");
            return;
        }
    }

    audio.remove_all_hitsound_samples();
    audio.remove_all_hitsounds();
    let mut hitsound_indices: HashMap<String, usize> = HashMap::new();
    for (name, _) in &skin.hitsounds {
        let index = hitsound_indices.len();
        hitsound_indices.insert(name.clone(), index);
    }
    for (name, bytes) in &skin.hitsounds {
        let index = hitsound_indices.get(name).unwrap();
        let hint_ext = std::path::Path::new(name)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());
        audio.set_hitsound_sample(bytes.clone(), *index, name.clone(), hint_ext);
    }

    open_editor_window(
        event_loop,
        selector,
        beatmapset,
        config,
        skin,
        Arc::clone(audio),
        hitsound_indices,
    );

    audio.stop();
}

fn load_beatmapset_audio(
    beatmapset: &BeatmapsetFolder,
    config: &Config,
    map_dir_name: &str,
    audio: &Arc<AudioEngine>,
) -> Option<()> {
    if let Some(bytes) = beatmapset
        .assets
        .get(beatmapset.beatmapset.audio_filename.as_str())
    {
        audio.pause();
        audio.set_fix_pitch(config.general.fix_pitch);
        audio.set_speed(config.general.speed);
        audio.set_volume(config.audio.sound_volume);
        audio.set_hitsound_volume(config.audio.hitsound_volume);
        audio.set_spacial_audio(config.audio.spacial_audio);
        audio.set_map_time_offset_ms(config.audio.audio_offset_ms);
        audio.set_hitsounds_offset_ms(config.audio.hitsounds_offset_ms);
        audio.load_music(
            bytes.clone(),
            map_dir_name,
            beatmapset.beatmapset.audio_filename.as_str(),
        );
        audio.pause();
        Some(())
    } else {
        println!(
            "Audio file '{}' not found in beatmap assets.",
            beatmapset.beatmapset.audio_filename
        );
        return None;
    }
}
