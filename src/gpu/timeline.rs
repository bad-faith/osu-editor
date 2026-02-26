use std::{
    cmp::{ min},
    collections::HashSet,
};

use crate::{map_format::colors::Color, state::Object, treap::Treap};

pub struct TimelinePoint {
    pub x: f32,
    pub is_selected: u32,

    pub is_object_start: u32,
    pub is_slide_repeat: u32,
    pub is_object_end: u32,
    pub selection_side: u32,

    pub combo_color_and_opacity: [f32; 4],

    pub is_slider_or_spinner: u32,
}

pub struct TimelineBox {
    pub x0: f32,
    pub x1: f32,
    pub points_index: u32,
    pub point_count: u32,
}

pub fn calculate_timeline_points_and_boxes<'a>(
    objects: &Treap<Object>,
    timeline_x0: f64,
    timeline_x_current: f64,
    timeline_x1: f64,
    timeline_ms_per_pixel: f64,
    current_time_ms: f64,
    left_selection: &HashSet<usize>,
    right_selection: &HashSet<usize>,
    circle_radius_px: f64,
    outline_thickness_px: f64,
    combo_colors: &[Color],
) -> (Vec<TimelinePoint>, Vec<TimelineBox>) {
    let radius = circle_radius_px + outline_thickness_px + 1.0;
    let ms_to_x = |time_ms: f64| -> f32 {
        (timeline_x_current + (time_ms - current_time_ms) / timeline_ms_per_pixel) as f32
    };
    let mut points = Vec::new();

    let combo_colors = combo_colors
        .iter()
        .map(|c| {
            [
                (c.r as f64 / 255.0) as f32,
                (c.g as f64 / 255.0) as f32,
                (c.b as f64 / 255.0) as f32,
                1.0,
            ]
        })
        .collect::<Vec<_>>();

    let mut combo_color_index: i64 = 0;

    for (i, object) in objects.iter().enumerate() {
        let combo_info = object.hit_object.combo_info();
        let object = object.instance().unwrap();
        if combo_info.new_combo && !object.is_spinner {
            combo_color_index =
                (combo_color_index + 1 + combo_info.color_skip) % (combo_colors.len() as i64);
        }
        let color = if object.is_spinner {
            [1.0, 1.0, 1.0, 0.5]
        } else {
            combo_colors[combo_color_index as usize]
        };
        let (selected, is_selection_left) = if left_selection.contains(&i) {
            (true, true)
        } else if right_selection.contains(&i) {
            (true, false)
        } else {
            (false, false)
        };
        let is_slider_or_spinner = if object.is_slider || object.is_spinner {
            1
        } else {
            0
        };
        points.push(TimelinePoint {
            x: ms_to_x(object.timeline_start_ms),
            is_selected: if selected { 1 } else { 0 },
            is_object_start: 1,
            is_slide_repeat: 0,
            is_object_end: 0,
            selection_side: if selected {
                if is_selection_left { 1 } else { 2 }
            } else {
                0
            },
            combo_color_and_opacity: color,
            is_slider_or_spinner,
        });
        for repeat_time in &object.timeline_repeat_ms {
            points.push(TimelinePoint {
                x: ms_to_x(*repeat_time),
                is_selected: if selected { 1 } else { 0 },
                is_object_start: 0,
                is_slide_repeat: 1,
                is_object_end: 0,
                selection_side: if selected {
                    if is_selection_left { 1 } else { 2 }
                } else {
                    0
                },
                combo_color_and_opacity: color,
                is_slider_or_spinner,
            });
        }
        points.push(TimelinePoint {
            x: ms_to_x(object.timeline_end_ms),
            is_selected: if selected { 1 } else { 0 },
            is_object_start: 0,
            is_slide_repeat: 0,
            is_object_end: 1,
            selection_side: if selected {
                if is_selection_left { 1 } else { 2 }
            } else {
                0
            },
            combo_color_and_opacity: color,
            is_slider_or_spinner,
        });
    }

    let x_splits = {
        let mut x_splits = Vec::with_capacity(65);
        for i in 1..64 {
            let x = timeline_x0 + i as f64 * (timeline_x1 - timeline_x0) / 64.0;
            x_splits.push(x);
        }
        x_splits.push(timeline_x1);
        x_splits
    };
    let mut boxes = Vec::with_capacity(x_splits.len() - 1);
    let mut min_point = 0;
    let mut max_point = 0;
    for i in 0..x_splits.len() - 1 {
        let x0 = x_splits[i];
        let x1 = x_splits[i + 1];
        while max_point < points.len() && points[max_point].x < (x1 + radius) as f32 {
            max_point += 1;
        }
        while min_point < points.len() && points[min_point].x < (x0 - radius) as f32 {
            min_point += 1;
        }
        // expand by 1
        let (min_point, max_point) = (
            min_point.saturating_sub(1),
            min(max_point + 1, points.len()),
        );
        boxes.push(TimelineBox {
            x0: x0 as f32,
            x1: x1 as f32,
            points_index: min_point as u32,
            point_count: (max_point - min_point) as u32,
        });
    }

    return (points, boxes);
}
