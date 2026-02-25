// each group should have a size that's a multiple of 16 bytes, so that we don't accidentally cause GPU drivers to insert padding in the middle of our data. 
// If we expect padding, add a separate type to fill the gap.
struct Globals {
    // Screen size in pixels: (width, height)
    screen_size: vec2<f32>,
    time_ms: f32,
    slider_border_thickness: f32,

    // (x0, y0, x1, y1) in screen pixels of the 512x384 playfield
    playfield_rect: vec4<f32>,

    // (x0, y0, x1, y1) in screen pixels of the 640x480 osu! coordinate system
    osu_rect: vec4<f32>,

    playfield_rgba: vec4<f32>,
    gameplay_rgba: vec4<f32>,
    outer_rgba: vec4<f32>,
    playfield_border_rgba: vec4<f32>,
    gameplay_border_rgba: vec4<f32>,
    slider_ridge_rgba: vec4<f32>,
    slider_body_rgba: vec4<f32>,

    break_time_lightness : f32,
    is_kiai_time: u32,
    is_break_time: u32,
    slider_progress: f32,

    slider_position: vec2<f32>,
    slider_ball_rotation_index: i32,
    slider_border_outer_thickness: f32,

    slider_color: vec3<f32>,
    fps_x10: u32,

    slider_radius: f32,
    slider_follow_circle_scaling: f32,
    slider_ball_direction: vec2<f32>,

    _pad0: vec3<u32>,
    fps_low_x10: u32,

    // HUD
    song_total_ms: f32,
    playback_rate: f32,
    hud_opacity: f32,
    is_playing: u32,
    
    time_elapsed_ms: f32,
    loading: u32,
    break_time: vec2<f32>,
    spinner_time: vec2<f32>,
    spinner_state: u32,
    undo_count: u32,
    undo_redo_info: vec4<u32>,
    undo_prev_state_info: vec4<u32>,
    undo_current_state_info: vec4<u32>,
    undo_next_states_uuid_0: vec4<u32>,
    undo_next_states_uuid_1: vec4<u32>,
    undo_next_states_age_0: vec4<u32>,
    undo_next_states_age_1: vec4<u32>,
    undo_next_states_age_unit_0: vec4<u32>,
    undo_next_states_age_unit_1: vec4<u32>,
    undo_prev_state_name_meta: vec4<u32>,
    undo_prev_state_name_packed: vec4<u32>,
    undo_next_states_name_len_0: vec4<u32>,
    undo_next_states_name_len_1: vec4<u32>,
    undo_next_states_name_packed: array<vec4<u32>, 8>,
    undo_button_meta: vec4<u32>,
    current_state_button_meta: vec4<u32>,
    current_state_name_meta: vec4<u32>,
    current_state_name_text_0: vec4<u32>,
    current_state_name_text_1: vec4<u32>,
    current_state_name_text_2: vec4<u32>,
    current_state_name_text_3: vec4<u32>,
    current_state_name_text_4: vec4<u32>,
    current_state_name_text_5: vec4<u32>,
    current_state_name_text_6: vec4<u32>,
    current_state_name_text_7: vec4<u32>,
    redo_buttons_meta: vec4<u32>,

    top_timeline_rect: vec4<f32>,
    top_timeline_hitbox_rect: vec4<f32>,
    top_timeline_second_rect: vec4<f32>,
    top_timeline_second_hitbox_rect: vec4<f32>,
    top_timeline_third_rect: vec4<f32>,
    top_timeline_third_hitbox_rect: vec4<f32>,

    timeline_rect: vec4<f32>,
    timeline_hitbox_rect: vec4<f32>,
    play_pause_button_rect: vec4<f32>,
    stats_box_rect: vec4<f32>,
    play_pause_button_meta: vec4<u32>,

    overlay_rect_left: vec4<f32>,
    overlay_rect_right: vec4<f32>,
    selection_quad_left_01: vec4<f32>,
    selection_quad_left_23: vec4<f32>,
    selection_quad_right_01: vec4<f32>,
    selection_quad_right_23: vec4<f32>,
    selection_origin_left: vec4<f32>,
    selection_origin_right: vec4<f32>,
    selection_drag_pos_left: vec4<f32>,
    selection_drag_pos_right: vec4<f32>,
    left_selection_colors: array<vec4<f32>, 12>,
    right_selection_colors: array<vec4<f32>, 12>,
    overlay_meta: vec4<u32>,
    selection_meta: vec4<u32>,
    spinner_selection_meta: vec4<u32>,

