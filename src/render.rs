use std::{
    collections::{HashSet, VecDeque},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::{
    audio::AudioEngine,
    config::Config,
    geometry::vec2::Vec2,
    gpu::gpu::{GpuRenderer, ObjectInstance},
    layout::{self, Rect},
    map_format::slider_boxing::{BBox, BBox4},
    state::{EditState, Object},
};

pub fn is_object_currently_visible(object: &ObjectInstance, time_ms: f64) -> bool {
    const FADE_OUT_MS: f64 = 250.0;

    let appear_ms = if object.is_spinner {
        object.time
    } else {
        object.time - object.preempt
    };
    let end_ms = if object.is_slider || object.is_spinner {
        object.slider_end_time_ms
    } else {
        object.time
    };
    let disappear_ms = end_ms + FADE_OUT_MS;

    time_ms >= appear_ms && time_ms <= disappear_ms
}

fn playfield_to_screen(pos: Vec2, playfield_rect: &layout::Rect) -> Vec2 {
    let scale_x = (playfield_rect.x1 - playfield_rect.x0) / 512.0;
    let scale_y = (playfield_rect.y1 - playfield_rect.y0) / 384.0;
    Vec2 {
        x: playfield_rect.x0 + pos.x * scale_x,
        y: playfield_rect.y0 + pos.y * scale_y,
    }
}

fn rect_contains_point(rect: [f32; 4], point: Vec2) -> bool {
    point.x >= rect[0] as f64
        && point.x <= rect[2] as f64
        && point.y >= rect[1] as f64
        && point.y <= rect[3] as f64
}

fn bbox4_to_screen_quad(bbox4: &BBox4, playfield_rect: &Rect) -> [[f32; 2]; 4] {
    let scale_x = (playfield_rect.x1 - playfield_rect.x0) / 512.0;
    let scale_y = (playfield_rect.y1 - playfield_rect.y0) / 384.0;
    bbox4.corners.map(|corner| {
        [
            (playfield_rect.x0 + corner.x * scale_x) as f32,
            (playfield_rect.y0 + corner.y * scale_y) as f32,
        ]
    })
}

pub fn select_visible_objects_in_rect<'a>(
    selection_rect: [f32; 4],
    object_instances: impl Iterator<Item = &'a Object>,
    playfield_rect: &layout::Rect,
    time_ms: f64,
    blocked_objects: &[usize],
    must_take: &[usize],
) -> (Vec<usize>, Option<BBox>) {
    let blocked: HashSet<usize> = blocked_objects.iter().copied().collect();
    let must_take: HashSet<usize> = must_take.iter().copied().collect();
    let mut bbox_inner: Option<BBox> = None;
    let objects = object_instances
        .enumerate()
        .filter_map(|(idx, object)| {
            if blocked.contains(&idx) {
                return None;
            }
            let must_take_this = must_take.contains(&idx);
            let object_instance = match object.instance() {
                Some(instance) => instance,
                None => panic!("object instance should be available while rendering"),
            };
            if !must_take_this && !is_object_currently_visible(object_instance, time_ms) {
                return None;
            }
            let object_screen_pos = playfield_to_screen(object_instance.pos, playfield_rect);
            let object_screen_end_pos =
                playfield_to_screen(object_instance.end_pos(), playfield_rect);

            if rect_contains_point(selection_rect, object_screen_pos)
                || rect_contains_point(selection_rect, object_screen_end_pos)
            {
                let object_bbox_inner = &object_instance.bbox_inner;
                bbox_inner = Some(match &bbox_inner {
                    Some(b) => BBox {
                        x: [
                            b.x[0].min(object_bbox_inner.x[0]),
                            b.x[1].max(object_bbox_inner.x[1]),
                        ],
                        y: [
                            b.y[0].min(object_bbox_inner.y[0]),
                            b.y[1].max(object_bbox_inner.y[1]),
                        ],
                    },
                    None => object_bbox_inner.clone(),
                });
                Some(idx)
            } else {
                None
            }
        })
        .collect();
    const BBOX_MIN_SIZE: f64 = 1e-3;
    bbox_inner = match bbox_inner {
        Some(b) => Some({
            let mut x = b.x;
            let mut y = b.y;
            if (x[1] - x[0]).abs() < BBOX_MIN_SIZE {
                let mid = (x[0] + x[1]) / 2.0;
                x[0] = mid - BBOX_MIN_SIZE;
                x[1] = mid + BBOX_MIN_SIZE;
            }
            if (y[1] - y[0]).abs() < BBOX_MIN_SIZE {
                let mid = (y[0] + y[1]) / 2.0;
                y[0] = mid - BBOX_MIN_SIZE;
                y[1] = mid + BBOX_MIN_SIZE;
            }
            BBox { x, y }
        }),
        None => None,
    };
    (objects, bbox_inner)
}

