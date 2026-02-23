use crate::{
    geometry::{vec2::Vec2, vec2_transform::Vec2Transform},
    map_format::slider_boxing::BBox4,
};

use super::drag_state::DragState;

pub struct Selection {
    pub objects: Vec<usize>,
    pub bbox_inner: BBox4,
    pub radius: f64,
    pub bbox_outer: BBox4,
    pub origin: Vec2,
    pub origin_locked: bool,
    pub scale_locked: bool,

    pub orig_center: Vec2,
    pub curr_center: Vec2,
    pub curr_center_plus1: Vec2,

    pub total_scale: f64,
    pub total_rotation_degrees: f64,
    pub moved: Vec2,

    pub drag_state: Option<DragState>,
}

impl Selection {
    pub fn apply_transform(&mut self, transform: Vec2Transform) {
        self.bbox_inner = self.bbox_inner.apply_transform(transform);
        self.bbox_outer = self.bbox_inner.expand(self.radius);
        if !self.origin_locked {
            self.origin = self.origin * transform;
        }
        self.curr_center = self.curr_center * transform;
        self.curr_center_plus1 = self.curr_center_plus1 * transform;
        let one_rotated = self.curr_center_plus1 - self.curr_center;
        self.total_rotation_degrees = one_rotated.arg().to_degrees();
        self.total_scale = one_rotated.len();
        self.moved = self.curr_center - self.orig_center;
        self.drag_state = match &self.drag_state {
            Some(state) => Some(DragState {
                pos: state.pos * transform,
                part_of_object: state.part_of_object,
                is_rotation: state.is_rotation,
            }),
            None => None,
        };
    }
}
