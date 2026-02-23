use std::{
    fmt::Display,
    hash::{Hash, Hasher},
    ops::{Add, Mul, Neg, Sub},
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Hash for Vec2 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}

impl Eq for Vec2 {}

impl Display for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Vec2 {
    pub fn dot(self, rhs: Vec2) -> f64 {
        self.x * rhs.x + self.y * rhs.y
    }
    pub fn cross(self, rhs: Vec2) -> f64 {
        self.x * rhs.y - self.y * rhs.x
    }
    pub fn len2(self) -> f64 {
        self.x * self.x + self.y * self.y
    }
    pub fn len(self) -> f64 {
        self.len2().sqrt()
    }
    pub fn arg(self) -> f64 {
        self.y.atan2(self.x)
    }
    pub fn conjugate(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: -self.y,
        }
    }
    pub fn mul_complex(self, rhs: Vec2) -> Vec2 {
        Vec2 {
            x: self.x * rhs.x - self.y * rhs.y,
            y: self.x * rhs.y + self.y * rhs.x,
        }
    }
    pub fn div_complex(self, rhs: Vec2) -> Vec2 {
        let denom = rhs.x * rhs.x + rhs.y * rhs.y;
        Vec2 {
            x: (self.x * rhs.x + self.y * rhs.y) / denom,
            y: (self.y * rhs.x - self.x * rhs.y) / denom,
        }
    }
    pub fn div_complex_normalized(self, rhs: Vec2) -> Vec2 {
        self.mul_complex(rhs.conjugate()).normalize()
    }
    pub fn normalize(self) -> Vec2 {
        let len = self.len();
        Vec2 {
            x: self.x / len,
            y: self.y / len,
        }
    }
    pub fn distance(self, rhs: Vec2) -> f64 {
        (self - rhs).len()
    }
    pub fn distance2(self, rhs: Vec2) -> f64 {
        (self - rhs).len2()
    }
}

impl Neg for Vec2 {
    type Output = Vec2;

    fn neg(self) -> Self::Output {
        Vec2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Mul<f64> for Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: f64) -> Self::Output {
        Vec2 {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Add for Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Vec2) -> Self::Output {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

pub fn segments_intersects(seg1: [Vec2; 2], seg2: [Vec2; 2]) -> bool {
    let d1 = (seg2[1] - seg2[0]).cross(seg1[0] - seg2[0]);
    let d2 = (seg2[1] - seg2[0]).cross(seg1[1] - seg2[0]);
    let d3 = (seg1[1] - seg1[0]).cross(seg2[0] - seg1[0]);
    let d4 = (seg1[1] - seg1[0]).cross(seg2[1] - seg1[0]);
    if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
        return true;
    }
    false
}

pub fn circle_center(p: [Vec2; 3]) -> Vec2 {
    let mid1 = (p[0] + p[1]) * 0.5;
    let mid2 = (p[1] + p[2]) * 0.5;

    let dir1 = Vec2 {
        x: -(p[1].y - p[0].y),
        y: p[1].x - p[0].x,
    };
    let dir2 = Vec2 {
        x: -(p[2].y - p[1].y),
        y: p[2].x - p[1].x,
    };

    let a1 = dir1.y;
    let b1 = -dir1.x;
    let c1 = a1 * mid1.x + b1 * mid1.y;

    let a2 = dir2.y;
    let b2 = -dir2.x;
    let c2 = a2 * mid2.x + b2 * mid2.y;

    let det = a1 * b2 - a2 * b1;
    if det.abs() < 1e-10 {
        return Vec2 { x: 0.0, y: 0.0 }; // Points are collinear; return origin as fallback
    }

    let cx = (b2 * c1 - b1 * c2) / det;
    let cy = (a1 * c2 - a2 * c1) / det;

    Vec2 { x: cx, y: cy }
}