    kiai_interval_count: u32,
    break_interval_count: u32,
    bookmark_count: u32,
    red_line_count: u32,

    audio_volume: f32,
    hitsound_volume: f32,
    cpu_pass_x10: u32,
    gpu_pass_x10: u32,
    cursor_pos: vec2<f32>,
    selected_fade_in_opacity_cap: f32,
    selected_fade_out_opacity_cap: f32,
    selection_color_mix_strength: f32,
    selection_left_scale: f32,
    selection_right_scale: f32,
    selection_left_rotation_degrees: f32,
    selection_right_rotation_degrees: f32,
    selection_exists_meta: vec4<u32>,
    selection_origin_left_playfield: vec2<f32>,
    selection_origin_right_playfield: vec2<f32>,
    selection_moved_left_playfield: vec2<f32>,
    selection_moved_right_playfield: vec2<f32>,
    selection_lock_meta: vec4<u32>,
    selection_box_dragging_meta: vec4<u32>,
    snap_marker_rgba: vec4<f32>,
    snap_marker_style: vec4<f32>,
    movable_snap_marker_rgba: vec4<f32>,
    movable_snap_marker_style: vec4<f32>,
    snap_meta: vec4<u32>,
    drag_state_marker_rgba: vec4<f32>,
    drag_state_marker_style: vec4<f32>,
    offscreen_playfield_tint_rgba: vec4<f32>,
    offscreen_osu_tint_rgba: vec4<f32>,

    timeline_window_ms: vec2<f32>,
    timeline_current_x: f32,
    timeline_zoom: f32,
    timeline_object_meta: vec4<u32>,
    timeline_style: vec4<f32>,
    timeline_slider_outline_rgba: vec4<f32>,
    timeline_slider_head_body_rgba: vec4<f32>,
    timeline_slider_head_overlay_rgba: vec4<f32>,
    timeline_circle_head_body_rgba: vec4<f32>,
    timeline_circle_head_overlay_rgba: vec4<f32>,
    timeline_past_grayscale_strength: f32,
    _timeline_past_pad: vec3<f32>,
    timeline_past_tint_rgba: vec4<f32>,
    timeline_past_object_tint_rgba: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

@group(1) @binding(0)
var skin_samp: sampler;

@group(1) @binding(1)
var hitcircle_tex: texture_2d<f32>;

@group(1) @binding(2)
var hitcircle_overlay_tex: texture_2d<f32>;

@group(1) @binding(3)
var slidercircle_tex: texture_2d<f32>;

@group(1) @binding(4)
var slidercircle_overlay_tex: texture_2d<f32>;

@group(1) @binding(5)
var approach_circle_tex: texture_2d<f32>;

@group(1) @binding(6)
var numbers_tex: texture_2d_array<f32>;

@group(1) @binding(7)
var<uniform> digits_meta: DigitsMeta;

@group(1) @binding(8)
var background_tex: texture_2d<f32>;

@group(1) @binding(9)
var<uniform> skin_meta: SkinMeta;

@group(1) @binding(10)
var sliderendcircle_tex: texture_2d<f32>;

@group(1) @binding(11)
var sliderendcircle_overlay_tex: texture_2d<f32>;

@group(1) @binding(12)
var reverse_arrow_tex: texture_2d<f32>;

@group(1) @binding(13)
var sliderball_tex: texture_2d_array<f32>;

@group(1) @binding(14)
var sliderfollowcircle_tex: texture_2d_array<f32>;

@group(1) @binding(15)
var loading_tex: texture_2d<f32>;

@group(1) @binding(16)
var break_tex: texture_2d<f32>;

@group(1) @binding(17)
var spinner_tex: texture_2d<f32>;

@group(3) @binding(3)
var<storage, read> timeline_marks: array<vec2<f32>>;

@group(2) @binding(4)
var<storage, read> snap_positions: array<vec2<f32>>;

struct TimelineSnakeGPU {
    start_end_ms: vec2<f32>,
    center_y: f32,
    radius_px: f32,
    point_start: u32,
    point_count: u32,
    _pad0: vec2<u32>,
    color: vec4<f32>,
};

struct TimelinePointGPU {
    time_ms: f32,
    center_y: f32,
    radius_mult: f32,
    point_kind: u32,
    color: vec4<f32>,
};

@group(2) @binding(5)
var<storage, read> timeline_snakes: array<TimelineSnakeGPU>;

@group(2) @binding(6)
var<storage, read> timeline_points: array<TimelinePointGPU>;

struct DigitsMeta {
    // uv' = uv * scale + offset; stored as vec4(scale.xy, offset.zw)
    uv_xform: array<vec4<f32>, 10>,
    // (max_w, max_h) in pixels
    max_size_px: vec2<f32>,
    _pad: vec2<f32>,
};

struct SkinMeta {
    // Scale factor relative to nominal 128px (or 256px for @2x), e.g. 192px 1x => 1.5.
    hitcircle_scale: f32,
    hitcircleoverlay_scale: f32,
    sliderstartcircle_scale: f32,
    sliderstartcircleoverlay_scale: f32,