struct AtomicOverlayRect {
    enabled: AtomicBool,
    x0: AtomicU32,
    y0: AtomicU32,
    x1: AtomicU32,
    y1: AtomicU32,
}

impl AtomicOverlayRect {
    fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            x0: AtomicU32::new(0),
            y0: AtomicU32::new(0),
            x1: AtomicU32::new(0),
            y1: AtomicU32::new(0),
        }
    }

    fn set(&self, rect: Option<[f32; 4]>) {
        match rect {
            Some([x0, y0, x1, y1]) => {
                self.x0.store(x0.to_bits(), Ordering::Release);
                self.y0.store(y0.to_bits(), Ordering::Release);
                self.x1.store(x1.to_bits(), Ordering::Release);
                self.y1.store(y1.to_bits(), Ordering::Release);
                self.enabled.store(true, Ordering::Release);
            }
            None => {
                self.enabled.store(false, Ordering::Release);
            }
        }
    }

    fn get(&self) -> Option<[f32; 4]> {
        if !self.enabled.load(Ordering::Acquire) {
            return None;
        }
        Some([
            f32::from_bits(self.x0.load(Ordering::Acquire)),
            f32::from_bits(self.y0.load(Ordering::Acquire)),
            f32::from_bits(self.x1.load(Ordering::Acquire)),
            f32::from_bits(self.y1.load(Ordering::Acquire)),
        ])
    }
}

pub struct RenderShared {
    exit: AtomicBool,
    resize_pending: AtomicBool,
    width: AtomicU32,
    height: AtomicU32,
    playfield_scale_bits: AtomicU32,
    timeline_zoom_bits: AtomicU32,
    is_playing: AtomicBool,
    is_loading: AtomicBool,
    overlay_rect_left: AtomicOverlayRect,
    overlay_rect_right: AtomicOverlayRect,
    selection_left_bbox_hovered: AtomicBool,
    selection_right_bbox_hovered: AtomicBool,
    selection_left_bbox_dragging: AtomicBool,
    selection_right_bbox_dragging: AtomicBool,
    selection_left_origin_hovered: AtomicBool,
    selection_right_origin_hovered: AtomicBool,
    selection_left_origin_dragging: AtomicBool,
    selection_right_origin_dragging: AtomicBool,
    cursor_x: AtomicU32,
    cursor_y: AtomicU32,
    play_pause_button_hovered: AtomicBool,
    play_pause_button_clicked: AtomicBool,
    undo_button_hovered: AtomicBool,
    undo_button_clicked: AtomicBool,
    current_state_button_hovered: AtomicBool,
    current_state_button_clicked: AtomicBool,
    current_state_rename_active: AtomicBool,
    current_state_rename_text: RwLock<String>,
    redo_button_hovered_row: AtomicU32,
    redo_button_clicked_row: AtomicU32,
    edit_state: Arc<RwLock<EditState>>,
}

impl RenderShared {
    pub fn new(
        width: u32,
        height: u32,
        playfield_scale: f64,
        edit_state: Arc<RwLock<EditState>>,
    ) -> Self {
        Self {
            exit: AtomicBool::new(false),
            resize_pending: AtomicBool::new(false),
            width: AtomicU32::new(width),
            height: AtomicU32::new(height),
            playfield_scale_bits: AtomicU32::new((playfield_scale.clamp(0.01, 1.0) as f32).to_bits()),
            timeline_zoom_bits: AtomicU32::new((1.0f32).to_bits()),
            is_playing: AtomicBool::new(false),
            is_loading: AtomicBool::new(true),
            overlay_rect_left: AtomicOverlayRect::new(),
            overlay_rect_right: AtomicOverlayRect::new(),
            selection_left_bbox_hovered: AtomicBool::new(false),
            selection_right_bbox_hovered: AtomicBool::new(false),
            selection_left_bbox_dragging: AtomicBool::new(false),
            selection_right_bbox_dragging: AtomicBool::new(false),
            selection_left_origin_hovered: AtomicBool::new(false),
            selection_right_origin_hovered: AtomicBool::new(false),
            selection_left_origin_dragging: AtomicBool::new(false),
            selection_right_origin_dragging: AtomicBool::new(false),
            cursor_x: AtomicU32::new(0.0f32.to_bits()),
            cursor_y: AtomicU32::new(0.0f32.to_bits()),
            play_pause_button_hovered: AtomicBool::new(false),
            play_pause_button_clicked: AtomicBool::new(false),
            undo_button_hovered: AtomicBool::new(false),
            undo_button_clicked: AtomicBool::new(false),
            current_state_button_hovered: AtomicBool::new(false),
            current_state_button_clicked: AtomicBool::new(false),
            current_state_rename_active: AtomicBool::new(false),
            current_state_rename_text: RwLock::new(String::new()),
            redo_button_hovered_row: AtomicU32::new(u32::MAX),
            redo_button_clicked_row: AtomicU32::new(u32::MAX),
            edit_state,
        }
    }

