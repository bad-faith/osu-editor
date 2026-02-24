use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::time::Instant;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Fullscreen, Icon, Window, WindowId},
};

use winit::platform::run_on_demand::EventLoopExtRunOnDemand;

use crate::dotosu::helpers::{get_key_value_pairs, get_section};
use crate::geometry::atomic_vec2::AtomicVec2;
use crate::geometry::vec2::Vec2;
use crate::gpu::gpu::GpuRenderer;
use crate::gui::{DragEvent, HoverEvent, MouseHandler, RectHitbox, SimpleButton};
use crate::hitbox_handlers;
use crate::layout;
use crate::map_format::events::BreakEvent;
use crate::map_format::slider_boxing::BBox4;
use crate::render::{RenderShared, RendererThread};
use crate::skin::{Texture, load_texture};
use crate::state::{
    EditState, HitsoundRouting, HitsoundSamplesetIndices, HitsoundThreadConfig, MapState,
};
use crate::dialogue_app::DialogueApp;
use crate::{
    audio::AudioEngine, config::Config, files::BeatmapsetFolder,
    files::sanitize_name,
    skin::Skin,
};

use crate::map_format::events::Event::Break;

struct AtomicOverlayRectState {
    dragging: AtomicBool,
    start: AtomicVec2,
    corner0: AtomicVec2,
    corner1: AtomicVec2,
}

impl AtomicOverlayRectState {
    fn new() -> Self {
        Self {
            dragging: AtomicBool::new(false),
            start: AtomicVec2::new(Vec2 { x: 0.0, y: 0.0 }),
            corner0: AtomicVec2::new(Vec2 { x: 0.0, y: 0.0 }),
            corner1: AtomicVec2::new(Vec2 { x: 0.0, y: 0.0 }),
        }
    }

    fn update_drag(&self, pos: Vec2) {
        let start = {
            if !self.dragging.load(Ordering::Acquire) {
                self.start.store(pos);
                pos
            } else {
                self.start.load()
            }
        };
        let corner0 = Vec2 {
            x: start.x.min(pos.x),
            y: start.y.min(pos.y),
        };
        let corner1 = Vec2 {
            x: start.x.max(pos.x),
            y: start.y.max(pos.y),
        };
        self.corner0.store(corner0);
        self.corner1.store(corner1);
        self.dragging.store(true, Ordering::Release);
    }

    fn end_drag(&self) {
        self.dragging.store(false, Ordering::Release);
    }

    fn rect(&self) -> Option<[f32; 4]> {
        if !self.dragging.load(Ordering::Acquire) {
            return None;
        }
        Some([
            self.corner0.load().x as f32,
            self.corner0.load().y as f32,
            self.corner1.load().x as f32,
            self.corner1.load().y as f32,
        ])
    }
}

pub fn open_editor_window(
    event_loop: &mut EventLoop<()>,
    selector: &mut DialogueApp,
    beatmapset: BeatmapsetFolder,
    config: Config,
    skin: Skin,
    audio: Arc<AudioEngine>,
    hitsound_indices: HashMap<String, usize>,
) {
    let versions_strings: Vec<String> = beatmapset
        .beatmaps
        .iter()
        .map(|b| b.version.clone())
        .collect();
    if versions_strings.is_empty() {
        println!("Beatmapset has no difficulties.");
        return;
    }

    println!("Select a difficulty to edit:");
    let difficulty_images: Vec<Option<Vec<u8>>> = beatmapset
        .beatmaps
        .iter()
        .map(|beatmap| {
            let diff_dir = sanitize_name(&beatmap.version);
            let bg_small_path = Path::new("saves")
                .join(&beatmapset.map_dir_name)
                .join("diffs")
                .join(diff_dir)
                .join("bg_small.png");
            fs::read(bg_small_path).ok()
        })
        .collect();

    let selected_diff_idx = match selector.select_with_images(
        event_loop,
        "Select difficulty",
        &versions_strings,
        &difficulty_images,
    ) {
        Some(idx) => idx,
        None => {
            println!("Difficulty selection cancelled.");
            return;
        }
    };
    println!("Selected difficulty: {}", versions_strings[selected_diff_idx]);

    let mut app = match EditorApp::new(
        beatmapset,
        config,
        skin,
        audio,
        hitsound_indices,
        selected_diff_idx,
    ) {
        Some(a) => a,
        None => {
            println!("Failed to initialize editor app.");
            return;
        }
    };

    match event_loop.run_app_on_demand(&mut app) {
        Ok(()) => {}
        Err(e) => {
            println!("Editor event loop error: {:?}", e);
        }
    }
}

pub struct EditorApp {
    title: String,
    pub window: Option<Arc<Window>>,
    width: u32,
    height: u32,
    pub exiting: bool,
    editor_config: Config,
    skin: Skin,
    ui_start: Instant,
    background: Texture,
    pub audio: Arc<AudioEngine>,
    renderer: Option<RendererThread>,
    render_shared: Option<Arc<RenderShared>>,

    edit_state: Arc<RwLock<EditState>>,

    pub desired_sound_volume: f64,
    pub desired_hitsound_volume: f64,
    pub desired_fix_pitch: bool,

    sound_volume_hitbox: Rc<RectHitbox>,
    hitsound_volume_hitbox: Rc<RectHitbox>,
    playfield_scale_hitbox: Rc<RectHitbox>,
    global_interaction_hitbox: Rc<RectHitbox>,
    selection_left_bbox_hitbox: Rc<RectHitbox>,
    selection_right_bbox_hitbox: Rc<RectHitbox>,
    selection_left_origin_hitbox: Rc<RectHitbox>,
    selection_right_origin_hitbox: Rc<RectHitbox>,
    undo_button_hitbox: Rc<RectHitbox>,
    current_state_button_hitbox: Rc<RectHitbox>,
    redo_buttons_hitbox: Rc<RectHitbox>,
    progress_bar_hitbox: Rc<RectHitbox>,
    play_pause_button: Rc<SimpleButton>,

    pub mouse_handler: MouseHandler,

    pub global_interaction_hitbox_hovered: Arc<AtomicBool>,

    pub progress_bar_hitbox_hovered: Arc<AtomicBool>,

    pub sound_volume_hitbox_hovered: Arc<AtomicBool>,
    pub hitsound_volume_hitbox_hovered: Arc<AtomicBool>,
    pub playfield_scale_hitbox_hovered: Arc<AtomicBool>,
    pub selection_left_bbox_hovered: Arc<AtomicBool>,
    pub selection_right_bbox_hovered: Arc<AtomicBool>,
    pub selection_left_bbox_dragging: Arc<AtomicBool>,
    pub selection_right_bbox_dragging: Arc<AtomicBool>,
    pub selection_left_origin_hovered: Arc<AtomicBool>,
    pub selection_right_origin_hovered: Arc<AtomicBool>,
    pub selection_left_origin_dragging: Arc<AtomicBool>,
    pub selection_right_origin_dragging: Arc<AtomicBool>,
    undo_button_hovered: Arc<AtomicBool>,
    undo_button_clicked: Arc<AtomicBool>,
    current_state_button_hovered: Arc<AtomicBool>,
    current_state_button_clicked: Arc<AtomicBool>,
    current_state_button_activate_requested: Arc<AtomicBool>,
    redo_buttons_hovered_row: Arc<AtomicU32>,
    redo_buttons_clicked_row: Arc<AtomicU32>,
    selection_left_bbox_screen: Arc<RwLock<Option<BBox4>>>,
    selection_right_bbox_screen: Arc<RwLock<Option<BBox4>>>,
    selection_left_origin_playfield: Arc<AtomicVec2>,
    selection_right_origin_playfield: Arc<AtomicVec2>,
    selection_left_origin_present: Arc<AtomicBool>,
    selection_right_origin_present: Arc<AtomicBool>,
    playfield_screen_scale: Arc<AtomicVec2>,
    playfield_screen_top_left: Arc<AtomicVec2>,
    playfield_scale_state: Arc<AtomicU32>,
    viewport_width_state: Arc<AtomicU32>,
    viewport_height_state: Arc<AtomicU32>,

    drag_rect_left: Rc<AtomicOverlayRectState>,
    drag_rect_right: Rc<AtomicOverlayRectState>,
    is_renaming_current_state: bool,
    current_state_name_input: String,
}