    sliderendcircle_scale: f32,
    sliderendcircleoverlay_scale: f32,
    reversearrow_scale: f32,
    sliderball_scale: f32,

    sliderfollowcircle_scale: f32,
    _pad: vec2<f32>,
};

struct CircleGPU {
    center_xy: vec2<f32>,
    radius: f32,
    time_ms: f32,

    color: vec3<f32>,
    preempt_ms: f32,

    approach_circle_start_scale: f32,
    approach_circle_end_scale: f32,
    combo: u32,
    is_slider: u32,

    slider_box_start: u32,
    slider_box_count: u32,
    slider_end_center_xy: vec2<f32>,

    slider_start_border_color: array<u32, 3>,
    slider_length_duration_ms: f32,

    slider_end_border_color: array<u32, 3>,
    slider_end_time_ms: f32,

    slides: u32,
    selected_side: u32,
    _pad1: array<u32, 2>,

    slider_head_rotation: vec2<f32>,
    slider_end_rotation: vec2<f32>,
};

@group(2) @binding(0)
var<storage, read> circles: array<CircleGPU>;

@group(3) @binding(0)
var<storage, read> slider_segs: array<SliderSeg>;

@group(3) @binding(1)
var<storage, read> slider_boxes: array<SliderBox>;

// Maps a slider-draw instance index -> object index into `circles`.
@group(3) @binding(2)
var<storage, read> slider_draw_indices: array<u32>;

struct SliderSeg {
    ridge0: vec4<f32>,
    ridge1: vec4<f32>,
};

struct SliderBox {
    bbox_min: vec2<f32>,
    bbox_max: vec2<f32>,
    seg_start: u32,
    seg_count: u32,
    obj_iid: u32,
    _pad: u32,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) @interpolate(flat) combo: u32,
    @location(3) @interpolate(flat) time_ms: f32,
    @location(4) @interpolate(flat) preempt_ms: f32,
    @location(5) @interpolate(flat) is_slider: u32,
    @location(6) @interpolate(flat) approach_start: f32,
    @location(7) @interpolate(flat) approach_end: f32,
    @location(8) @interpolate(flat) selected_side: u32,
    @location(9) screen_px: vec2<f32>,
    @location(10) @interpolate(flat) center_screen_px: vec2<f32>,
};

struct SliderVsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) pf_pos: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) @interpolate(flat) time_ms: f32,
    @location(3) @interpolate(flat) preempt_ms: f32,
    @location(4) @interpolate(flat) slider_end_time_ms: f32,
    @location(5) @interpolate(flat) radius: f32,
    @location(6) @interpolate(flat) seg_start: u32,
    @location(7) @interpolate(flat) seg_count: u32,
    @location(8) @interpolate(flat) obj_iid: u32,
    @location(9) @interpolate(flat) bbox_min: vec2<f32>,
    @location(10) @interpolate(flat) bbox_max: vec2<f32>,
};

struct SliderCapsVsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) pf_pos: vec2<f32>,
    @location(1) @interpolate(flat) obj_iid: u32,
};

struct BgOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct HudOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
