use serde::{Deserialize, Serialize};

// no default values and no aliases, everything is required.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub audio: AudioConfig,
    pub performance: PerformanceConfig,
}

// no default values and no aliases, everything is required.
#[derive(Serialize, Deserialize, Clone)]
pub struct PerformanceConfig{
    pub msaa_samples: u32,
    pub fps_limiter: f64,
    pub prefer_vrr: bool,
}

// no default values and no aliases, everything is required.
#[derive(Serialize, Deserialize, Clone)]
pub struct GeneralConfig{
    pub playfield_scale: f64,
    pub fix_pitch: bool,
    pub speed: f64,
}

// no default values and no aliases, everything is required.
#[derive(Serialize, Deserialize, Clone)]
pub struct AppearanceConfig{
    pub general: AppearanceGeneralConfig,
    pub layout: AppearanceLayoutConfig,
    pub colors: AppearanceColorsConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppearanceGeneralConfig {
    pub skin: String,
    pub use_custom_slider_end_color: bool,
    pub break_time_lightness: f64,
    pub selected_fade_in_opacity_cap: f64,
    pub selected_fade_out_opacity_cap: f64,
    pub selection_color_mix_strength: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppearanceLayoutConfig {
    pub snap_marker_radius_px: f64,
    pub snap_distance_px: f64,
    pub movable_snap_hitbox_radius_px: f64,
    pub drag_state_marker_radius_px: f64,
    pub slider_border_thickness: f64,
    pub slider_outer_thickness: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppearanceColorsConfig {
    pub snap_marker_rgba: [f64; 4],
    pub movable_snap_hitbox_rgba: [f64; 4],
    pub drag_state_marker_rgba: [f64; 4],
    pub slider_end_rgba: [f64; 4],
    pub playfield_rgba: [f64; 4],
    pub playfield_border_rgba: [f64; 4],
    pub gameplay_rgba: [f64; 4],
    pub gameplay_border_rgba: [f64; 4],
    pub outer_rgba: [f64; 4],
    pub slider_ridge_rgba: [f64; 4],
    pub slider_body_rgba: [f64; 4],
    pub offscreen_playfield_tint_rgb: [f64; 3],
    pub offscreen_osu_tint_rgb: [f64; 3],
    pub left_selection_colors: SelectionColors,
    pub right_selection_colors: SelectionColors,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SelectionColors {
    pub drag_rectangle: [f64; 4],
    pub selection_border: [f64; 4],
    pub selection_border_hovered: [f64; 4],
    pub selection_border_dragging: [f64; 4],
    pub selection_tint: [f64; 4],
    pub selection_tint_hovered: [f64; 4],
    pub selection_tint_dragging: [f64; 4],
    pub selection_origin: [f64; 4],
    pub selection_origin_hovered: [f64; 4],
    pub selection_origin_clicked: [f64; 4],
    pub selection_origin_locked: [f64; 4],
    pub selection_combo_color: [f64; 4],
}

// no default values and no aliases, everything is required.
#[derive(Serialize, Deserialize, Clone)]
pub struct AudioConfig{
    pub audio_offset_ms: f64,
    pub hitsounds_offset_ms: f64,
    pub sound_volume: f64,
    pub hitsound_volume: f64,
    pub spacial_audio: f64,
}