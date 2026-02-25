use bytemuck::{Pod, Zeroable};

use crate::geometry::vec2::{Vec2};
use crate::map_format::slider_boxing::BBox;
use crate::map_format::slider_curve::{ SliderCurveWithBoxes};

pub const MAX_KIAI_INTERVALS: usize = 1024;
pub const MAX_BREAK_INTERVALS: usize = 1024;
pub const MAX_BOOKMARKS: usize = 1024;
pub const MAX_RED_LINES: usize = 1024;
pub const MAX_TIMELINE_MARKS: usize = MAX_BOOKMARKS + MAX_RED_LINES;
pub const MAX_SNAP_MARKERS: usize = 8192;
pub const MAX_TIMELINE_SNAKES: usize = 4096;
pub const MAX_TIMELINE_POINTS: usize = 8192;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Globals {
    // Screen size in pixels: (width, height)
    pub screen_size: [f32; 2],
    // Editor time in milliseconds
    pub time_ms: f32,
    pub slider_border_thickness: f32,

    // (x0, y0, x1, y1) in screen pixels of the 512x384 playfield
    pub playfield_rect: [f32; 4],

    // (x0, y0, x1, y1) in screen pixels of the 640x480 osu! coordinate system,
    pub osu_rect: [f32; 4],

    pub playfield_rgba: [f32; 4],
    pub gameplay_rgba: [f32; 4],
    pub outer_rgba: [f32; 4],
    pub playfield_border_rgba: [f32; 4],
    pub gameplay_border_rgba: [f32; 4],
    pub slider_ridge_rgba: [f32; 4],
    pub slider_body_rgba: [f32; 4],

    pub break_time_lightness: f32,
    pub is_kiai_time: u32,
    pub is_break_time: u32,
    pub slider_progress: f32,

    pub slider_position: [f32; 2],
    pub slider_ball_rotation_index: i32,
    pub slider_border_outer_thickness: f32,

    pub slider_color: [f32; 3],
    pub fps_x10: u32,

    pub slider_radius: f32,
    pub slider_follow_circle_scaling: f32,
    pub slider_ball_direction: [f32; 2],
    pub _pad0: [u32; 3],
    pub fps_low_x10: u32,

    // HUD
    pub song_total_ms: f32,
    pub playback_rate: f32,
    pub hud_opacity: f32,
    pub is_playing: u32,

    pub time_elapsed_ms: f32,
    pub loading: u32,
    pub break_time: [f32; 2],
    pub spinner_time: [f32; 2],
    pub spinner_state: u32,
    pub undo_count: u32,
    pub undo_redo_info: [u32; 4],
    pub undo_prev_state_info: [u32; 4],
    pub undo_current_state_info: [u32; 4],
    pub undo_next_states_uuid_0: [u32; 4],
    pub undo_next_states_uuid_1: [u32; 4],
    pub undo_next_states_age_0: [u32; 4],
    pub undo_next_states_age_1: [u32; 4],
    pub undo_next_states_age_unit_0: [u32; 4],
    pub undo_next_states_age_unit_1: [u32; 4],
    pub undo_prev_state_name_meta: [u32; 4],
    pub undo_prev_state_name_packed: [u32; 4],
    pub undo_next_states_name_len_0: [u32; 4],
    pub undo_next_states_name_len_1: [u32; 4],
    pub undo_next_states_name_packed: [[u32; 4]; 8],
    pub undo_button_meta: [u32; 4],
    pub current_state_button_meta: [u32; 4],
    pub current_state_name_meta: [u32; 4],
    pub current_state_name_text_0: [u32; 4],
    pub current_state_name_text_1: [u32; 4],
    pub current_state_name_text_2: [u32; 4],
    pub current_state_name_text_3: [u32; 4],
    pub current_state_name_text_4: [u32; 4],
    pub current_state_name_text_5: [u32; 4],
    pub current_state_name_text_6: [u32; 4],
    pub current_state_name_text_7: [u32; 4],
    pub redo_buttons_meta: [u32; 4],

    pub top_timeline_rect: [f32; 4],

    pub top_timeline_hitbox_rect: [f32; 4],

    pub top_timeline_second_rect: [f32; 4],

    pub top_timeline_second_hitbox_rect: [f32; 4],

    pub top_timeline_third_rect: [f32; 4],

    pub top_timeline_third_hitbox_rect: [f32; 4],

    pub timeline_rect: [f32; 4],

    pub timeline_hitbox_rect: [f32; 4],

    pub play_pause_button_rect: [f32; 4],

    pub stats_box_rect: [f32; 4],

    pub play_pause_button_meta: [u32; 4],

    pub overlay_rect_left: [f32; 4],
    pub overlay_rect_right: [f32; 4],
    pub selection_quad_left_01: [f32; 4],
    pub selection_quad_left_23: [f32; 4],
    pub selection_quad_right_01: [f32; 4],
    pub selection_quad_right_23: [f32; 4],
    pub selection_origin_left: [f32; 4],
    pub selection_origin_right: [f32; 4],
    pub selection_drag_pos_left: [f32; 4],
    pub selection_drag_pos_right: [f32; 4],

    pub left_selection_colors: [[f32; 4]; 12],
    pub right_selection_colors: [[f32; 4]; 12],

    pub overlay_meta: [u32; 4],
    pub selection_meta: [u32; 4],
    pub spinner_selection_meta: [u32; 4],

    pub kiai_interval_count: u32,
    pub break_interval_count: u32,
    pub bookmark_count: u32,
    pub red_line_count: u32,

    pub audio_volume: f32,
    pub hitsound_volume: f32,
    pub cpu_pass_x10: u32,
    pub gpu_pass_x10: u32,
    pub cursor_pos: [f32; 2],
    pub selected_fade_in_opacity_cap: f32,
    pub selected_fade_out_opacity_cap: f32,
    pub selection_color_mix_strength: f32,
    pub selection_left_scale: f32,
    pub selection_right_scale: f32,
    pub selection_left_rotation_degrees: f32,
    pub selection_right_rotation_degrees: f32,
    pub _pad4: [u32; 3],
    pub selection_exists_meta: [u32; 4],
    pub selection_origin_left_playfield: [f32; 2],
    pub selection_origin_right_playfield: [f32; 2],
    pub selection_moved_left_playfield: [f32; 2],
    pub selection_moved_right_playfield: [f32; 2],
    pub selection_lock_meta: [u32; 4],
    pub selection_box_dragging_meta: [u32; 4],
    pub snap_marker_rgba: [f32; 4],
    pub snap_marker_style: [f32; 4],
    pub movable_snap_marker_rgba: [f32; 4],
    pub movable_snap_marker_style: [f32; 4],
    pub snap_meta: [u32; 4],
    pub drag_state_marker_rgba: [f32; 4],
    pub drag_state_marker_style: [f32; 4],
    pub offscreen_playfield_tint_rgba: [f32; 4],
    pub offscreen_osu_tint_rgba: [f32; 4],

    pub timeline_window_ms: [f32; 2],
    pub timeline_current_x: f32,
    pub timeline_zoom: f32,
    pub timeline_object_meta: [u32; 4],
    pub timeline_style: [f32; 4],
    pub timeline_slider_outline_rgba: [f32; 4],
    pub timeline_slider_head_body_rgba: [f32; 4],
    pub timeline_slider_head_overlay_rgba: [f32; 4],
    pub timeline_circle_head_body_rgba: [f32; 4],
    pub timeline_circle_head_overlay_rgba: [f32; 4],
    pub timeline_past_grayscale_strength: f32,
    pub _timeline_past_pad: [f32; 3],
    pub timeline_past_tint_rgba: [f32; 4],
    pub timeline_past_object_tint_rgba: [f32; 4],
    pub _pad_end: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TimelineSnakeGpu {
    pub start_end_ms: [f32; 2],
    pub center_y: f32,
    pub radius_px: f32,
    pub point_start: u32,
    pub point_count: u32,
    pub _pad0: [u32; 2],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TimelinePointGpu {
    pub time_ms: f32,
    pub center_y: f32,
    pub radius_mult: f32,
    pub point_kind: u32,
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct DigitsMeta {
    // uv' = uv * scale + offset; stored as vec4(scale.xy, offset.zw)
    pub uv_xform: [[f32; 4]; 10],
    // Maximum digit atlas layer size (pixels): (max_w, max_h)
    pub max_size_px: [f32; 2],
    pub _pad: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct SkinMeta {
    // Scale factor relative to nominal 128px (or 256px for @2x):
    // e.g. 192px 1x => 1.5
    pub hitcircle_scale: f32,
    pub hitcircleoverlay_scale: f32,
    pub sliderstartcircle_scale: f32,
    pub sliderstartcircleoverlay_scale: f32,

    pub sliderendcircle_scale: f32,
    pub sliderendcircleoverlay_scale: f32,
    pub reversearrow_scale: f32,
    pub sliderball_scale: f32,

    pub sliderfollowcircle_scale: f32,
    pub _pad0: f32,
    pub _pad: [f32; 2],
}

pub const MAX_CIRCLES: usize = 8192;
// Slider body rendering loops over path segments per pixel within the slider bbox.
// Keep a reasonable per-object cap to avoid pathological shader workloads.
pub const INITIAL_SLIDER_SEGS_CAPACITY: usize = 32768;
pub const INITIAL_SLIDER_BOXES_CAPACITY: usize = 8192;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct SliderSegGpu {
    // All positions are in playfield coordinates (512x384).
    pub ridge0: [f32; 4],
    pub ridge1: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct SliderBoxGpu {
    // All positions are in playfield coordinates (512x384).
    pub bbox_min: [f32; 2],
    pub bbox_max: [f32; 2],
    pub seg_start: u32,
    pub seg_count: u32,
    pub obj_iid: u32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CircleGpu {
    pub center_xy: [f32; 2],
    pub radius: f32,
    pub time_ms: f32,

    pub color: [f32; 3],
    pub preempt_ms: f32,

    pub approach_circle_start_scale: f32,
    pub approach_circle_end_scale: f32,
    pub combo: u32,
    pub is_slider: u32,

    // Slider box lookup into `slider_boxes` buffer (group(3)).
    pub slider_box_start: u32,
    pub slider_box_count: u32,
    pub slider_end_center_xy: [f32; 2],

    pub slider_start_border_color: [u32; 3],
    pub slider_length_duration_ms: f32,

    pub slider_end_border_color: [u32; 3],
    pub slider_end_time_ms: f32,

    pub slides: u32,
    pub selected_side: u32,
    pub _pad1: [u32; 2],

    pub slider_head_rotation: [f32; 2],
    pub slider_end_rotation: [f32; 2],
}

impl CircleGpu {
    pub fn from_instance(
        instance: &ObjectInstance,
        combo: u32,
        color: [f32; 3],
        slider_start_border_color: [u32; 3],
        slider_end_border_color: [u32; 3],
    ) -> Self {
        CircleGpu {
            center_xy: [instance.pos.x as f32, instance.pos.y as f32],
            radius: instance.radius as f32,
            time_ms: instance.time as f32,

            color,
            preempt_ms: instance.preempt as f32,

            approach_circle_start_scale: 4.0,
            approach_circle_end_scale: 1.0,
            combo,
            is_slider: if instance.is_slider { 1 } else { 0 },

            slider_box_start: 0,
            slider_box_count: 0,
            slider_end_center_xy: [0.0, 0.0],
            slider_start_border_color,
            slider_length_duration_ms: instance.slider_slide_duration_ms as f32,
            slider_end_border_color,
            slider_end_time_ms: instance.slider_end_time_ms as f32,
            slides: instance.slides as u32,
            selected_side: 0,
            slider_head_rotation: [1.0, 0.0],
            _pad1: [0, 0],
            slider_end_rotation: [1.0, 0.0],
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjectInstance {
    pub pos: Vec2,
    pub time: f64,
    pub radius: f64,
    pub preempt: f64,
    pub is_new_combo: bool,
    pub is_slider: bool,
    pub is_spinner: bool,
    pub slider_path: Option<SliderCurveWithBoxes>,
    pub slider_length_px: f64,
    pub slider_slide_duration_ms: f64,
    pub slider_end_time_ms: f64,
    pub slides: u64,
    pub bbox_inner: BBox,
    pub snap_points: Vec<Vec2>,
    pub timeline_start_ms: f64,
    pub timeline_end_ms: f64,
    pub timeline_repeat_ms: Vec<f64>,
}

impl ObjectInstance {
    pub fn end_pos(&self) -> Vec2 {
        if self.is_slider {
            let path = self.slider_path.as_ref().unwrap();
            let end_pos = path.ridge.ridge.last().unwrap();
            end_pos.point
        } else {
            self.pos
        }
    }
    pub fn get_bbox(&self) -> Option<BBox> {
        if self.is_slider {
            self.slider_path.as_ref().map(|curve| curve.bbox.clone())
        } else {
            Some(BBox {
                x: [self.pos.x - self.radius, self.pos.x + self.radius],
                y: [self.pos.y - self.radius, self.pos.y + self.radius],
            })
        }
    }
    pub fn sample_position_and_progress_and_direction(
        &self,
        timestamp: f64,
    ) -> (Vec2, f64, Vec2) {
        let mut time_into_slider = timestamp - self.time;
        let loop_duration = self.slider_slide_duration_ms * 2.0;
        if time_into_slider >= loop_duration {
            let loops = (time_into_slider / loop_duration).floor() as u32;
            time_into_slider -= loops as f64 * loop_duration;
        }

        let progress = if time_into_slider <= self.slider_slide_duration_ms {
            time_into_slider / self.slider_slide_duration_ms
        } else {
            1.0 - (time_into_slider - self.slider_slide_duration_ms) / self.slider_slide_duration_ms
        };
        let expected_length = progress * self.slider_length_px;
        let curve = self.slider_path.as_ref().unwrap();

        let (pos, direction, fully_used) = curve
            .ridge
            .get_position_and_direction_at_length(expected_length);
        if fully_used {
            (pos, 1.0, direction)
        } else {
            (pos, progress, direction)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CircleGpu, DigitsMeta, Globals, SkinMeta, SliderBoxGpu, SliderSegGpu};

    #[derive(Clone, Debug)]
    enum WgslType {
        Scalar,
        Vec(u32),
        Array(Box<WgslType>, usize),
    }

    #[derive(Clone, Debug)]
    struct WgslField {
        name: String,
        ty: WgslType,
    }

    #[derive(Clone, Copy, Debug)]
    enum AddrSpace {
        Uniform,
        Storage,
    }

    fn round_up(value: usize, align: usize) -> usize {
        if align == 0 {
            return value;
        }
        value.div_ceil(align) * align
    }

    fn parse_wgsl_type(raw: &str) -> WgslType {
        let ty = raw.trim();
        if ty == "f32" || ty == "u32" || ty == "i32" {
            return WgslType::Scalar;
        }
        if ty.starts_with("vec") {
            let n = ty
                .chars()
                .nth(3)
                .and_then(|ch| ch.to_digit(10))
                .expect("vector width");
            return WgslType::Vec(n);
        }
        if let Some(inner) = ty.strip_prefix("array<").and_then(|s| s.strip_suffix('>')) {
            let (elem_raw, len_raw) = inner
                .split_once(',')
                .expect("array type must contain element type and length");
            let elem_ty = parse_wgsl_type(elem_raw.trim());
            let len = len_raw.trim().parse::<usize>().expect("array length");
            return WgslType::Array(Box::new(elem_ty), len);
        }
        panic!("unsupported WGSL type in Globals: {ty}");
    }

    fn wgsl_align_and_size(ty: &WgslType, addr_space: AddrSpace) -> (usize, usize) {
        match ty {
            WgslType::Scalar => (4, 4),
            WgslType::Vec(2) => (8, 8),
            WgslType::Vec(3) => (16, 12),
            WgslType::Vec(4) => (16, 16),
            WgslType::Vec(n) => panic!("unsupported vec width: {n}"),
            WgslType::Array(elem, len) => {
                let (elem_align, elem_size) = wgsl_align_and_size(elem, addr_space);
                let (array_align, stride) = match addr_space {
                    AddrSpace::Uniform => (16, round_up(elem_size, 16)),
                    AddrSpace::Storage => (elem_align, round_up(elem_size, elem_align)),
                };
                (array_align, stride * len)
            }
        }
    }

    fn extract_struct_fields(wgsl_src: &str, struct_name: &str) -> Vec<WgslField> {
        let marker = format!("struct {struct_name}");
        let start = wgsl_src
            .find(&marker)
            .unwrap_or_else(|| panic!("struct {struct_name} must exist in 00_defs.wgsl"));
        let body = &wgsl_src[start..];
        let open = body.find('{').expect("struct opening brace");
        let after_open = &body[(open + 1)..];

        let mut out = Vec::new();
        for line in after_open.lines() {
            let trimmed = line.split("//").next().unwrap_or("").trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("};") {
                break;
            }

            let no_comma = trimmed.trim_end_matches(',').trim();
            let (name_raw, ty_raw) = no_comma
                .split_once(':')
                .unwrap_or_else(|| panic!("invalid {struct_name} field line: {trimmed}"));
            out.push(WgslField {
                name: name_raw.trim().to_string(),
                ty: parse_wgsl_type(ty_raw.trim()),
            });
        }
        out
    }

    fn validate_wgsl_struct_and_simulate_layout(
        wgsl_src: &str,
        struct_name: &str,
        addr_space: AddrSpace,
    ) -> (Vec<(String, usize)>, usize) {
        let fields = extract_struct_fields(wgsl_src, struct_name);
        let mut offsets = Vec::with_capacity(fields.len());
        let mut offset: usize = 0;
        let mut max_align: usize = 1;

        for field in fields {
            let (align, size) = wgsl_align_and_size(&field.ty, addr_space);
            offset = round_up(offset, align);
            max_align = max_align.max(align);

            if let WgslType::Array(elem, _) = &field.ty {
                let (_, elem_size) = wgsl_align_and_size(elem, addr_space);
                let stride = match addr_space {
                    AddrSpace::Uniform => round_up(elem_size, 16),
                    AddrSpace::Storage => {
                        let (elem_align, _) = wgsl_align_and_size(elem, addr_space);
                        round_up(elem_size, elem_align)
                    }
                };

                if matches!(addr_space, AddrSpace::Uniform) {
                    assert_eq!(
                        offset % 16,
                        0,
                        "WGSL struct invalid: array field `{}` in {} does not start at 16-byte aligned offset {}",
                        field.name,
                        struct_name,
                        offset
                    );
                    assert_eq!(
                        stride % 16,
                        0,
                        "WGSL struct invalid: array field `{}` in {} has non-16-byte stride {}",
                        field.name,
                        struct_name,
                        stride
                    );
                }
            }

            offsets.push((field.name, offset));
            offset += size;
        }

        let size = match addr_space {
            AddrSpace::Uniform => round_up(offset, 16),
            AddrSpace::Storage => round_up(offset, max_align),
        };

        (offsets, size)
    }

    fn extract_globals_groups(wgsl_src: &str) -> Vec<Vec<WgslField>> {
        let start = wgsl_src
            .find("struct Globals")
            .expect("struct Globals must exist in 00_defs.wgsl");
        let body = &wgsl_src[start..];
        let open = body.find('{').expect("Globals opening brace");
        let after_open = &body[(open + 1)..];

        let mut groups: Vec<Vec<WgslField>> = Vec::new();
        let mut current_group: Vec<WgslField> = Vec::new();

        let flush_group = |group: &mut Vec<WgslField>, groups: &mut Vec<Vec<WgslField>>| {
            if !group.is_empty() {
                groups.push(std::mem::take(group));
            }
        };

        for line in after_open.lines() {
            let trimmed = line.split("//").next().unwrap_or("").trim();
            if trimmed.is_empty() {
                flush_group(&mut current_group, &mut groups);
                continue;
            }
            if trimmed.starts_with("};") {
                flush_group(&mut current_group, &mut groups);
                break;
            }

            let no_comma = trimmed.trim_end_matches(',').trim();
            let (name_raw, ty_raw) = no_comma
                .split_once(':')
                .unwrap_or_else(|| panic!("invalid Globals field line: {trimmed}"));
            current_group.push(WgslField {
                name: name_raw.trim().to_string(),
                ty: parse_wgsl_type(ty_raw.trim()),
            });
        }
        groups
    }

    fn validate_wgsl_globals_and_simulate_layout(wgsl_src: &str) -> (Vec<(String, usize)>, usize) {
        let groups = extract_globals_groups(wgsl_src);
        let total_fields: usize = groups.iter().map(|group| group.len()).sum();
        let mut offsets = Vec::with_capacity(total_fields);
        let mut offset: usize = 0;

        for group in groups {
            let mut group_start: Option<usize> = None;
            let mut group_max_align: usize = 1;

            for field in group {
                let (align, size) = wgsl_align_and_size(&field.ty, AddrSpace::Uniform);
                offset = round_up(offset, align);

                if group_start.is_none() {
                    group_start = Some(offset);
                }
                group_max_align = group_max_align.max(align);

                if let WgslType::Array(elem, _) = &field.ty {
                    let (_, elem_size) = wgsl_align_and_size(elem, AddrSpace::Uniform);
                    let stride = round_up(elem_size, 16);
                    assert_eq!(
                        offset % 16,
                        0,
                        "WGSL struct invalid: array field `{}` does not start at 16-byte aligned offset {}",
                        field.name,
                        offset
                    );
                    assert_eq!(
                        stride % 16,
                        0,
                        "WGSL struct invalid: array field `{}` has non-16-byte stride {}",
                        field.name,
                        stride
                    );
                }

                offsets.push((field.name, offset));
                offset += size;
            }

            if let Some(group_start) = group_start {
                let group_end = offset;
                assert_eq!(
                    group_start % group_max_align,
                    0,
                    "WGSL group invalid: group start offset {} is not aligned to group alignment {}",
                    group_start,
                    group_max_align
                );
                assert_eq!(
                    group_end % group_max_align,
                    0,
                    "WGSL group invalid: group end offset {} is not aligned to group alignment {}",
                    group_end,
                    group_max_align
                );
            }
        }

        (offsets, round_up(offset, 16))
    }

    fn rust_globals_layout() -> (Vec<(&'static str, usize)>, usize) {
        let fields = vec![
            ("screen_size", std::mem::offset_of!(Globals, screen_size)),
            ("time_ms", std::mem::offset_of!(Globals, time_ms)),
            (
                "slider_border_thickness",
                std::mem::offset_of!(Globals, slider_border_thickness),
            ),
            (
                "playfield_rect",
                std::mem::offset_of!(Globals, playfield_rect),
            ),
            ("osu_rect", std::mem::offset_of!(Globals, osu_rect)),
            (
                "playfield_rgba",
                std::mem::offset_of!(Globals, playfield_rgba),
            ),
            (
                "gameplay_rgba",
                std::mem::offset_of!(Globals, gameplay_rgba),
            ),
            ("outer_rgba", std::mem::offset_of!(Globals, outer_rgba)),
            (
                "playfield_border_rgba",
                std::mem::offset_of!(Globals, playfield_border_rgba),
            ),
            (
                "gameplay_border_rgba",
                std::mem::offset_of!(Globals, gameplay_border_rgba),
            ),
            (
                "slider_ridge_rgba",
                std::mem::offset_of!(Globals, slider_ridge_rgba),
            ),
            (
                "slider_body_rgba",
                std::mem::offset_of!(Globals, slider_body_rgba),
            ),
            (
                "break_time_lightness",
                std::mem::offset_of!(Globals, break_time_lightness),
            ),
            ("is_kiai_time", std::mem::offset_of!(Globals, is_kiai_time)),
            (
                "is_break_time",
                std::mem::offset_of!(Globals, is_break_time),
            ),
            (
                "slider_progress",
                std::mem::offset_of!(Globals, slider_progress),
            ),
            (
                "slider_position",
                std::mem::offset_of!(Globals, slider_position),
            ),
            (
                "slider_ball_rotation_index",
                std::mem::offset_of!(Globals, slider_ball_rotation_index),
            ),
            (
                "slider_border_outer_thickness",
                std::mem::offset_of!(Globals, slider_border_outer_thickness),
            ),
            ("slider_color", std::mem::offset_of!(Globals, slider_color)),
            ("fps_x10", std::mem::offset_of!(Globals, fps_x10)),
            (
                "slider_radius",
                std::mem::offset_of!(Globals, slider_radius),
            ),
            (
                "slider_follow_circle_scaling",
                std::mem::offset_of!(Globals, slider_follow_circle_scaling),
            ),
            (
                "slider_ball_direction",
                std::mem::offset_of!(Globals, slider_ball_direction),
            ),
            ("_pad0", std::mem::offset_of!(Globals, _pad0)),
            ("fps_low_x10", std::mem::offset_of!(Globals, fps_low_x10)),
            (
                "song_total_ms",
                std::mem::offset_of!(Globals, song_total_ms),
            ),
            (
                "playback_rate",
                std::mem::offset_of!(Globals, playback_rate),
            ),
            ("hud_opacity", std::mem::offset_of!(Globals, hud_opacity)),
            ("is_playing", std::mem::offset_of!(Globals, is_playing)),
            (
                "time_elapsed_ms",
                std::mem::offset_of!(Globals, time_elapsed_ms),
            ),
            ("loading", std::mem::offset_of!(Globals, loading)),
            ("break_time", std::mem::offset_of!(Globals, break_time)),
            ("spinner_time", std::mem::offset_of!(Globals, spinner_time)),
            (
                "spinner_state",
                std::mem::offset_of!(Globals, spinner_state),
            ),
            ("undo_count", std::mem::offset_of!(Globals, undo_count)),
            (
                "undo_redo_info",
                std::mem::offset_of!(Globals, undo_redo_info),
            ),
            (
                "undo_prev_state_info",
                std::mem::offset_of!(Globals, undo_prev_state_info),
            ),
            (
                "undo_current_state_info",
                std::mem::offset_of!(Globals, undo_current_state_info),
            ),
            (
                "undo_next_states_uuid_0",
                std::mem::offset_of!(Globals, undo_next_states_uuid_0),
            ),
            (
                "undo_next_states_uuid_1",
                std::mem::offset_of!(Globals, undo_next_states_uuid_1),
            ),
            (
                "undo_next_states_age_0",
                std::mem::offset_of!(Globals, undo_next_states_age_0),
            ),
            (
                "undo_next_states_age_1",
                std::mem::offset_of!(Globals, undo_next_states_age_1),
            ),
            (
                "undo_next_states_age_unit_0",
                std::mem::offset_of!(Globals, undo_next_states_age_unit_0),
            ),
            (
                "undo_next_states_age_unit_1",
                std::mem::offset_of!(Globals, undo_next_states_age_unit_1),
            ),
            (
                "undo_button_meta",
                std::mem::offset_of!(Globals, undo_button_meta),
            ),
            (
                "redo_buttons_meta",
                std::mem::offset_of!(Globals, redo_buttons_meta),
            ),
            (
                "top_timeline_rect",
                std::mem::offset_of!(Globals, top_timeline_rect),
            ),
            (
                "top_timeline_hitbox_rect",
                std::mem::offset_of!(Globals, top_timeline_hitbox_rect),
            ),
            (
                "top_timeline_second_rect",
                std::mem::offset_of!(Globals, top_timeline_second_rect),
            ),
            (
                "top_timeline_second_hitbox_rect",
                std::mem::offset_of!(Globals, top_timeline_second_hitbox_rect),
            ),
            (
                "top_timeline_third_rect",
                std::mem::offset_of!(Globals, top_timeline_third_rect),
            ),
            (
                "top_timeline_third_hitbox_rect",
                std::mem::offset_of!(Globals, top_timeline_third_hitbox_rect),
            ),
            (
                "timeline_rect",
                std::mem::offset_of!(Globals, timeline_rect),
            ),
            (
                "timeline_hitbox_rect",
                std::mem::offset_of!(Globals, timeline_hitbox_rect),
            ),
            (
                "play_pause_button_rect",
                std::mem::offset_of!(Globals, play_pause_button_rect),
            ),
            (
                "stats_box_rect",
                std::mem::offset_of!(Globals, stats_box_rect),
            ),
            (
                "play_pause_button_meta",
                std::mem::offset_of!(Globals, play_pause_button_meta),
            ),
            (
                "overlay_rect_left",
                std::mem::offset_of!(Globals, overlay_rect_left),
            ),
            (
                "overlay_rect_right",
                std::mem::offset_of!(Globals, overlay_rect_right),
            ),
            (
                "selection_quad_left_01",
                std::mem::offset_of!(Globals, selection_quad_left_01),
            ),
            (
                "selection_quad_left_23",
                std::mem::offset_of!(Globals, selection_quad_left_23),
            ),
            (
                "selection_quad_right_01",
                std::mem::offset_of!(Globals, selection_quad_right_01),
            ),
            (
                "selection_quad_right_23",
                std::mem::offset_of!(Globals, selection_quad_right_23),
            ),
            (
                "selection_origin_left",
                std::mem::offset_of!(Globals, selection_origin_left),
            ),
            (
                "selection_origin_right",
                std::mem::offset_of!(Globals, selection_origin_right),
            ),
            (
                "left_selection_colors",
                std::mem::offset_of!(Globals, left_selection_colors),
            ),
            (
                "right_selection_colors",
                std::mem::offset_of!(Globals, right_selection_colors),
            ),
            ("overlay_meta", std::mem::offset_of!(Globals, overlay_meta)),
            (
                "selection_meta",
                std::mem::offset_of!(Globals, selection_meta),
            ),
            (
                "spinner_selection_meta",
                std::mem::offset_of!(Globals, spinner_selection_meta),
            ),
            (
                "kiai_interval_count",
                std::mem::offset_of!(Globals, kiai_interval_count),
            ),
            (
                "break_interval_count",
                std::mem::offset_of!(Globals, break_interval_count),
            ),
            (
                "bookmark_count",
                std::mem::offset_of!(Globals, bookmark_count),
            ),
            (
                "red_line_count",
                std::mem::offset_of!(Globals, red_line_count),
            ),
            ("audio_volume", std::mem::offset_of!(Globals, audio_volume)),
            (
                "hitsound_volume",
                std::mem::offset_of!(Globals, hitsound_volume),
            ),
            ("cpu_pass_x10", std::mem::offset_of!(Globals, cpu_pass_x10)),
            ("gpu_pass_x10", std::mem::offset_of!(Globals, gpu_pass_x10)),
            ("cursor_pos", std::mem::offset_of!(Globals, cursor_pos)),
            (
                "selected_fade_in_opacity_cap",
                std::mem::offset_of!(Globals, selected_fade_in_opacity_cap),
            ),
            (
                "selected_fade_out_opacity_cap",
                std::mem::offset_of!(Globals, selected_fade_out_opacity_cap),
            ),
            (
                "selection_color_mix_strength",
                std::mem::offset_of!(Globals, selection_color_mix_strength),
            ),
            (
                "selection_left_scale",
                std::mem::offset_of!(Globals, selection_left_scale),
            ),
            (
                "selection_right_scale",
                std::mem::offset_of!(Globals, selection_right_scale),
            ),
            (
                "selection_left_rotation_degrees",
                std::mem::offset_of!(Globals, selection_left_rotation_degrees),
            ),
            (
                "selection_right_rotation_degrees",
                std::mem::offset_of!(Globals, selection_right_rotation_degrees),
            ),
            (
                "selection_exists_meta",
                std::mem::offset_of!(Globals, selection_exists_meta),
            ),
            (
                "selection_origin_left_playfield",
                std::mem::offset_of!(Globals, selection_origin_left_playfield),
            ),
            (
                "selection_origin_right_playfield",
                std::mem::offset_of!(Globals, selection_origin_right_playfield),
            ),
            (
                "selection_moved_left_playfield",
                std::mem::offset_of!(Globals, selection_moved_left_playfield),
            ),
            (
                "selection_moved_right_playfield",
                std::mem::offset_of!(Globals, selection_moved_right_playfield),
            ),
            (
                "selection_lock_meta",
                std::mem::offset_of!(Globals, selection_lock_meta),
            ),
            (
                "selection_box_dragging_meta",
                std::mem::offset_of!(Globals, selection_box_dragging_meta),
            ),
            (
                "offscreen_playfield_tint_rgba",
                std::mem::offset_of!(Globals, offscreen_playfield_tint_rgba),
            ),
            (
                "offscreen_osu_tint_rgba",
                std::mem::offset_of!(Globals, offscreen_osu_tint_rgba),
            ),
        ];
        (fields, std::mem::size_of::<Globals>())
    }

    fn rust_digits_meta_layout() -> (Vec<(&'static str, usize)>, usize) {
        (
            vec![
                ("uv_xform", std::mem::offset_of!(DigitsMeta, uv_xform)),
                ("max_size_px", std::mem::offset_of!(DigitsMeta, max_size_px)),
                ("_pad", std::mem::offset_of!(DigitsMeta, _pad)),
            ],
            std::mem::size_of::<DigitsMeta>(),
        )
    }

    fn rust_skin_meta_layout() -> (Vec<(&'static str, usize)>, usize) {
        (
            vec![
                (
                    "hitcircle_scale",
                    std::mem::offset_of!(SkinMeta, hitcircle_scale),
                ),
                (
                    "hitcircleoverlay_scale",
                    std::mem::offset_of!(SkinMeta, hitcircleoverlay_scale),
                ),
                (
                    "sliderstartcircle_scale",
                    std::mem::offset_of!(SkinMeta, sliderstartcircle_scale),
                ),
                (
                    "sliderstartcircleoverlay_scale",
                    std::mem::offset_of!(SkinMeta, sliderstartcircleoverlay_scale),
                ),
                (
                    "sliderendcircle_scale",
                    std::mem::offset_of!(SkinMeta, sliderendcircle_scale),
                ),
                (
                    "sliderendcircleoverlay_scale",
                    std::mem::offset_of!(SkinMeta, sliderendcircleoverlay_scale),
                ),
                (
                    "reversearrow_scale",
                    std::mem::offset_of!(SkinMeta, reversearrow_scale),
                ),
                ("_pad", std::mem::offset_of!(SkinMeta, _pad)),
            ],
            std::mem::size_of::<SkinMeta>(),
        )
    }

    fn rust_circle_gpu_layout() -> (Vec<(&'static str, usize)>, usize) {
        (
            vec![
                ("center_xy", std::mem::offset_of!(CircleGpu, center_xy)),
                ("radius", std::mem::offset_of!(CircleGpu, radius)),
                ("time_ms", std::mem::offset_of!(CircleGpu, time_ms)),
                ("color", std::mem::offset_of!(CircleGpu, color)),
                ("preempt_ms", std::mem::offset_of!(CircleGpu, preempt_ms)),
                (
                    "approach_circle_start_scale",
                    std::mem::offset_of!(CircleGpu, approach_circle_start_scale),
                ),
                (
                    "approach_circle_end_scale",
                    std::mem::offset_of!(CircleGpu, approach_circle_end_scale),
                ),
                ("combo", std::mem::offset_of!(CircleGpu, combo)),
                ("is_slider", std::mem::offset_of!(CircleGpu, is_slider)),
                (
                    "slider_box_start",
                    std::mem::offset_of!(CircleGpu, slider_box_start),
                ),
                (
                    "slider_box_count",
                    std::mem::offset_of!(CircleGpu, slider_box_count),
                ),
                (
                    "slider_end_center_xy",
                    std::mem::offset_of!(CircleGpu, slider_end_center_xy),
                ),
                (
                    "slider_start_border_color",
                    std::mem::offset_of!(CircleGpu, slider_start_border_color),
                ),
                (
                    "slider_length_duration_ms",
                    std::mem::offset_of!(CircleGpu, slider_length_duration_ms),
                ),
                (
                    "slider_end_border_color",
                    std::mem::offset_of!(CircleGpu, slider_end_border_color),
                ),
                (
                    "slider_end_time_ms",
                    std::mem::offset_of!(CircleGpu, slider_end_time_ms),
                ),
                ("slides", std::mem::offset_of!(CircleGpu, slides)),
                (
                    "selected_side",
                    std::mem::offset_of!(CircleGpu, selected_side),
                ),
                ("_pad1", std::mem::offset_of!(CircleGpu, _pad1)),
                (
                    "slider_head_rotation",
                    std::mem::offset_of!(CircleGpu, slider_head_rotation),
                ),
                (
                    "slider_end_rotation",
                    std::mem::offset_of!(CircleGpu, slider_end_rotation),
                ),
            ],
            std::mem::size_of::<CircleGpu>(),
        )
    }

    fn rust_slider_seg_gpu_layout() -> (Vec<(&'static str, usize)>, usize) {
        (
            vec![
                ("ridge0", std::mem::offset_of!(SliderSegGpu, ridge0)),
                ("ridge1", std::mem::offset_of!(SliderSegGpu, ridge1)),
            ],
            std::mem::size_of::<SliderSegGpu>(),
        )
    }

    fn rust_slider_box_gpu_layout() -> (Vec<(&'static str, usize)>, usize) {
        (
            vec![
                ("bbox_min", std::mem::offset_of!(SliderBoxGpu, bbox_min)),
                ("bbox_max", std::mem::offset_of!(SliderBoxGpu, bbox_max)),
                ("seg_start", std::mem::offset_of!(SliderBoxGpu, seg_start)),
                ("seg_count", std::mem::offset_of!(SliderBoxGpu, seg_count)),
                ("obj_iid", std::mem::offset_of!(SliderBoxGpu, obj_iid)),
                ("_pad", std::mem::offset_of!(SliderBoxGpu, _pad)),
            ],
            std::mem::size_of::<SliderBoxGpu>(),
        )
    }

    fn assert_wgsl_rust_layout_match(
        wgsl_struct_name: &str,
        addr_space: AddrSpace,
        rust_offsets: Vec<(&'static str, usize)>,
        rust_size: usize,
    ) {
        let wgsl_src = include_str!("shaders/00_defs.wgsl");
        let (wgsl_offsets, wgsl_size) =
            validate_wgsl_struct_and_simulate_layout(wgsl_src, wgsl_struct_name, addr_space);

        assert_eq!(
            rust_offsets.len(),
            wgsl_offsets.len(),
            "{} field count mismatch between Rust and WGSL",
            wgsl_struct_name
        );

        for (index, ((rust_name, _), (wgsl_name, _))) in
            rust_offsets.iter().zip(wgsl_offsets.iter()).enumerate()
        {
            assert_eq!(
                rust_name, wgsl_name,
                "{} field mismatch at index {index}: Rust=`{rust_name}`, WGSL=`{wgsl_name}`",
                wgsl_struct_name
            );
        }

        for ((name, rust_offset), (_, wgsl_offset)) in rust_offsets.iter().zip(wgsl_offsets.iter())
        {
            assert_eq!(
                *rust_offset, *wgsl_offset,
                "offset mismatch for {} field `{name}`",
                wgsl_struct_name
            );
        }

        assert_eq!(
            rust_size, wgsl_size,
            "{} size mismatch: Rust={rust_size}, WGSL={wgsl_size}",
            wgsl_struct_name
        );
    }

    #[test]
    fn globals_wgsl_is_valid() {
        let wgsl_src = include_str!("shaders/00_defs.wgsl");
        let (_wgsl_offsets, _wgsl_size) = validate_wgsl_globals_and_simulate_layout(wgsl_src);
    }

    #[test]
    fn globals_wgsl_matches_rust_layout() {
        let wgsl_src = include_str!("shaders/00_defs.wgsl");
        let (wgsl_offsets, wgsl_size) = validate_wgsl_globals_and_simulate_layout(wgsl_src);
        let (rust_offsets, rust_size) = rust_globals_layout();

        assert_eq!(
            rust_offsets.len(),
            wgsl_offsets.len(),
            "Globals field count mismatch between Rust and WGSL"
        );

        for (index, ((rust_name, _), (wgsl_name, _))) in
            rust_offsets.iter().zip(wgsl_offsets.iter()).enumerate()
        {
            assert_eq!(
                rust_name, wgsl_name,
                "Globals field mismatch at index {index}: Rust=`{rust_name}`, WGSL=`{wgsl_name}`"
            );
        }

        for ((name, rust_offset), (_, wgsl_offset)) in rust_offsets.iter().zip(wgsl_offsets.iter())
        {
            assert_eq!(
                *rust_offset, *wgsl_offset,
                "offset mismatch for Globals field `{name}`"
            );
        }

        assert_eq!(
            rust_size, wgsl_size,
            "Globals size mismatch: Rust={rust_size}, WGSL={wgsl_size}"
        );
    }

    #[test]
    fn digits_meta_matches_wgsl_layout() {
        let (rust_offsets, rust_size) = rust_digits_meta_layout();
        assert_wgsl_rust_layout_match("DigitsMeta", AddrSpace::Uniform, rust_offsets, rust_size);
    }

    #[test]
    fn skin_meta_matches_wgsl_layout() {
        let (rust_offsets, rust_size) = rust_skin_meta_layout();
        assert_wgsl_rust_layout_match("SkinMeta", AddrSpace::Uniform, rust_offsets, rust_size);
    }

    #[test]
    fn circle_gpu_matches_wgsl_layout() {
        let (rust_offsets, rust_size) = rust_circle_gpu_layout();
        assert_wgsl_rust_layout_match("CircleGPU", AddrSpace::Storage, rust_offsets, rust_size);
    }

    #[test]
    fn slider_seg_matches_wgsl_layout() {
        let (rust_offsets, rust_size) = rust_slider_seg_gpu_layout();
        assert_wgsl_rust_layout_match("SliderSeg", AddrSpace::Storage, rust_offsets, rust_size);
    }

    #[test]
    fn slider_box_matches_wgsl_layout() {
        let (rust_offsets, rust_size) = rust_slider_box_gpu_layout();
        assert_wgsl_rust_layout_match("SliderBox", AddrSpace::Storage, rust_offsets, rust_size);
    }
}
