use crate::geometry::vec2::Vec2;

pub struct DragState {
    pub pos: Vec2,
    pub part_of_object: bool,
    pub is_rotation: bool,
}