struct SamplesetIdx {
    hitclap: usize,
    hitfinish: usize,
    hitnormal: usize,
    hitwhistle: usize,
}

impl SamplesetIdx {
    fn to_hitsound_sampleset_indices(&self) -> HitsoundSamplesetIndices {
        HitsoundSamplesetIndices {
            hitclap: self.hitclap,
            hitfinish: self.hitfinish,
            hitnormal: self.hitnormal,
            hitwhistle: self.hitwhistle,
        }
    }
}

fn load_sampleset(name: &str, hitsound_indices: &HashMap<String, usize>) -> Option<SamplesetIdx> {
    let load_sample = |sample_name: &str| -> Option<usize> {
        match hitsound_indices.get(&format!("{}-{}.wav", name, sample_name)) {
            Some(idx) => return Some(*idx),
            _ => {}
        }
        match hitsound_indices.get(&format!("{}-{}.ogg", name, sample_name)) {
            Some(idx) => return Some(*idx),
            _ => {}
        }
        match hitsound_indices.get(&format!("{}-{}.mp3", name, sample_name)) {
            Some(idx) => return Some(*idx),
            _ => {}
        }
        return None;
    };
    let sampleset_ids = SamplesetIdx {
        hitclap: match load_sample("hitclap") {
            Some(idx) => idx,
            None => {
                println!("Missing hitclap hitsound index");
                return None;
            }
        },
        hitfinish: match load_sample("hitfinish") {
            Some(idx) => idx,
            None => {
                println!("Missing hitfinish hitsound index");
                return None;
            }
        },
        hitnormal: match load_sample("hitnormal") {
            Some(idx) => idx,
            None => {
                println!("Missing hitnormal hitsound index");
                return None;
            }
        },
        hitwhistle: match load_sample("hitwhistle") {
            Some(idx) => idx,
            None => {
                println!("Missing hitwhistle hitsound index");
                return None;
            }
        },
    };
    return Some(sampleset_ids);
}

fn parse_bookmarks_from_editor_section(osu_text: &str) -> Vec<f64> {
    let editor_section = match get_section(osu_text, "Editor") {
        Some(section) => section,
        None => return Vec::new(),
    };
    let pairs = match get_key_value_pairs(editor_section) {
        Some(pairs) => pairs,
        None => return Vec::new(),
    };
    let bookmarks = match pairs.get("Bookmarks") {
        Some(value) => value,
        None => return Vec::new(),
    };

    let mut out = Vec::new();
    for chunk in bookmarks.split(',') {
        let value = chunk.trim();
        if value.is_empty() {
            continue;
        }
        if let Ok(ms) = value.parse::<f64>() {
            out.push(ms);
        }
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    out.dedup_by(|a, b| (*a - *b).abs() < 0.0001);
    out
}

fn load_bookmarks_for_diff(map_dir_name: &str, version: &str) -> Vec<f64> {
    let imported_diffs_dir = Path::new("saves").join(map_dir_name).join("imported_diffs");
    let entries = match fs::read_dir(&imported_diffs_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path
            .extension()
            .and_then(|x| x.to_str())
            .map(|x| x.eq_ignore_ascii_case("osu"))
            .unwrap_or(false)
        {
            continue;
        }

        let osu_text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };

        let metadata_section = match get_section(&osu_text, "Metadata") {
            Some(section) => section,
            None => continue,
        };
        let metadata = match get_key_value_pairs(metadata_section) {
            Some(pairs) => pairs,
            None => continue,
        };
        let osu_version = match metadata.get("Version") {
            Some(value) => value.trim(),
            None => continue,
        };
        if osu_version == version {
            return parse_bookmarks_from_editor_section(&osu_text);
        }
    }

    Vec::new()
}