    pub fn set_playfield_scale(&self, playfield_scale: f64) {
        self.playfield_scale_bits
            .store((playfield_scale.clamp(0.01, 1.0) as f32).to_bits(), Ordering::Release);
    }

    pub fn playfield_scale(&self) -> f64 {
        f32::from_bits(self.playfield_scale_bits.load(Ordering::Acquire)) as f64
    }

    pub fn set_timeline_zoom(&self, timeline_zoom: f64) {
        self.timeline_zoom_bits
            .store((timeline_zoom.clamp(0.1, 10.0) as f32).to_bits(), Ordering::Release);
    }

    pub fn timeline_zoom(&self) -> f64 {
        f32::from_bits(self.timeline_zoom_bits.load(Ordering::Acquire)) as f64
    }

    pub fn set_overlay_rect_left(&self, rect: Option<[f32; 4]>) {
        self.overlay_rect_left.set(rect);
    }

    pub fn overlay_rect_left(&self) -> Option<[f32; 4]> {
        self.overlay_rect_left.get()
    }

    pub fn set_overlay_rect_right(&self, rect: Option<[f32; 4]>) {
        self.overlay_rect_right.set(rect);
    }

    pub fn overlay_rect_right(&self) -> Option<[f32; 4]> {
        self.overlay_rect_right.get()
    }

    pub fn set_play_pause_button_hovered(&self, hovered: bool) {
        self.play_pause_button_hovered
            .store(hovered, Ordering::Release);
    }

    pub fn set_selection_left_bbox_hovered(&self, hovered: bool) {
        self.selection_left_bbox_hovered
            .store(hovered, Ordering::Release);
    }

    pub fn selection_left_bbox_hovered(&self) -> bool {
        self.selection_left_bbox_hovered.load(Ordering::Acquire)
    }

    pub fn set_selection_right_bbox_hovered(&self, hovered: bool) {
        self.selection_right_bbox_hovered
            .store(hovered, Ordering::Release);
    }

    pub fn selection_right_bbox_hovered(&self) -> bool {
        self.selection_right_bbox_hovered.load(Ordering::Acquire)
    }

    pub fn set_selection_left_bbox_dragging(&self, dragging: bool) {
        self.selection_left_bbox_dragging
            .store(dragging, Ordering::Release);
    }

    pub fn selection_left_bbox_dragging(&self) -> bool {
        self.selection_left_bbox_dragging.load(Ordering::Acquire)
    }

    pub fn set_selection_right_bbox_dragging(&self, dragging: bool) {
        self.selection_right_bbox_dragging
            .store(dragging, Ordering::Release);
    }

    pub fn selection_right_bbox_dragging(&self) -> bool {
        self.selection_right_bbox_dragging.load(Ordering::Acquire)
    }

    pub fn set_selection_left_origin_hovered(&self, hovered: bool) {
        self.selection_left_origin_hovered
            .store(hovered, Ordering::Release);
    }

    pub fn selection_left_origin_hovered(&self) -> bool {
        self.selection_left_origin_hovered.load(Ordering::Acquire)
    }

    pub fn set_selection_right_origin_hovered(&self, hovered: bool) {
        self.selection_right_origin_hovered
            .store(hovered, Ordering::Release);
    }

    pub fn selection_right_origin_hovered(&self) -> bool {
        self.selection_right_origin_hovered.load(Ordering::Acquire)
    }

