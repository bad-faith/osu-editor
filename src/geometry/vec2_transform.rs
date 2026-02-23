use std::ops::Mul;

use crate::geometry::vec2::Vec2;

#[derive(Clone, Copy)]
pub struct Vec2Transform {
    matrix: [[f64; 3]; 3],
}

impl Mul<Vec2Transform> for Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: Vec2Transform) -> Self::Output {
        let nom_x = rhs.matrix[0][0] * self.x + rhs.matrix[0][1] * self.y + rhs.matrix[0][2];
        let nom_y = rhs.matrix[1][0] * self.x + rhs.matrix[1][1] * self.y + rhs.matrix[1][2];
        Vec2 { x: nom_x, y: nom_y }
    }
}

pub fn merge(left: Vec2Transform, rhs: Vec2Transform) -> Vec2Transform {
    let mut result = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            result[i][j] = rhs.matrix[i][0] * left.matrix[0][j]
                + rhs.matrix[i][1] * left.matrix[1][j]
                + rhs.matrix[i][2] * left.matrix[2][j];
        }
    }
    Vec2Transform { matrix: result }
}

impl Vec2Transform {
    pub fn translate(t: Vec2) -> Self {
        Vec2Transform {
            matrix: [[1.0, 0.0, t.x], [0.0, 1.0, t.y], [0.0, 0.0, 1.0]],
        }
    }

    pub fn multiply_by_complex(c: Vec2) -> Self {
        Vec2Transform {
            matrix: [[c.x, -c.y, 0.0], [c.y, c.x, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    fn flip_around_axis(dir: Vec2) -> Self {
        let r2 = dir.len2();
        Vec2Transform {
            matrix: [
                [
                    2.0 * dir.x * dir.x / r2 - 1.0,
                    2.0 * dir.x * dir.y / r2,
                    0.0,
                ],
                [
                    2.0 * dir.x * dir.y / r2,
                    2.0 * dir.y * dir.y / r2 - 1.0,
                    0.0,
                ],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    pub const fn transpose_1() -> Self {
        Vec2Transform {
            matrix: [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    pub const fn transpose_2() -> Self {
        Vec2Transform {
            matrix: [[0.0, -1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    pub const fn transpose_3() -> Self {
        Vec2Transform {
            matrix: [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    pub const fn transpose_4() -> Self {
        Vec2Transform {
            matrix: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    pub fn flip_around_axis_line(points: [Vec2; 2]) -> Self {
        let dir = points[1] - points[0];
        Vec2Transform::transform_at_origin(Vec2Transform::flip_around_axis(dir), points[0])
    }

    pub fn transform_at_origin(transform: Vec2Transform, origin: Vec2) -> Self {
        merge(
            Vec2Transform::translate(-origin),
            merge(transform, Vec2Transform::translate(origin)),
        )
    }
}