impl EditorApp {
    fn new(
        beatmapset: BeatmapsetFolder,
        editor_config: Config,
        skin: Skin,
        audio: Arc<AudioEngine>,
        hitsound_indices: HashMap<String, usize>,
        selected_diff_idx: usize,
    ) -> Option<Self> {
        let beatmap = match beatmapset.beatmaps.get(selected_diff_idx) {
            Some(b) => b,
            None => {
                println!("Selected difficulty index out of range.");
                return None;
            }
        };

        let background = beatmapset.assets.get(&beatmap.events.background_name());
        let background = match background {
            Some(bytes) => match { load_texture(bytes) } {
                Some(tex) => {
                    log!("Loaded background texture from beatmap assets.");
                    tex
                }
                None => {
                    println!("Failed to load background texture from beatmap assets.");
                    return None;
                }
            },
            None => {
                println!("No background set.");
                return None;
            }
        };
        let normal_sampleset = match load_sampleset("normal", &hitsound_indices) {
            Some(s) => s,
            None => {
                println!("Failed to load normal sampleset.");
                return None;
            }
        };
        let soft_sampleset = match load_sampleset("soft", &hitsound_indices) {
            Some(s) => s,
            None => {
                println!("Failed to load soft sampleset.");
                return None;
            }
        };
        let drum_sampleset = match load_sampleset("drum", &hitsound_indices) {
            Some(s) => s,
            None => {
                println!("Failed to load drum sampleset.");
                return None;
            }
        };
        let bookmarks = load_bookmarks_for_diff(&beatmapset.map_dir_name, &beatmap.version);

        let desired_sound_volume = editor_config.audio.sound_volume;
        let desired_hitsound_volume = editor_config.audio.hitsound_volume;
        let desired_fix_pitch = editor_config.general.fix_pitch;
        let timeline_height_percent = editor_config.appearance.layout.timeline_height_percent;
        let timeline_second_box_width_percent =
            editor_config.appearance.layout.timeline_second_box_width_percent;
        let timeline_third_box_width_percent =
            editor_config.appearance.layout.timeline_third_box_width_percent;
        audio.set_volume(desired_sound_volume);
        audio.set_hitsound_volume(desired_hitsound_volume);
        audio.set_fix_pitch(desired_fix_pitch);

        let sound_volume_hitbox_hovered = Arc::new(AtomicBool::new(false));
        let hitsound_volume_hitbox_hovered = Arc::new(AtomicBool::new(false));
        let playfield_scale_hitbox_hovered = Arc::new(AtomicBool::new(false));
        let global_interaction_hitbox_hovered = Arc::new(AtomicBool::new(false));
        let progress_bar_hitbox_hovered = Arc::new(AtomicBool::new(false));
        let selection_left_bbox_hovered = Arc::new(AtomicBool::new(false));
        let selection_right_bbox_hovered = Arc::new(AtomicBool::new(false));
        let selection_left_bbox_dragging = Arc::new(AtomicBool::new(false));
        let selection_right_bbox_dragging = Arc::new(AtomicBool::new(false));
        let selection_left_origin_hovered = Arc::new(AtomicBool::new(false));
        let selection_right_origin_hovered = Arc::new(AtomicBool::new(false));
        let selection_left_origin_dragging = Arc::new(AtomicBool::new(false));
        let selection_right_origin_dragging = Arc::new(AtomicBool::new(false));
        let selection_left_bbox_screen: Arc<RwLock<Option<BBox4>>> = Arc::new(RwLock::new(None));
        let selection_right_bbox_screen: Arc<RwLock<Option<BBox4>>> = Arc::new(RwLock::new(None));
        let selection_left_origin_playfield = Arc::new(AtomicVec2::new(Vec2 { x: 0.0, y: 0.0 }));
        let selection_right_origin_playfield = Arc::new(AtomicVec2::new(Vec2 { x: 0.0, y: 0.0 }));
        let selection_left_origin_present = Arc::new(AtomicBool::new(false));
        let selection_right_origin_present = Arc::new(AtomicBool::new(false));
        let playfield_screen_scale = Arc::new(AtomicVec2::new(Vec2 { x: 1.0, y: 1.0 }));
        let playfield_screen_top_left = Arc::new(AtomicVec2::new(Vec2 { x: 0.0, y: 0.0 }));
        let playfield_scale_state = Arc::new(AtomicU32::new(
            (editor_config.general.playfield_scale.clamp(0.01, 1.0) as f32).to_bits(),
        ));
        let viewport_width_state = Arc::new(AtomicU32::new(1280));
        let viewport_height_state = Arc::new(AtomicU32::new(720));

        let drag_rect_left = Rc::new(AtomicOverlayRectState::new());
        let drag_rect_right = Rc::new(AtomicOverlayRectState::new());

        let drag_left_move: Rc<dyn Fn(Vec2)> = {
            let drag_rect_left_state = Rc::clone(&drag_rect_left);
            Rc::new(move |absolute: Vec2| {
                drag_rect_left_state.update_drag(absolute);
            })
        };
        let drag_right_move: Rc<dyn Fn(Vec2)> = {
            let drag_rect_right_state = Rc::clone(&drag_rect_right);
            Rc::new(move |absolute: Vec2| {
                drag_rect_right_state.update_drag(absolute);
            })
        };
        let drag_left_stop: Rc<dyn Fn()> = {
            let drag_rect_left_state = Rc::clone(&drag_rect_left);
            Rc::new(move || {
                drag_rect_left_state.end_drag();
            })
        };
        let drag_right_stop: Rc<dyn Fn()> = {
            let drag_rect_right_state = Rc::clone(&drag_rect_right);
            Rc::new(move || {
                drag_rect_right_state.end_drag();
            })
        };

        let audio_for_sound_drag = Arc::clone(&audio);
        let sound_volume_hitbox = hitbox_handlers::create_volume_control_hitbox(
            Arc::clone(&sound_volume_hitbox_hovered),
            Rc::new(move |value| {
                audio_for_sound_drag.set_volume(value);
            }),
        );

        let audio_for_hitsound_drag = Arc::clone(&audio);
        let hitsound_volume_hitbox = hitbox_handlers::create_volume_control_hitbox(
            Arc::clone(&hitsound_volume_hitbox_hovered),
            Rc::new(move |value| {
                audio_for_hitsound_drag.set_hitsound_volume(value);
            }),
        );

        let playfield_scale_state_for_drag = Arc::clone(&playfield_scale_state);
        let playfield_scale_hitbox = hitbox_handlers::create_volume_control_hitbox(
            Arc::clone(&playfield_scale_hitbox_hovered),
            Rc::new(move |value| {
                let clamped = value.clamp(0.01, 1.0);
                playfield_scale_state_for_drag.store((clamped as f32).to_bits(), Ordering::Release);
            }),
        );

        let global_interaction_hitbox = hitbox_handlers::create_drag_select_hitbox(
            Arc::clone(&global_interaction_hitbox_hovered),
            Rc::clone(&drag_left_move),
            Rc::clone(&drag_right_move),
            Rc::clone(&drag_left_stop),
            Rc::clone(&drag_right_stop),
        );

        let seek_dragging = Arc::new(AtomicBool::new(false));
        let seek_resume_after_drag = Arc::new(AtomicBool::new(false));
        let progress_bar_hitbox = hitbox_handlers::create_progress_bar_hitbox(
            Arc::clone(&audio),
            Arc::clone(&seek_dragging),
            Arc::clone(&seek_resume_after_drag),
            Arc::clone(&progress_bar_hitbox_hovered),
        );
        let play_pause_button = hitbox_handlers::create_play_pause_button(Arc::clone(&audio));

        let mut break_times: Vec<(f64, f64)> = Vec::new();
        for event in &beatmap.events.events {
            match event {
                Break(BreakEvent {
                    start_time: start,
                    end_time: end,
                }) => {
                    break_times.push((*start, *end));
                }
                _ => {}
            }
        }
        let kiai_times = {
            let mut kiai_times: Vec<(f64, f64)> = Vec::new();
            let mut kiai_start = None;

            for timing_point in &beatmap.timing.timing_points {
                if timing_point.effects().kiai_mode {
                    if kiai_start.is_none() {
                        kiai_start = Some(timing_point.time());
                    }
                } else {
                    if let Some(start) = kiai_start {
                        kiai_times.push((start, timing_point.time()));
                        kiai_start = None;
                    }
                }
            }
            kiai_times
        };

        let hitsound_thread_config = HitsoundThreadConfig {
            audio: Arc::clone(&audio),
            routing: HitsoundRouting {
                normal: normal_sampleset.to_hitsound_sampleset_indices(),
                soft: soft_sampleset.to_hitsound_sampleset_indices(),
                drum: drum_sampleset.to_hitsound_sampleset_indices(),
            },
        };

        let edit_state = EditState::new(
            MapState::new(
                beatmap.objects.objects.clone(),
                beatmap.timing.timing_points.clone(),
                bookmarks,
                kiai_times,
                break_times,
                beatmap.colors.combo_colors.clone(),
                beatmap.diff_settings.clone(),
                editor_config.clone(),
            ),
            hitsound_thread_config,
        );

        let undo_button_hovered = Arc::new(AtomicBool::new(false));
        let undo_button_clicked = Arc::new(AtomicBool::new(false));
        let current_state_button_hovered = Arc::new(AtomicBool::new(false));
        let current_state_button_clicked = Arc::new(AtomicBool::new(false));
        let current_state_button_activate_requested = Arc::new(AtomicBool::new(false));

        let undo_button_hitbox = Rc::new(RectHitbox::new(
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 1.0, y: 1.0 },
            {
                let edit_state = Arc::clone(&edit_state);
                let clicked = Arc::clone(&undo_button_clicked);
                let viewport_width_state = Arc::clone(&viewport_width_state);
                let viewport_height_state = Arc::clone(&viewport_height_state);
                let mut pressed_inside = false;
                let mut current_inside = false;
                Box::new(move |event: DragEvent| match event {
                    DragEvent::Move {
                        left,
                        absolute_cursor_pos,
                    } => {
                        if !left {
                            clicked.store(false, Ordering::Release);
                            pressed_inside = false;
                            current_inside = false;
                            return;
                        }
                        let screen_w = viewport_width_state.load(Ordering::Acquire).max(1) as f64;
                        let screen_h = viewport_height_state.load(Ordering::Acquire).max(1) as f64;
                        current_inside = EditorApp::undo_button_contains_cursor(
                            absolute_cursor_pos,
                            screen_w,
                            screen_h,
                            timeline_height_percent,
                        );
                        if !pressed_inside {
                            pressed_inside = current_inside;
                        }
                        clicked.store(pressed_inside && current_inside, Ordering::Release);
                    }
                    DragEvent::Stop => {
                        let trigger = pressed_inside && current_inside;
                        clicked.store(false, Ordering::Release);
                        pressed_inside = false;
                        current_inside = false;
                        if trigger {
                            edit_state
                                .write()
                                .expect("edit_state lock poisoned")
                                .undo();
                        }
                    }
                })
            },
            {
                let hovered = Arc::clone(&undo_button_hovered);
                Box::new(move |event: HoverEvent| match event {
                    HoverEvent::Move { .. } => hovered.store(true, Ordering::Release),
                    HoverEvent::Exit => hovered.store(false, Ordering::Release),
                })
            },
        ));