    pub fn set_selection_left_origin_dragging(&self, dragging: bool) {
        self.selection_left_origin_dragging
            .store(dragging, Ordering::Release);
    }

    pub fn selection_left_origin_dragging(&self) -> bool {
        self.selection_left_origin_dragging.load(Ordering::Acquire)
    }

    pub fn set_selection_right_origin_dragging(&self, dragging: bool) {
        self.selection_right_origin_dragging
            .store(dragging, Ordering::Release);
    }

    pub fn selection_right_origin_dragging(&self) -> bool {
        self.selection_right_origin_dragging.load(Ordering::Acquire)
    }

    pub fn set_cursor_pos(&self, pos: Vec2) {
        self.cursor_x
            .store((pos.x as f32).to_bits(), Ordering::Release);
        self.cursor_y
            .store((pos.y as f32).to_bits(), Ordering::Release);
    }

    pub fn cursor_pos(&self) -> [f32; 2] {
        [
            f32::from_bits(self.cursor_x.load(Ordering::Acquire)),
            f32::from_bits(self.cursor_y.load(Ordering::Acquire)),
        ]
    }

    pub fn play_pause_button_hovered(&self) -> bool {
        self.play_pause_button_hovered.load(Ordering::Acquire)
    }

    pub fn set_play_pause_button_clicked(&self, clicked: bool) {
        self.play_pause_button_clicked
            .store(clicked, Ordering::Release);
    }

    pub fn play_pause_button_clicked(&self) -> bool {
        self.play_pause_button_clicked.load(Ordering::Acquire)
    }

    pub fn set_undo_button_hovered(&self, hovered: bool) {
        self.undo_button_hovered.store(hovered, Ordering::Release);
    }

    pub fn undo_button_hovered(&self) -> bool {
        self.undo_button_hovered.load(Ordering::Acquire)
    }

    pub fn set_undo_button_clicked(&self, clicked: bool) {
        self.undo_button_clicked.store(clicked, Ordering::Release);
    }

    pub fn undo_button_clicked(&self) -> bool {
        self.undo_button_clicked.load(Ordering::Acquire)
    }

    pub fn set_current_state_button_hovered(&self, hovered: bool) {
        self.current_state_button_hovered
            .store(hovered, Ordering::Release);
    }

    pub fn current_state_button_hovered(&self) -> bool {
        self.current_state_button_hovered.load(Ordering::Acquire)
    }

    pub fn set_current_state_button_clicked(&self, clicked: bool) {
        self.current_state_button_clicked
            .store(clicked, Ordering::Release);
    }

    pub fn current_state_button_clicked(&self) -> bool {
        self.current_state_button_clicked.load(Ordering::Acquire)
    }

    pub fn set_current_state_rename_state(
        &self,
        active: bool,
        text: String,
    ) {
        self.current_state_rename_active
            .store(active, Ordering::Release);
        if let Ok(mut guard) = self.current_state_rename_text.write() {
            *guard = text;
        }
    }

    pub fn current_state_rename_state(&self) -> (bool, String) {
        let active = self.current_state_rename_active.load(Ordering::Acquire);
        let text = self
            .current_state_rename_text
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        (active, text)
    }

    pub fn set_redo_button_hovered_row(&self, row: Option<u32>) {
        self.redo_button_hovered_row
            .store(row.unwrap_or(u32::MAX), Ordering::Release);
    }

    pub fn redo_button_hovered_row(&self) -> Option<u32> {
        let row = self.redo_button_hovered_row.load(Ordering::Acquire);
        if row == u32::MAX {
            None
        } else {
            Some(row)
        }
    }

    pub fn set_redo_button_clicked_row(&self, row: Option<u32>) {
        self.redo_button_clicked_row
            .store(row.unwrap_or(u32::MAX), Ordering::Release);
    }

    pub fn redo_button_clicked_row(&self) -> Option<u32> {
        let row = self.redo_button_clicked_row.load(Ordering::Acquire);
        if row == u32::MAX {
            None
        } else {
            Some(row)
        }
    }
}

pub struct RendererThread {
    shared: Arc<RenderShared>,
    handle: Option<JoinHandle<()>>,
}

