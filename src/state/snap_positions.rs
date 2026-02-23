use crate::geometry::vec2::Vec2;

use super::snap_position::SnapPosition;

pub struct SnapPositions {
    pub positions: Vec<SnapPosition>,
}

impl SnapPositions {
    pub fn new() -> Self {
        Self {
            positions: vec![
                SnapPosition {
                    pos: Vec2 { x: 0.0, y: 0.0 },
                    virtual_stack: false,
                    part_of_object: false,
                    from_left_sel_and_movable: false,
                    from_right_sel_and_movable: false,
                    is_left_origin: false,
                    is_right_origin: false,
                },
                SnapPosition {
                    pos: Vec2 { x: 512.0, y: 0.0 },
                    virtual_stack: false,
                    part_of_object: false,
                    from_left_sel_and_movable: false,
                    from_right_sel_and_movable: false,
                    is_left_origin: false,
                    is_right_origin: false,
                },
                SnapPosition {
                    pos: Vec2 { x: 0.0, y: 384.0 },
                    virtual_stack: false,
                    part_of_object: false,
                    from_left_sel_and_movable: false,
                    from_right_sel_and_movable: false,
                    is_left_origin: false,
                    is_right_origin: false,
                },
                SnapPosition {
                    pos: Vec2 { x: 512.0, y: 384.0 },
                    virtual_stack: false,
                    part_of_object: false,
                    from_left_sel_and_movable: false,
                    from_right_sel_and_movable: false,
                    is_left_origin: false,
                    is_right_origin: false,
                },
                SnapPosition {
                    pos: Vec2 { x: 256.0, y: 192.0 },
                    virtual_stack: false,
                    part_of_object: false,
                    from_left_sel_and_movable: false,
                    from_right_sel_and_movable: false,
                    is_left_origin: false,
                    is_right_origin: false,
                },
                SnapPosition {
                    pos: Vec2 { x: 0.0, y: 192.0 },
                    virtual_stack: false,
                    part_of_object: false,
                    from_left_sel_and_movable: false,
                    from_right_sel_and_movable: false,
                    is_left_origin: false,
                    is_right_origin: false,
                },
                SnapPosition {
                    pos: Vec2 { x: 256.0, y: 0.0 },
                    virtual_stack: false,
                    part_of_object: false,
                    from_left_sel_and_movable: false,
                    from_right_sel_and_movable: false,
                    is_left_origin: false,
                    is_right_origin: false,
                },
                SnapPosition {
                    pos: Vec2 { x: 512.0, y: 192.0 },
                    virtual_stack: false,
                    part_of_object: false,
                    from_left_sel_and_movable: false,
                    from_right_sel_and_movable: false,
                    is_left_origin: false,
                    is_right_origin: false,
                },
                SnapPosition {
                    pos: Vec2 { x: 256.0, y: 384.0 },
                    virtual_stack: false,
                    part_of_object: false,
                    from_left_sel_and_movable: false,
                    from_right_sel_and_movable: false,
                    is_left_origin: false,
                    is_right_origin: false,
                },
            ],
        }
    }
}