        let current_state_button_hitbox = Rc::new(RectHitbox::new(
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 1.0, y: 1.0 },
            {
                let clicked = Arc::clone(&current_state_button_clicked);
                let activate_requested = Arc::clone(&current_state_button_activate_requested);
                let viewport_width_state = Arc::clone(&viewport_width_state);
                let viewport_height_state = Arc::clone(&viewport_height_state);
                let mut pressed_inside = false;
                let mut current_inside = false;
                Box::new(move |event: DragEvent| match event {
                    DragEvent::Move {
                        left,
                        absolute_cursor_pos,
                    } => {
                        if !left {
                            clicked.store(false, Ordering::Release);
                            pressed_inside = false;
                            current_inside = false;
                            return;
                        }
                        let screen_w = viewport_width_state.load(Ordering::Acquire).max(1) as f64;
                        let screen_h = viewport_height_state.load(Ordering::Acquire).max(1) as f64;
                        current_inside = EditorApp::current_state_button_contains_cursor(
                            absolute_cursor_pos,
                            screen_w,
                            screen_h,
                            timeline_height_percent,
                        );
                        if !pressed_inside {
                            pressed_inside = current_inside;
                        }
                        clicked.store(pressed_inside && current_inside, Ordering::Release);
                    }
                    DragEvent::Stop => {
                        let trigger = pressed_inside && current_inside;
                        clicked.store(false, Ordering::Release);
                        pressed_inside = false;
                        current_inside = false;
                        if trigger {
                            activate_requested.store(true, Ordering::Release);
                        }
                    }
                })
            },
            {
                let hovered = Arc::clone(&current_state_button_hovered);
                Box::new(move |event: HoverEvent| match event {
                    HoverEvent::Move { .. } => hovered.store(true, Ordering::Release),
                    HoverEvent::Exit => hovered.store(false, Ordering::Release),
                })
            },
        ));

        let redo_buttons_hovered_row = Arc::new(AtomicU32::new(u32::MAX));
        let redo_buttons_clicked_row = Arc::new(AtomicU32::new(u32::MAX));

        let redo_buttons_hitbox = Rc::new(RectHitbox::new(
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 1.0, y: 1.0 },
            {
                let edit_state = Arc::clone(&edit_state);
                let clicked_row = Arc::clone(&redo_buttons_clicked_row);
                let viewport_width_state = Arc::clone(&viewport_width_state);
                let viewport_height_state = Arc::clone(&viewport_height_state);
                let mut pressed_row: Option<usize> = None;
                let mut current_row: Option<usize> = None;
                Box::new(move |event: DragEvent| match event {
                    DragEvent::Move {
                        left,
                        absolute_cursor_pos,
                    } => {
                        if !left {
                            clicked_row.store(u32::MAX, Ordering::Release);
                            pressed_row = None;
                            current_row = None;
                            return;
                        }
                        let screen_w = viewport_width_state.load(Ordering::Acquire).max(1) as f64;
                        let screen_h = viewport_height_state.load(Ordering::Acquire).max(1) as f64;
                        current_row = EditorApp::redo_button_index_from_cursor_y(
                            absolute_cursor_pos.y,
                            screen_w,
                            screen_h,
                            timeline_height_percent,
                        );
                        if pressed_row.is_none() {
                            pressed_row = current_row;
                        }
                        let click_value = pressed_row
                            .map(|row| row as u32)
                            .unwrap_or(u32::MAX);
                        clicked_row.store(click_value, Ordering::Release);
                    }
                    DragEvent::Stop => {
                        clicked_row.store(u32::MAX, Ordering::Release);
                        let trigger_row = if pressed_row.is_some() && pressed_row == current_row {
                            pressed_row
                        } else {
                            None
                        };
                        pressed_row = None;
                        current_row = None;

                        if let Some(row_idx) = trigger_row {
                            let uuid = {
                                let guard = edit_state.read().expect("edit_state lock poisoned");
                                let info = guard.undo_redo_info_for_hud();
                                info.next_states.get(row_idx).map(|state| state.uuid)
                            };
                            if let Some(uuid) = uuid {
                                edit_state
                                    .write()
                                    .expect("edit_state lock poisoned")
                                    .redo(Some(uuid));
                            }
                        }
                    }
                })
            },
            {
                let hovered_row = Arc::clone(&redo_buttons_hovered_row);
                let viewport_width_state = Arc::clone(&viewport_width_state);
                let viewport_height_state = Arc::clone(&viewport_height_state);
                Box::new(move |event: HoverEvent| match event {
                    HoverEvent::Move {
                        absolute_cursor_pos,
                    } => {
                        let screen_w = viewport_width_state.load(Ordering::Acquire).max(1) as f64;
                        let screen_h = viewport_height_state.load(Ordering::Acquire).max(1) as f64;
                        let row = EditorApp::redo_button_index_from_cursor_y(
                            absolute_cursor_pos.y,
                            screen_w,
                            screen_h,
                            timeline_height_percent,
                        )
                        .map(|idx| idx as u32)
                        .unwrap_or(u32::MAX);
                        hovered_row.store(row, Ordering::Release);
                    }
                    HoverEvent::Exit => hovered_row.store(u32::MAX, Ordering::Release),
                })
            },
        ));

        let selection_left_bbox_hitbox = hitbox_handlers::create_selection_drag_hitbox(
            Arc::clone(&selection_left_bbox_hovered),
            Arc::clone(&selection_left_bbox_dragging),
            Arc::clone(&edit_state),
            true,
            editor_config.appearance.layout.snap_distance_px,
            editor_config.appearance.layout.movable_snap_hitbox_radius_px,
            Arc::clone(&playfield_screen_scale),
            Arc::clone(&playfield_screen_top_left),
        );
        let selection_right_bbox_hitbox = hitbox_handlers::create_selection_drag_hitbox(
            Arc::clone(&selection_right_bbox_hovered),
            Arc::clone(&selection_right_bbox_dragging),
            Arc::clone(&edit_state),
            false,
            editor_config.appearance.layout.snap_distance_px,
            editor_config.appearance.layout.movable_snap_hitbox_radius_px,
            Arc::clone(&playfield_screen_scale),
            Arc::clone(&playfield_screen_top_left),
        );
        let selection_left_origin_hitbox = hitbox_handlers::create_selection_origin_drag_hitbox(
            Arc::clone(&selection_left_origin_hovered),
            Arc::clone(&selection_left_origin_dragging),
            Arc::clone(&edit_state),
            true,
            editor_config.appearance.layout.snap_distance_px,
            editor_config.appearance.layout.movable_snap_hitbox_radius_px,
            Arc::clone(&playfield_screen_scale),
            Arc::clone(&playfield_screen_top_left),
        );
        let selection_right_origin_hitbox = hitbox_handlers::create_selection_origin_drag_hitbox(
            Arc::clone(&selection_right_origin_hovered),
            Arc::clone(&selection_right_origin_dragging),
            Arc::clone(&edit_state),
            false,
            editor_config.appearance.layout.snap_distance_px,
            editor_config.appearance.layout.movable_snap_hitbox_radius_px,
            Arc::clone(&playfield_screen_scale),
            Arc::clone(&playfield_screen_top_left),
        );
        {
            let selection_left_bbox_screen = Arc::clone(&selection_left_bbox_screen);
            let selection_left_simple_hitbox = selection_left_bbox_hitbox.hitbox();
            hitbox_handlers::wire_point_hit_test(&selection_left_simple_hitbox, move |pos| {
                let Ok(guard) = selection_left_bbox_screen.read() else {
                    return false;
                };
                guard.as_ref().map(|bbox| bbox.contains(pos)).unwrap_or(false)
            });
        }
        {
            let selection_right_bbox_screen = Arc::clone(&selection_right_bbox_screen);
            let selection_right_simple_hitbox = selection_right_bbox_hitbox.hitbox();
            hitbox_handlers::wire_point_hit_test(&selection_right_simple_hitbox, move |pos| {
                let Ok(guard) = selection_right_bbox_screen.read() else {
                    return false;
                };
                guard.as_ref().map(|bbox| bbox.contains(pos)).unwrap_or(false)
            });
        }
        {
            let selection_left_origin_playfield = Arc::clone(&selection_left_origin_playfield);
            let selection_left_origin_present = Arc::clone(&selection_left_origin_present);
            let playfield_screen_scale = Arc::clone(&playfield_screen_scale);
            let playfield_screen_top_left = Arc::clone(&playfield_screen_top_left);
            let selection_left_origin_simple_hitbox = selection_left_origin_hitbox.hitbox();
            hitbox_handlers::wire_point_hit_test(&selection_left_origin_simple_hitbox, move |pos| {
                if !selection_left_origin_present.load(Ordering::Acquire) {
                    return false;
                }
                let origin = selection_left_origin_playfield.load();
                let scale = playfield_screen_scale.load();
                let top_left = playfield_screen_top_left.load();
                let origin_screen = Vec2 {
                    x: top_left.x + origin.x * scale.x,
                    y: top_left.y + origin.y * scale.y,
                };
                (pos - origin_screen).len2() <= 26.0 * 26.0
            });
        }
        {
            let selection_right_origin_playfield = Arc::clone(&selection_right_origin_playfield);
            let selection_right_origin_present = Arc::clone(&selection_right_origin_present);
            let playfield_screen_scale = Arc::clone(&playfield_screen_scale);
            let playfield_screen_top_left = Arc::clone(&playfield_screen_top_left);
            let selection_right_origin_simple_hitbox = selection_right_origin_hitbox.hitbox();
            hitbox_handlers::wire_point_hit_test(&selection_right_origin_simple_hitbox, move |pos| {
                if !selection_right_origin_present.load(Ordering::Acquire) {
                    return false;
                }
                let origin = selection_right_origin_playfield.load();
                let scale = playfield_screen_scale.load();
                let top_left = playfield_screen_top_left.load();
                let origin_screen = Vec2 {
                    x: top_left.x + origin.x * scale.x,
                    y: top_left.y + origin.y * scale.y,
                };
                (pos - origin_screen).len2() <= 26.0 * 26.0
            });
        }
        let (width, height) = (1280, 720);
        Self::update_hitbox_bounds(
            width,
            height,
            editor_config.general.playfield_scale.clamp(0.01, 1.0),
            timeline_height_percent,
            timeline_second_box_width_percent,
            timeline_third_box_width_percent,
            &sound_volume_hitbox,
            &hitsound_volume_hitbox,
            &playfield_scale_hitbox,
            &global_interaction_hitbox,
            &undo_button_hitbox,
            &current_state_button_hitbox,
            &redo_buttons_hitbox,
            &progress_bar_hitbox,
            &play_pause_button,
        );
        let mut mouse_handler = MouseHandler::new();
        mouse_handler.add_hitbox(global_interaction_hitbox.hitbox());
        mouse_handler.add_hitbox(sound_volume_hitbox.hitbox());
        mouse_handler.add_hitbox(hitsound_volume_hitbox.hitbox());
        mouse_handler.add_hitbox(playfield_scale_hitbox.hitbox());
        mouse_handler.add_hitbox(progress_bar_hitbox.hitbox());
        mouse_handler.add_hitbox(play_pause_button.hitbox());
        mouse_handler.add_hitbox(selection_right_bbox_hitbox.hitbox());
        mouse_handler.add_hitbox(selection_left_bbox_hitbox.hitbox());
        mouse_handler.add_hitbox(selection_right_origin_hitbox.hitbox());
        mouse_handler.add_hitbox(selection_left_origin_hitbox.hitbox());
        mouse_handler.add_hitbox(undo_button_hitbox.hitbox());
        mouse_handler.add_hitbox(current_state_button_hitbox.hitbox());
        mouse_handler.add_hitbox(redo_buttons_hitbox.hitbox());

        return Some(Self {
            title: format!(
                "osu editor | {} - {} [{}]",
                beatmapset.beatmapset.title,
                beatmapset.beatmapset.artist,
                beatmapset.beatmapset.creator
            ),

            edit_state,

            window: None,
            width,
            height,
            exiting: false,
            editor_config,
            skin,
            ui_start: Instant::now(),
            background: background,
            audio,
            renderer: None,
            render_shared: None,

            desired_sound_volume,
            desired_hitsound_volume,
            desired_fix_pitch,

            sound_volume_hitbox,
            hitsound_volume_hitbox,
            playfield_scale_hitbox,
            global_interaction_hitbox,
            selection_left_bbox_hitbox,
            selection_right_bbox_hitbox,
            selection_left_origin_hitbox,
            selection_right_origin_hitbox,
            undo_button_hitbox,
            current_state_button_hitbox,
            redo_buttons_hitbox,
            progress_bar_hitbox,
            play_pause_button,

            mouse_handler,
            progress_bar_hitbox_hovered,
            sound_volume_hitbox_hovered,
            hitsound_volume_hitbox_hovered,
            playfield_scale_hitbox_hovered,
            selection_left_bbox_hovered,
            selection_right_bbox_hovered,
            selection_left_bbox_dragging,
            selection_right_bbox_dragging,
            selection_left_origin_hovered,
            selection_right_origin_hovered,
            selection_left_origin_dragging,
            selection_right_origin_dragging,
            undo_button_hovered,
            undo_button_clicked,
            current_state_button_hovered,
            current_state_button_clicked,
            current_state_button_activate_requested,
            redo_buttons_hovered_row,
            redo_buttons_clicked_row,
            selection_left_bbox_screen,
            selection_right_bbox_screen,
            selection_left_origin_playfield,
            selection_right_origin_playfield,
            selection_left_origin_present,
            selection_right_origin_present,
            playfield_screen_scale,
            playfield_screen_top_left,
            playfield_scale_state,
            viewport_width_state,
            viewport_height_state,
            drag_rect_left,
            drag_rect_right,
            is_renaming_current_state: false,
            current_state_name_input: String::new(),
            global_interaction_hitbox_hovered,
        });
    }

    fn init(&mut self, event_loop: &ActiveEventLoop, editor_config: Config, skin: Skin) {
        let window_icon = match std::fs::read("assets/icon.png") {
            Ok(bytes) => match load_texture(&bytes) {
                Some(tex) => match Icon::from_rgba(tex.rgba, tex.width, tex.height) {
                    Ok(icon) => Some(icon),
                    Err(err) => {
                        log!("Failed to create window icon from assets/icon.png: {err}");
                        None
                    }
                },
                None => {
                    log!("Failed to decode assets/icon.png for window icon");
                    None
                }
            },
            Err(err) => {
                log!("Failed to read assets/icon.png for window icon: {err}");
                None
            }
        };

        let window_attributes = Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(1280, 720))
            .with_min_inner_size(LogicalSize::new(100, 100))
            .with_visible(true)
            .with_active(true)
            .with_decorations(true)
            .with_window_icon(window_icon);

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let size = window.inner_size();
        self.width = size.width.max(1);
        self.height = size.height.max(1);
        self.viewport_width_state.store(self.width, Ordering::Release);
        self.viewport_height_state.store(self.height, Ordering::Release);
        Self::update_hitbox_bounds(
            self.width,
            self.height,
            self.current_playfield_scale(),
            self.editor_config.appearance.layout.timeline_height_percent,
            self.editor_config
                .appearance
                .layout
                .timeline_second_box_width_percent,
            self.editor_config
                .appearance
                .layout
                .timeline_third_box_width_percent,
            &self.sound_volume_hitbox,
            &self.hitsound_volume_hitbox,
            &self.playfield_scale_hitbox,
            &self.global_interaction_hitbox,
            &self.undo_button_hitbox,
            &self.current_state_button_hitbox,
            &self.redo_buttons_hitbox,
            &self.progress_bar_hitbox,
            &self.play_pause_button,
        );

        // Start paused; do not advance time until the user presses play.
        let gpu = GpuRenderer::new(
            window.clone(),
            editor_config.clone(),
            skin.clone(),
            self.background.clone(),
        )
        .expect("failed to init GPU renderer");
        self.window = Some(window);

        let shared = Arc::new(RenderShared::new(
            self.width,
            self.height,
            self.current_playfield_scale(),
            Arc::clone(&self.edit_state),
        ));
        self.render_shared = Some(Arc::clone(&shared));
        self.sync_overlay_rects_to_renderer();

        self.renderer = Some(RendererThread::start(
            gpu,
            Arc::clone(&shared),
            Arc::clone(&self.audio),
            self.editor_config.clone(),
            self.ui_start,
        ));
    }
}

