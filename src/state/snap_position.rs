use crate::geometry::vec2::Vec2;

pub struct SnapPosition {
    pub pos: Vec2,
    pub virtual_stack: bool,
    pub part_of_object: bool,
    pub from_left_sel_and_movable: bool,
    pub from_right_sel_and_movable: bool,
    pub is_left_origin: bool,
    pub is_right_origin: bool,
}