impl RendererThread {
    pub fn start(
        mut gpu: GpuRenderer,
        shared: Arc<RenderShared>,
        audio: Arc<AudioEngine>,
        editor_config: Config,
        ui_start: Instant,
    ) -> Self {
        let fps = editor_config.performance.fps_limiter;
        let timeline_height_percent = editor_config.appearance.layout.timeline_height_percent;
        let timeline_second_box_width_percent =
            editor_config.appearance.layout.timeline_second_box_width_percent;
        let timeline_third_box_width_percent =
            editor_config.appearance.layout.timeline_third_box_width_percent;
        let frame_duration = Duration::from_secs_f64(1.0 / fps);
        let shared_for_thread = Arc::clone(&shared);
        let handle = std::thread::Builder::new()
            .name("renderer".to_string())
            .spawn(move || {
                let mut width = shared_for_thread.width.load(Ordering::Acquire);
                let mut height = shared_for_thread.height.load(Ordering::Acquire);
                let mut last_frame = Instant::now();
                let mut fps_history: VecDeque<(Instant, f64)> = VecDeque::new();
                let mut playfield_scale = shared_for_thread.playfield_scale().clamp(0.01, 1.0);
                let mut frame_layout = layout::compute_layout(
                    width as f64,
                    height as f64,
                    playfield_scale,
                    timeline_height_percent,
                    timeline_second_box_width_percent,
                    timeline_third_box_width_percent,
                );

                loop {
                    if shared_for_thread.exit.load(Ordering::Acquire) {
                        break;
                    }

                    if shared_for_thread
                        .resize_pending
                        .swap(false, Ordering::AcqRel)
                    {
                        width = shared_for_thread.width.load(Ordering::Acquire).max(1);
                        height = shared_for_thread.height.load(Ordering::Acquire).max(1);
                        gpu.resize(winit::dpi::PhysicalSize::new(width, height));
                        playfield_scale = shared_for_thread.playfield_scale().clamp(0.01, 1.0);
                        frame_layout = layout::compute_layout(
                            width as f64,
                            height as f64,
                            playfield_scale,
                            timeline_height_percent,
                            timeline_second_box_width_percent,
                            timeline_third_box_width_percent,
                        );
                    }

                    let latest_playfield_scale = shared_for_thread.playfield_scale().clamp(0.01, 1.0);
                    if (latest_playfield_scale - playfield_scale).abs() > 1e-6 {
                        playfield_scale = latest_playfield_scale;
                        frame_layout = layout::compute_layout(
                            width as f64,
                            height as f64,
                            playfield_scale,
                            timeline_height_percent,
                            timeline_second_box_width_percent,
                            timeline_third_box_width_percent,
                        );
                    }

                    let now = Instant::now();
                    let fps_current = 1.0 / now.duration_since(last_frame).as_secs_f64().max(1e-12);
                    if now >= last_frame + frame_duration {
                        last_frame = now;
                    } else {
                        continue;
                    }

                    let fps_clamped = fps_current.clamp(0.0, u32::MAX as f64 / 10.0);
                    fps_history.push_back((now, fps_clamped));
                    while let Some((ts, _)) = fps_history.front() {
                        if now.duration_since(*ts).as_secs_f64() > 1.0 {
                            fps_history.pop_front();
                        } else {
                            break;
                        }
                    }
                    let fps_low = fps_history
                        .iter()
                        .map(|(_, value)| *value)
                        .fold(fps_clamped, f64::min);

                    let song_total_ms = audio.song_total_ms();
                    let time_ms = audio.current_time_ms();
                    let timeline_zoom = shared_for_thread.timeline_zoom().clamp(0.1, 10.0);
                    let time_elapsed_ms = ui_start.elapsed().as_secs_f64() * 1000.0;
                    let is_loading = song_total_ms <= 0.0 || audio.is_loading();
                    let is_playing = audio.is_playing();
                    let audio_volume = audio.get_volume();
                    let hitsound_volume = audio.get_hitsound_volume();

                    shared_for_thread
                        .is_playing
                        .store(is_playing, Ordering::Release);
                    shared_for_thread
                        .is_loading
                        .store(is_loading, Ordering::Release);

                    let overlay_rect_left = shared_for_thread.overlay_rect_left();
                    let overlay_rect_right = shared_for_thread.overlay_rect_right();
                    let selection_left_bbox_hovered =
                        shared_for_thread.selection_left_bbox_hovered();
                    let selection_right_bbox_hovered =
                        shared_for_thread.selection_right_bbox_hovered();
                    let selection_left_bbox_dragging =
                        shared_for_thread.selection_left_bbox_dragging();
                    let selection_right_bbox_dragging =
                        shared_for_thread.selection_right_bbox_dragging();
                    let selection_left_origin_hovered =
                        shared_for_thread.selection_left_origin_hovered();
                    let selection_right_origin_hovered =
                        shared_for_thread.selection_right_origin_hovered();
                    let selection_left_origin_dragging =
                        shared_for_thread.selection_left_origin_dragging();
                    let selection_right_origin_dragging =
                        shared_for_thread.selection_right_origin_dragging();
                    let cursor_pos = shared_for_thread.cursor_pos();
                    let play_pause_button_hovered = shared_for_thread.play_pause_button_hovered();
                    let play_pause_button_clicked = shared_for_thread.play_pause_button_clicked();
                    let undo_button_hovered = shared_for_thread.undo_button_hovered();
                    let undo_button_clicked = shared_for_thread.undo_button_clicked();
                    let current_state_button_hovered =
                        shared_for_thread.current_state_button_hovered();
                    let current_state_button_clicked =
                        shared_for_thread.current_state_button_clicked();
                    let (current_state_rename_active, current_state_rename_text) =
                        shared_for_thread.current_state_rename_state();
                    let redo_button_hovered_row = shared_for_thread.redo_button_hovered_row();
                    let redo_button_clicked_row = shared_for_thread.redo_button_clicked_row();

                    let (
                        left_selected_objects,
                        right_selected_objects,
                        left_bbox,
                        right_bbox,
                        left_origin,
                        right_origin,
                        left_moved,
                        right_moved,
                        left_selection_exists,
                        right_selection_exists,
                        left_selection_scale,
                        right_selection_scale,
                        left_selection_rotation_degrees,
                        right_selection_rotation_degrees,
                        left_selection_origin_locked,
                        right_selection_origin_locked,
                        left_selection_scale_locked,
                        right_selection_scale_locked,
                        left_drag_pos,
                        right_drag_pos,
                    ) = shared_for_thread
                        .edit_state
                        .write()
                        .unwrap()
                        .prepare_for_render(
                            &frame_layout,
                            time_ms,
                            overlay_rect_left,
                            overlay_rect_right,
                            cursor_pos,
                            selection_left_bbox_dragging,
                            selection_right_bbox_dragging,
                            selection_left_origin_dragging,
                            selection_right_origin_dragging,
                        );
                    let (
                        undo_depth,
                        undo_prev_state,
                        undo_prev_state_display_name,
                        undo_current_state,
                        undo_current_state_display_name,
                        undo_next_states,
                        undo_next_state_display_names,
                    ) = {
                        let edit_state_guard = shared_for_thread.edit_state.read().unwrap();
                        let undo_info = edit_state_guard.undo_redo_info_for_hud();

                        let age_value_and_unit = |created_at: std::time::Instant| -> (u32, u32) {
                            let elapsed_secs = created_at.elapsed().as_secs();
                            if elapsed_secs < 60 {
                                (elapsed_secs.min(u32::MAX as u64) as u32, 0)
                            } else {
                                ((elapsed_secs / 60).min(u32::MAX as u64) as u32, 1)
                            }
                        };

                        let undo_prev_state = undo_info.prev_state.as_ref().map(|state| {
                            let (age_value, age_unit) = age_value_and_unit(state.created_at);
                            (state.uuid as u32, age_value, age_unit)
                        });
                        let undo_prev_state_display_name =
                            undo_info.prev_state.as_ref().and_then(|state| state.display_name.clone());
                        let undo_current_state = {
                            let (age_value, age_unit) =
                                age_value_and_unit(undo_info.current_state.created_at);
                            (undo_info.current_state.uuid as u32, age_value, age_unit)
                        };
                        let undo_current_state_display_name =
                            undo_info.current_state.display_name.clone();
                        let undo_next_states: Vec<(u32, u32, u32)> = undo_info
                            .next_states
                            .iter()
                            .map(|state| {
                                let (age_value, age_unit) = age_value_and_unit(state.created_at);
                                (state.uuid as u32, age_value, age_unit)
                            })
                            .collect();
                        let undo_next_state_display_names: Vec<Option<String>> = undo_info
                            .next_states
                            .iter()
                            .map(|state| state.display_name.clone())
                            .collect();

                        (
                            edit_state_guard.undo_depth(),
                            undo_prev_state,
                            undo_prev_state_display_name,
                            undo_current_state,
                            undo_current_state_display_name,
                            undo_next_states,
                            undo_next_state_display_names,
                        )
                    };

                    let selection_rect_left = left_bbox
                        .as_ref()
                        .map(|bbox| bbox4_to_screen_quad(bbox, &frame_layout.playfield_rect));
                    let selection_rect_right = right_bbox
                        .as_ref()
                        .map(|bbox| bbox4_to_screen_quad(bbox, &frame_layout.playfield_rect));
                    let selection_origin_left = left_origin.map(|origin| {
                        let pos = playfield_to_screen(origin, &frame_layout.playfield_rect);
                        [pos.x as f32, pos.y as f32]
                    });
                    let selection_origin_right = right_origin.map(|origin| {
                        let pos = playfield_to_screen(origin, &frame_layout.playfield_rect);
                        [pos.x as f32, pos.y as f32]
                    });
                    let selection_drag_pos_left = left_drag_pos.map(|pos| {
                        let screen = playfield_to_screen(pos, &frame_layout.playfield_rect);
                        [screen.x as f32, screen.y as f32]
                    });
                    let selection_drag_pos_right = right_drag_pos.map(|pos| {
                        let screen = playfield_to_screen(pos, &frame_layout.playfield_rect);
                        [screen.x as f32, screen.y as f32]
                    });
                    let selection_origin_left_playfield =
                        left_origin.map(|origin| [origin.x as f32, origin.y as f32]);
                    let selection_origin_right_playfield =
                        right_origin.map(|origin| [origin.x as f32, origin.y as f32]);
                    let selection_moved_left_playfield = [left_moved.x as f32, left_moved.y as f32];
                    let selection_moved_right_playfield =
                        [right_moved.x as f32, right_moved.y as f32];

                    let selection_dragging =
                        selection_left_bbox_dragging || selection_right_bbox_dragging;
                    let origin_dragging =
                        selection_left_origin_dragging || selection_right_origin_dragging;

                    let (state, snap_positions, movable_snap_positions) = {
                        let edit_state_guard = shared_for_thread.edit_state.read().unwrap();
                        let left_origin_locked = edit_state_guard
                            .left_selection
                            .as_ref()
                            .map(|s| s.origin_locked)
                            .unwrap_or(false);
                        let right_origin_locked = edit_state_guard
                            .right_selection
                            .as_ref()
                            .map(|s| s.origin_locked)
                            .unwrap_or(false);
                        let left_drag_uses_object_stack = selection_left_bbox_dragging
                            && edit_state_guard
                                .left_selection
                                .as_ref()
                                .and_then(|s| s.drag_state.as_ref())
                                .map(|d| d.part_of_object)
                                .unwrap_or(false);
                        let right_drag_uses_object_stack = selection_right_bbox_dragging
                            && edit_state_guard
                                .right_selection
                                .as_ref()
                                .and_then(|s| s.drag_state.as_ref())
                                .map(|d| d.part_of_object)
                                .unwrap_or(false);
                        let left_selection_rotating = selection_left_bbox_dragging
                            && edit_state_guard
                                .left_selection
                                .as_ref()
                                .and_then(|s| s.drag_state.as_ref())
                                .map(|d| d.is_rotation)
                                .unwrap_or(false);
                        let right_selection_rotating = selection_right_bbox_dragging
                            && edit_state_guard
                                .right_selection
                                .as_ref()
                                .and_then(|s| s.drag_state.as_ref())
                                .map(|d| d.is_rotation)
                                .unwrap_or(false);
                        let left_selection_translating =
                            selection_left_bbox_dragging && !left_selection_rotating;
                        let right_selection_translating =
                            selection_right_bbox_dragging && !right_selection_rotating;
                        let show_stacking_offsets =
                            left_drag_uses_object_stack || right_drag_uses_object_stack;
                        let mut static_positions: Vec<Vec2> = Vec::new();
                        let mut movable_positions: Vec<Vec2> = Vec::new();
                        for snap in edit_state_guard.snap_positions.positions.iter() {
                            if !show_stacking_offsets && snap.virtual_stack {
                                continue;
                            }
                            if (selection_left_origin_dragging && snap.is_left_origin)
                                || (selection_right_origin_dragging && snap.is_right_origin)
                            {
                                continue;
                            }

                            if selection_dragging && snap.is_left_origin {
                                if left_selection_rotating
                                    || (left_selection_translating && !left_origin_locked)
                                {
                                    continue;
                                }
                                static_positions.push(snap.pos);
                                continue;
                            }
                            if selection_dragging && snap.is_right_origin {
                                if right_selection_rotating
                                    || (right_selection_translating && !right_origin_locked)
                                {
                                    continue;
                                }
                                static_positions.push(snap.pos);
                                continue;
                            }

                            let left_origin_movable = snap.is_left_origin && selection_left_origin_dragging;
                            let right_origin_movable =
                                snap.is_right_origin && selection_right_origin_dragging;
                            let movable = snap.from_left_sel_and_movable
                                || snap.from_right_sel_and_movable
                                || left_origin_movable
                                || right_origin_movable;
                            if origin_dragging && movable {
                                static_positions.push(snap.pos);
                                continue;
                            }
                            if selection_dragging && movable {
                                continue;
                            }
                            if movable {
                                movable_positions.push(snap.pos);
                            } else {
                                static_positions.push(snap.pos);
                            }
                        }
                        (
                            edit_state_guard.get_latest_export(),
                            static_positions,
                            movable_positions,
                        )
                    };
                    let drag_happening = selection_dragging || origin_dragging;
                    let render_result = gpu.render(
                        &frame_layout,
                        state.objects.iter(),
                        state.combo_colors.as_slice(),
                        state.break_times.iter(),
                        state.kiai_times.iter(),
                        state.bookmarks.iter(),
                        state.red_lines.iter(),
                        left_selected_objects.as_slice(),
                        right_selected_objects.as_slice(),
                        time_ms,
                        song_total_ms,
                        time_elapsed_ms,
                        undo_depth,
                        undo_prev_state,
                        undo_prev_state_display_name,
                        undo_current_state,
                        undo_current_state_display_name,
                        current_state_rename_active,
                        current_state_rename_text.as_str(),
                        undo_next_states.as_slice(),
                        undo_next_state_display_names.as_slice(),
                        fps_current,
                        fps_low,
                        audio.get_speed(),
                        audio_volume,
                        hitsound_volume,
                        &editor_config,
                        is_playing,
                        is_loading,
                        overlay_rect_left,
                        overlay_rect_right,
                        selection_rect_left,
                        selection_rect_right,
                        selection_origin_left,
                        selection_origin_right,
                        selection_drag_pos_left,
                        selection_drag_pos_right,
                        selection_origin_left_playfield,
                        selection_origin_right_playfield,
                        selection_moved_left_playfield,
                        selection_moved_right_playfield,
                        selection_left_bbox_hovered,
                        selection_right_bbox_hovered,
                        selection_left_bbox_dragging,
                        selection_right_bbox_dragging,
                        selection_left_origin_hovered,
                        selection_right_origin_hovered,
                        selection_left_origin_dragging,
                        selection_right_origin_dragging,
                        cursor_pos,
                        play_pause_button_hovered,
                        play_pause_button_clicked,
                        undo_button_hovered,
                        undo_button_clicked,
                        current_state_button_hovered,
                        current_state_button_clicked,
                        redo_button_hovered_row,
                        redo_button_clicked_row,
                        left_selection_exists,
                        right_selection_exists,
                        left_selection_scale,
                        right_selection_scale,
                        left_selection_rotation_degrees,
                        right_selection_rotation_degrees,
                        left_selection_origin_locked,
                        right_selection_origin_locked,
                        left_selection_scale_locked,
                        right_selection_scale_locked,
                        snap_positions.as_slice(),
                        movable_snap_positions.as_slice(),
                        drag_happening,
                        timeline_zoom,
                    );

                    match render_result {
                        Ok(()) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            gpu.resize(winit::dpi::PhysicalSize::new(width, height));
                        }
                        Err(wgpu::SurfaceError::Timeout) => {}
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            shared_for_thread.exit.store(true, Ordering::Release);
                        }
                        Err(wgpu::SurfaceError::Other) => {}
                    }
                }
            })
            .expect("spawn renderer thread");

        Self {
            shared,
            handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        self.shared.exit.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    pub fn mark_resize(&mut self, width: u32, height: u32) {
        self.shared.width.store(width, Ordering::Release);
        self.shared.height.store(height, Ordering::Release);
        self.shared.resize_pending.store(true, Ordering::Release);
    }
}