impl ApplicationHandler for EditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.init(event_loop, self.editor_config.clone(), self.skin.clone());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        self.handle_kb_or_mouse_event(&event);
        self.sync_overlay_rects_to_renderer();
        match event {
            WindowEvent::CloseRequested => {
                // Close just this editor instance. (The CLI process keeps running.)
                self.exit_editor_window();
            }
            WindowEvent::Destroyed => {
                // The native window is gone. Safe to leave the loop.
                self.exiting = true;
                self.stop_renderer();
                self.window.take();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                self.width = size.width.max(1);
                self.height = size.height.max(1);
                self.viewport_width_state.store(self.width, Ordering::Release);
                self.viewport_height_state.store(self.height, Ordering::Release);
                Self::update_hitbox_bounds(
                    self.width,
                    self.height,
                    self.current_playfield_scale(),
                    self.editor_config.appearance.layout.timeline_height_percent,
                    self.editor_config
                        .appearance
                        .layout
                        .timeline_second_box_width_percent,
                    self.editor_config
                        .appearance
                        .layout
                        .timeline_third_box_width_percent,
                    &self.sound_volume_hitbox,
                    &self.hitsound_volume_hitbox,
                    &self.playfield_scale_hitbox,
                    &self.global_interaction_hitbox,
                    &self.undo_button_hitbox,
                    &self.current_state_button_hitbox,
                    &self.redo_buttons_hitbox,
                    &self.progress_bar_hitbox,
                    &self.play_pause_button,
                );
                self.mark_resize(self.width, self.height);
            }
            WindowEvent::Moved(_) => {}
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = self.window.as_ref() {
                    let size = window.inner_size();
                    self.width = size.width.max(1);
                    self.height = size.height.max(1);
                    self.viewport_width_state.store(self.width, Ordering::Release);
                    self.viewport_height_state.store(self.height, Ordering::Release);
                    Self::update_hitbox_bounds(
                        self.width,
                        self.height,
                        self.current_playfield_scale(),
                        self.editor_config.appearance.layout.timeline_height_percent,
                        self.editor_config
                            .appearance
                            .layout
                            .timeline_second_box_width_percent,
                        self.editor_config
                            .appearance
                            .layout
                            .timeline_third_box_width_percent,
                        &self.sound_volume_hitbox,
                        &self.hitsound_volume_hitbox,
                        &self.playfield_scale_hitbox,
                        &self.global_interaction_hitbox,
                        &self.undo_button_hitbox,
                        &self.current_state_button_hitbox,
                        &self.redo_buttons_hitbox,
                        &self.progress_bar_hitbox,
                        &self.play_pause_button,
                    );
                    self.mark_resize(self.width, self.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let _ = event_loop;
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: ()) {
        if self.exiting {
            return;
        }
        let _ = event_loop;
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.sync_overlay_rects_to_renderer();
        if self.exiting {
            if self.window.is_none() {
                event_loop.exit();
            }
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        let _ = (event_loop, cause);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let _ = (event_loop, device_id, event);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }
}

impl EditorApp {
    fn undo_current_redo_button_metrics(
        screen_w: f64,
        screen_h: f64,
        timeline_height_percent: f64,
    ) -> (f64, f64, f64, f64, f64) {
        let margin = 8.0;
        let prev_box_h = 48.0;
        let outer_gap = 8.0;
        let text_h = 14.0;
        let adv = (text_h / 7.0) * 6.0;
        let side_padding = 8.0;
        let label_chars = 12.0;
        let value_chars = 10.0;
        let column_gap_chars = 1.0;
        let box_w = side_padding * 2.0 + adv * (label_chars + column_gap_chars + value_chars);
        let box_x1 = screen_w - margin;
        let box_x0 = box_x1 - box_w;

        let box_y0 = (screen_h * timeline_height_percent.clamp(0.0, 1.0)).max(0.0)
            + margin
            + prev_box_h
            + outer_gap;
        let button_h = 30.0;
        let button_gap = 8.0;
        (box_x0, box_x1, box_y0, button_h, button_gap)
    }

    fn undo_button_bounds(
        screen_w: f64,
        screen_h: f64,
        timeline_height_percent: f64,
    ) -> (Vec2, Vec2) {
        let (box_x0, box_x1, top_y0, button_h, _) =
            Self::undo_current_redo_button_metrics(screen_w, screen_h, timeline_height_percent);
        (
            Vec2 { x: box_x0, y: top_y0 },
            Vec2 {
                x: (box_x1 - box_x0).max(0.0),
                y: button_h.max(0.0),
            },
        )
    }

    fn current_state_button_bounds(
        screen_w: f64,
        screen_h: f64,
        timeline_height_percent: f64,
    ) -> (Vec2, Vec2) {
        let (box_x0, box_x1, top_y0, button_h, button_gap) =
            Self::undo_current_redo_button_metrics(screen_w, screen_h, timeline_height_percent);
        (
            Vec2 {
                x: box_x0,
                y: top_y0 + button_h + button_gap,
            },
            Vec2 {
                x: (box_x1 - box_x0).max(0.0),
                y: button_h.max(0.0),
            },
        )
    }

    fn redo_buttons_hitbox_bounds(
        screen_w: f64,
        screen_h: f64,
        timeline_height_percent: f64,
    ) -> (Vec2, Vec2) {
        let (box_x0, box_x1, top_y0, button_h, button_gap) =
            Self::undo_current_redo_button_metrics(screen_w, screen_h, timeline_height_percent);
        let buttons_y0 = top_y0 + (button_h + button_gap) * 2.0;
        let max_rows = 8.0;
        (
            Vec2 {
                x: box_x0,
                y: buttons_y0,
            },
            Vec2 {
                x: (box_x1 - box_x0).max(0.0),
                y: (max_rows * button_h + (max_rows - 1.0) * button_gap).max(0.0),
            },
        )
    }

    fn redo_button_index_from_cursor_y(
        cursor_y: f64,
        screen_w: f64,
        screen_h: f64,
        timeline_height_percent: f64,
    ) -> Option<usize> {
        let (_, _, top_y0, button_h, button_gap) =
            Self::undo_current_redo_button_metrics(screen_w, screen_h, timeline_height_percent);
        let buttons_y0 = top_y0 + (button_h + button_gap) * 2.0;
        if cursor_y < buttons_y0 {
            return None;
        }
        let stride = button_h + button_gap;
        let row = ((cursor_y - buttons_y0) / stride).floor();
        if row < 0.0 || row >= 8.0 {
            return None;
        }
        let row_start = buttons_y0 + row * stride;
        if cursor_y > row_start + button_h {
            return None;
        }
        Some(row as usize)
    }

    fn undo_button_contains_cursor(
        cursor_pos: Vec2,
        screen_w: f64,
        screen_h: f64,
        timeline_height_percent: f64,
    ) -> bool {
        let (origin, size) = Self::undo_button_bounds(screen_w, screen_h, timeline_height_percent);
        cursor_pos.x >= origin.x
            && cursor_pos.x <= origin.x + size.x
            && cursor_pos.y >= origin.y
            && cursor_pos.y <= origin.y + size.y
    }

    fn current_state_button_contains_cursor(
        cursor_pos: Vec2,
        screen_w: f64,
        screen_h: f64,
        timeline_height_percent: f64,
    ) -> bool {
        let (origin, size) =
            Self::current_state_button_bounds(screen_w, screen_h, timeline_height_percent);
        cursor_pos.x >= origin.x
            && cursor_pos.x <= origin.x + size.x
            && cursor_pos.y >= origin.y
            && cursor_pos.y <= origin.y + size.y
    }

    pub(crate) fn current_playfield_scale(&self) -> f64 {
        (f32::from_bits(self.playfield_scale_state.load(Ordering::Acquire)) as f64)
            .clamp(0.01, 1.0)
    }

    pub(crate) fn set_playfield_scale(&self, playfield_scale: f64) {
        let clamped = playfield_scale.clamp(0.01, 1.0);
        self.playfield_scale_state
            .store((clamped as f32).to_bits(), Ordering::Release);
    }

    fn update_hitbox_bounds(
        width: u32,
        height: u32,
        playfield_scale: f64,
        timeline_height_percent: f64,
        timeline_second_box_width_percent: f64,
        timeline_third_box_width_percent: f64,
        sound_volume_hitbox: &Rc<RectHitbox>,
        hitsound_volume_hitbox: &Rc<RectHitbox>,
        playfield_scale_hitbox: &Rc<RectHitbox>,
        global_interaction_hitbox: &Rc<RectHitbox>,
        undo_button_hitbox: &Rc<RectHitbox>,
        current_state_button_hitbox: &Rc<RectHitbox>,
        redo_buttons_hitbox: &Rc<RectHitbox>,
        progress_bar_hitbox: &Rc<RectHitbox>,
        play_pause_button: &Rc<SimpleButton>,
    ) {
        let screen_w = width.max(1);
        let screen_h = height.max(1);

        let rect_to_bounds = |rect: &layout::Rect| -> (Vec2, Vec2) {
            (
                Vec2 {
                    x: rect.x0,
                    y: rect.y0,
                },
                Vec2 {
                    x: (rect.x1 - rect.x0).max(0.0),
                    y: (rect.y1 - rect.y0).max(0.0),
                },
            )
        };

        let layout = layout::compute_layout(
            screen_w as f64,
            screen_h as f64,
            playfield_scale,
            timeline_height_percent,
            timeline_second_box_width_percent,
            timeline_third_box_width_percent,
        );
        let _legacy_split_hitboxes = (&layout.left_hitbox_rect, &layout.right_hitbox_rect);

        let (audio_top_left, audio_size) = rect_to_bounds(&layout.audio_volume_box_rect);
        let (hitsound_top_left, hitsound_size) = rect_to_bounds(&layout.hitsound_volume_box_rect);
        let (playfield_scale_top_left, playfield_scale_size) =
            rect_to_bounds(&layout.playfield_scale_box_rect);
        sound_volume_hitbox.set_bounds(audio_top_left, audio_size);
        hitsound_volume_hitbox.set_bounds(hitsound_top_left, hitsound_size);
        playfield_scale_hitbox.set_bounds(playfield_scale_top_left, playfield_scale_size);

        global_interaction_hitbox.set_bounds(
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 {
                x: screen_w as f64,
                y: screen_h as f64,
            },
        );

        let (undo_top_left, undo_size) = Self::undo_button_bounds(
            screen_w as f64,
            screen_h as f64,
            timeline_height_percent,
        );
        undo_button_hitbox.set_bounds(undo_top_left, undo_size);

        let (current_state_top_left, current_state_size) = Self::current_state_button_bounds(
            screen_w as f64,
            screen_h as f64,
            timeline_height_percent,
        );
        current_state_button_hitbox.set_bounds(current_state_top_left, current_state_size);

        let (redo_top_left, redo_size) = Self::redo_buttons_hitbox_bounds(
            screen_w as f64,
            screen_h as f64,
            timeline_height_percent,
        );
        redo_buttons_hitbox.set_bounds(redo_top_left, redo_size);

        let (timeline_top_left, timeline_size) = rect_to_bounds(&layout.timeline_hitbox_rect);
        progress_bar_hitbox.set_bounds(timeline_top_left, timeline_size);

        let (play_pause_top_left, play_pause_size) = rect_to_bounds(&layout.play_pause_button_rect);
        play_pause_button.set_bounds(play_pause_top_left, play_pause_size);
    }

    fn selection_bbox_to_screen_bbox4(playfield_rect: &layout::Rect, bbox: &BBox4) -> BBox4 {
        let scale_x = (playfield_rect.x1 - playfield_rect.x0) / 512.0;
        let scale_y = (playfield_rect.y1 - playfield_rect.y0) / 384.0;
        BBox4 {
            corners: bbox.corners.map(|corner| Vec2 {
                x: playfield_rect.x0 + corner.x * scale_x,
                y: playfield_rect.y0 + corner.y * scale_y,
            }),
        }
    }

    fn update_selection_bbox_hitbox_bounds(&self) {
        let frame_layout = layout::compute_layout(
            self.width.max(1) as f64,
            self.height.max(1) as f64,
            self.current_playfield_scale(),
            self.editor_config.appearance.layout.timeline_height_percent,
            self.editor_config
                .appearance
                .layout
                .timeline_second_box_width_percent,
            self.editor_config
                .appearance
                .layout
                .timeline_third_box_width_percent,
        );

        self.playfield_screen_scale.store(Vec2 {
            x: ((frame_layout.playfield_rect.x1 - frame_layout.playfield_rect.x0) / 512.0)
                .max(1e-9),
            y: ((frame_layout.playfield_rect.y1 - frame_layout.playfield_rect.y0) / 384.0)
                .max(1e-9),
        });
        self.playfield_screen_top_left.store(Vec2 {
            x: frame_layout.playfield_rect.x0,
            y: frame_layout.playfield_rect.y0,
        });

        let (left_bbox, right_bbox, left_origin, right_origin) = {
            let edit_state = self.edit_state.read().expect("edit_state lock poisoned");
            (
                edit_state.left_selection.as_ref().map(|s| s.bbox_outer.clone()),
                edit_state.right_selection.as_ref().map(|s| s.bbox_outer.clone()),
                edit_state.left_selection.as_ref().map(|s| s.origin),
                edit_state.right_selection.as_ref().map(|s| s.origin),
            )
        };

        let playfield_to_screen = |point: Vec2| Vec2 {
            x: frame_layout.playfield_rect.x0
                + point.x * ((frame_layout.playfield_rect.x1 - frame_layout.playfield_rect.x0) / 512.0),
            y: frame_layout.playfield_rect.y0
                + point.y * ((frame_layout.playfield_rect.y1 - frame_layout.playfield_rect.y0) / 384.0),
        };

        if let Some(left_bbox) = left_bbox {
            let screen_bbox = Self::selection_bbox_to_screen_bbox4(&frame_layout.playfield_rect, &left_bbox);
            if let Ok(mut guard) = self.selection_left_bbox_screen.write() {
                *guard = Some(screen_bbox.clone());
            }
            let aabb = screen_bbox.to_bbox();
            self.selection_left_bbox_hitbox.set_bounds(
                Vec2 {
                    x: aabb.x[0],
                    y: aabb.y[0],
                },
                Vec2 {
                    x: (aabb.x[1] - aabb.x[0]).max(0.0),
                    y: (aabb.y[1] - aabb.y[0]).max(0.0),
                },
            );
        } else {
            if let Ok(mut guard) = self.selection_left_bbox_screen.write() {
                *guard = None;
            }
            self.selection_left_bbox_hitbox
                .set_bounds(Vec2 { x: -1.0, y: -1.0 }, Vec2 { x: 0.0, y: 0.0 });
            self.selection_left_bbox_hovered
                .store(false, Ordering::Release);
        }

        if let Some(left_origin) = left_origin {
            let screen_origin = playfield_to_screen(left_origin);
            self.selection_left_origin_playfield.store(left_origin);
            self.selection_left_origin_present.store(true, Ordering::Release);
            self.selection_left_origin_hitbox.set_bounds(
                Vec2 {
                    x: screen_origin.x - 26.0,
                    y: screen_origin.y - 26.0,
                },
                Vec2 { x: 52.0, y: 52.0 },
            );
        } else {
            self.selection_left_origin_present.store(false, Ordering::Release);
            self.selection_left_origin_hitbox
                .set_bounds(Vec2 { x: -1.0, y: -1.0 }, Vec2 { x: 0.0, y: 0.0 });
            self.selection_left_origin_hovered
                .store(false, Ordering::Release);
            self.selection_left_origin_dragging
                .store(false, Ordering::Release);
        }

        if let Some(right_bbox) = right_bbox {
            let screen_bbox = Self::selection_bbox_to_screen_bbox4(&frame_layout.playfield_rect, &right_bbox);
            if let Ok(mut guard) = self.selection_right_bbox_screen.write() {
                *guard = Some(screen_bbox.clone());
            }
            let aabb = screen_bbox.to_bbox();
            self.selection_right_bbox_hitbox.set_bounds(
                Vec2 {
                    x: aabb.x[0],
                    y: aabb.y[0],
                },
                Vec2 {
                    x: (aabb.x[1] - aabb.x[0]).max(0.0),
                    y: (aabb.y[1] - aabb.y[0]).max(0.0),
                },
            );
        } else {
            if let Ok(mut guard) = self.selection_right_bbox_screen.write() {
                *guard = None;
            }
            self.selection_right_bbox_hitbox
                .set_bounds(Vec2 { x: -1.0, y: -1.0 }, Vec2 { x: 0.0, y: 0.0 });
            self.selection_right_bbox_hovered
                .store(false, Ordering::Release);
        }

        if let Some(right_origin) = right_origin {
            let screen_origin = playfield_to_screen(right_origin);
            self.selection_right_origin_playfield.store(right_origin);
            self.selection_right_origin_present
                .store(true, Ordering::Release);
            self.selection_right_origin_hitbox.set_bounds(
                Vec2 {
                    x: screen_origin.x - 26.0,
                    y: screen_origin.y - 26.0,
                },
                Vec2 { x: 52.0, y: 52.0 },
            );
        } else {
            self.selection_right_origin_present
                .store(false, Ordering::Release);
            self.selection_right_origin_hitbox
                .set_bounds(Vec2 { x: -1.0, y: -1.0 }, Vec2 { x: 0.0, y: 0.0 });
            self.selection_right_origin_hovered
                .store(false, Ordering::Release);
            self.selection_right_origin_dragging
                .store(false, Ordering::Release);
        }
    }

    fn update_selection_bbox_cursor(&self) {
        if let Some(window) = self.window.as_ref() {
            let selection_bbox_hovered = self.selection_left_bbox_hovered.load(Ordering::Acquire)
                || self.selection_right_bbox_hovered.load(Ordering::Acquire)
                || self.selection_left_origin_hovered.load(Ordering::Acquire)
                || self.selection_right_origin_hovered.load(Ordering::Acquire);
            if selection_bbox_hovered {
                window.set_cursor(winit::window::CursorIcon::Move);
            } else {
                window.set_cursor(winit::window::CursorIcon::Default);
            }
        }
    }

    fn begin_current_state_rename(&mut self) {
        self.current_state_name_input.clear();
        self.is_renaming_current_state = true;
    }

    pub fn cancel_current_state_rename(&mut self) {
        self.is_renaming_current_state = false;
        self.current_state_name_input.clear();
    }

    pub fn commit_current_state_rename(&mut self) {
        if !self.is_renaming_current_state {
            return;
        }
        self.edit_state
            .write()
            .expect("edit_state lock poisoned")
            .rename_current_state(self.current_state_name_input.clone());
        self.cancel_current_state_rename();
    }

    pub fn append_current_state_rename_text(&mut self, text: &str) {
        if !self.is_renaming_current_state {
            return;
        }
        const MAX_LEN: usize = 40;
        if self.current_state_name_input.len() >= MAX_LEN {
            return;
        }
        for ch in text.chars() {
            if ch.is_control() {
                continue;
            }
            if self.current_state_name_input.len() >= MAX_LEN {
                break;
            }
            self.current_state_name_input.push(ch);
        }
    }

    pub fn backspace_current_state_rename(&mut self) {
        if !self.is_renaming_current_state {
            return;
        }
        self.current_state_name_input.pop();
    }

    pub fn is_current_state_rename_active(&self) -> bool {
        self.is_renaming_current_state
    }

    pub fn sync_overlay_rects_to_renderer(&mut self) {
        if self
            .current_state_button_activate_requested
            .swap(false, Ordering::AcqRel)
        {
            self.begin_current_state_rename();
        }
        self.update_selection_bbox_hitbox_bounds();
        self.update_selection_bbox_cursor();
        if let Some(shared) = self.render_shared.as_ref() {
            shared.set_playfield_scale(self.current_playfield_scale());
            shared.set_overlay_rect_left(self.drag_rect_left.rect());
            shared.set_overlay_rect_right(self.drag_rect_right.rect());
            shared.set_play_pause_button_hovered(self.play_pause_button.is_hovered());
            shared.set_play_pause_button_clicked(self.play_pause_button.is_clicked());
            shared.set_undo_button_hovered(self.undo_button_hovered.load(Ordering::Acquire));
            shared.set_undo_button_clicked(self.undo_button_clicked.load(Ordering::Acquire));
            shared.set_current_state_button_hovered(
                self.current_state_button_hovered.load(Ordering::Acquire),
            );
            shared.set_current_state_button_clicked(
                self.current_state_button_clicked.load(Ordering::Acquire),
            );
            shared.set_current_state_rename_state(
                self.is_renaming_current_state,
                self.current_state_name_input.clone(),
            );
            let redo_hover_row = self.redo_buttons_hovered_row.load(Ordering::Acquire);
            shared.set_redo_button_hovered_row(if redo_hover_row == u32::MAX {
                None
            } else {
                Some(redo_hover_row)
            });
            let redo_click_row = self.redo_buttons_clicked_row.load(Ordering::Acquire);
            shared.set_redo_button_clicked_row(if redo_click_row == u32::MAX {
                None
            } else {
                Some(redo_click_row)
            });
            shared.set_selection_left_bbox_hovered(
                self.selection_left_bbox_hovered.load(Ordering::Acquire),
            );
            shared.set_selection_right_bbox_hovered(
                self.selection_right_bbox_hovered.load(Ordering::Acquire),
            );
            shared.set_selection_left_bbox_dragging(
                self.selection_left_bbox_dragging.load(Ordering::Acquire),
            );
            shared.set_selection_right_bbox_dragging(
                self.selection_right_bbox_dragging.load(Ordering::Acquire),
            );
            shared.set_selection_left_origin_hovered(
                self.selection_left_origin_hovered.load(Ordering::Acquire),
            );
            shared.set_selection_right_origin_hovered(
                self.selection_right_origin_hovered.load(Ordering::Acquire),
            );
            shared.set_selection_left_origin_dragging(
                self.selection_left_origin_dragging.load(Ordering::Acquire),
            );
            shared.set_selection_right_origin_dragging(
                self.selection_right_origin_dragging.load(Ordering::Acquire),
            );
            shared.set_cursor_pos(self.mouse_handler.position());
        }
    }

    pub fn clear_selections(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.clear_selections();
    }

    pub fn select_all_to_left(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.select_all_to_left();
    }

    pub fn select_visible_to_left(&self) {
        let time_ms = self.audio.current_time_ms();
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.select_visible_to_left(time_ms);
    }

    pub fn swap_selections(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.swap_selections();
    }

    pub fn toggle_selection_position_lock(&self, left: bool) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.toggle_selection_origin_lock(left);
    }

    pub fn toggle_selection_scale_lock(&self, left: bool) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.toggle_selection_scale_lock(left);
    }

    pub fn rotate_selection_left_90(&self, left_selection: bool) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.rotate_selection_left_90(left_selection);
    }

    pub fn rotate_selection_right_90(&self, left_selection: bool) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.rotate_selection_right_90(left_selection);
    }

    pub fn flip_selection_horizontal(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.flip_selection_horizontal();
    }

    pub fn flip_selection_vertical(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.flip_selection_vertical();
    }

    pub fn flip_left_selection_coordinates(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.flip_selection_coordinates(true);
    }

    pub fn swap_left_selection_xy(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.swap_selection_xy(true);
    }

    pub fn swap_left_selection_xy_2(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.swap_selection_xy_2(true);
    }

    pub fn swap_left_selection_xy_3(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.swap_selection_xy_3(true);
    }

    pub fn swap_left_selection_xy_4(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.swap_selection_xy_4(true);
    }

    pub fn rotate_selection_degrees(&self, left: bool, degrees: f64, checkpoint: bool) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.rotate_selection_degrees(left, degrees, checkpoint);
    }

    pub fn scale_selection_percent(&self, left: bool, percent_delta: f64, checkpoint: bool) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.scale_selection_percent(left, percent_delta, checkpoint);
    }

    pub fn translate_selection(&self, left: bool, delta: Vec2, checkpoint: bool) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.translate_selection(left, delta, checkpoint);
    }

    pub fn undo(&self) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.undo();
    }

    pub fn redo(&self, uuid: Option<u128>) {
        let mut edit_state = self.edit_state.write().expect("edit_state lock poisoned");
        edit_state.redo(uuid);
    }

    pub fn is_fullscreen(&self) -> bool {
        self.window
            .as_ref()
            .and_then(|window| window.fullscreen())
            .is_some()
    }

    pub fn set_fullscreen(&self, enabled: bool) {
        if let Some(window) = self.window.as_ref() {
            if enabled {
                let fullscreen = window.current_monitor().and_then(|monitor| {
                    monitor
                        .video_modes()
                        .max_by_key(|mode| {
                            (
                                mode.size().width as u64 * mode.size().height as u64,
                                mode.refresh_rate_millihertz(),
                            )
                        })
                        .map(Fullscreen::Exclusive)
                });

                if let Some(fullscreen) = fullscreen {
                    window.set_fullscreen(Some(fullscreen));
                } else {
                    window.set_fullscreen(Some(Fullscreen::Borderless(window.current_monitor())));
                }
            } else {
                window.set_fullscreen(None);
            }
        }
    }

    pub fn toggle_fullscreen(&self) {
        self.set_fullscreen(!self.is_fullscreen());
    }

    pub fn exit_editor_window(&mut self) {
        self.exiting = true;

        if let Some(window) = self.window.as_ref() {
            window.set_visible(false);
        }

        self.stop_renderer();
        self.window.take();
    }

    pub fn stop_renderer(&mut self) {
        if let Some(mut renderer) = self.renderer.take() {
            renderer.stop();
        }
        if let Ok(edit_state) = self.edit_state.write() {
            edit_state.request_export_thread_stop();
        }
        self.render_shared = None;
    }

    fn mark_resize(&mut self, width: u32, height: u32) {
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.mark_resize(width, height);
        }
    }
}
