use core::f64;

use serde::{Deserialize, Serialize};

use crate::{
    geometry::{
        vec2::{Vec2, segments_intersects},
        vec2_transform::Vec2Transform,
    },
    map_format::slider_curve::PointWithProgress,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BBox {
    pub x: [f64; 2],
    pub y: [f64; 2],
}

impl BBox {
    pub fn contains(&self, point: Vec2) -> bool {
        return point.x >= self.x[0]
            && point.x <= self.x[1]
            && point.y >= self.y[0]
            && point.y <= self.y[1];
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BBox4 {
    pub corners: [Vec2; 4],
}

impl BBox4 {
    pub fn expand(&self, radius: f64) -> Self {
        let mut copy = self.clone();
        for i in 0..4 {
            let next = (i + 1) % 4;
            let prev = (i + 3) % 4;
            let mut edge1 = self.corners[i] - self.corners[next];
            let mut edge2 = self.corners[i] - self.corners[prev];
            edge1 = edge1 * (radius / edge1.len());
            edge2 = edge2 * (radius / edge2.len());
            copy.corners[i] = self.corners[i] + edge1 + edge2;
        }
        return copy;
    }

    pub fn from_bbox(bbox: BBox) -> Self {
        Self {
            corners: [
                Vec2 {
                    x: bbox.x[0],
                    y: bbox.y[0],
                },
                Vec2 {
                    x: bbox.x[1],
                    y: bbox.y[0],
                },
                Vec2 {
                    x: bbox.x[1],
                    y: bbox.y[1],
                },
                Vec2 {
                    x: bbox.x[0],
                    y: bbox.y[1],
                },
            ],
        }
    }

    pub fn center(&self) -> Vec2 {
        return Vec2 {
            x: (self.corners[0].x + self.corners[1].x + self.corners[2].x + self.corners[3].x)
                / 4.0,
            y: (self.corners[0].y + self.corners[1].y + self.corners[2].y + self.corners[3].y)
                / 4.0,
        };
    }

    pub fn to_bbox(&self) -> BBox {
        let mut bbox = BBox {
            x: [f64::INFINITY, f64::NEG_INFINITY],
            y: [f64::INFINITY, f64::NEG_INFINITY],
        };
        for corner in self.corners.iter() {
            bbox.x[0] = bbox.x[0].min(corner.x);
            bbox.x[1] = bbox.x[1].max(corner.x);
            bbox.y[0] = bbox.y[0].min(corner.y);
            bbox.y[1] = bbox.y[1].max(corner.y);
        }
        bbox
    }

    pub fn apply_transform(&self, transform: Vec2Transform) -> Self {
        const BBOX_MIN_EDGE: f64 = 1e-3;
        const BBOX_MIN_EDGE_2: f64 = BBOX_MIN_EDGE * BBOX_MIN_EDGE;
        let corners = self.corners.map(|corner| corner * transform);

        if (corners[1] - corners[0]).len2() < BBOX_MIN_EDGE_2
            || (corners[2] - corners[1]).len2() < BBOX_MIN_EDGE_2
            || (corners[3] - corners[2]).len2() < BBOX_MIN_EDGE_2
            || (corners[0] - corners[3]).len2() < BBOX_MIN_EDGE_2
        {
            let x = [
                corners[0]
                    .x
                    .min(corners[1].x)
                    .min(corners[2].x)
                    .min(corners[3].x)
                    - BBOX_MIN_EDGE,
                corners[0]
                    .x
                    .max(corners[1].x)
                    .max(corners[2].x)
                    .max(corners[3].x)
                    + BBOX_MIN_EDGE,
            ];
            let y = [
                corners[0]
                    .y
                    .min(corners[1].y)
                    .min(corners[2].y)
                    .min(corners[3].y)
                    - BBOX_MIN_EDGE,
                corners[0]
                    .y
                    .max(corners[1].y)
                    .max(corners[2].y)
                    .max(corners[3].y)
                    + BBOX_MIN_EDGE,
            ];
            return BBox4 {
                corners: [
                    Vec2 { x: x[0], y: y[0] },
                    Vec2 { x: x[1], y: y[0] },
                    Vec2 { x: x[1], y: y[1] },
                    Vec2 { x: x[0], y: y[1] },
                ],
            };
        }
        Self { corners }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        let mut has_pos = false;
        let mut has_neg = false;

        for i in 0..4 {
            let a = self.corners[i];
            let b = self.corners[(i + 1) % 4];
            let ab = b - a;
            let ap = point - a;
            let cross = ab.x * ap.y - ab.y * ap.x;
            if cross > 1e-9 {
                has_pos = true;
            } else if cross < -1e-9 {
                has_neg = true;
            }
            if has_pos && has_neg {
                return false;
            }
        }

        true
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SliderBox {
    pub bbox: BBox,
    pub segments: Vec<[PointWithProgress; 2]>,
}

pub fn convert_to_boxes(
    path: &[PointWithProgress],
    effective_radius: f64,
) -> (Vec<SliderBox>, BBox) {
    let bbox = {
        let mut bbox = BBox {
            x: [f64::INFINITY, f64::NEG_INFINITY],
            y: [f64::INFINITY, f64::NEG_INFINITY],
        };
        for p in path.iter() {
            bbox.x[0] = bbox.x[0].min(p.point.x);
            bbox.x[1] = bbox.x[1].max(p.point.x);
            bbox.y[0] = bbox.y[0].min(p.point.y);
            bbox.y[1] = bbox.y[1].max(p.point.y);
        }
        bbox.x[0] -= effective_radius;
        bbox.x[1] += effective_radius;
        bbox.y[0] -= effective_radius;
        bbox.y[1] += effective_radius;
        bbox
    };
    let mut segments: Vec<[PointWithProgress; 2]> = vec![];
    for i in 1..path.len() {
        segments.push((&path[i - 1..=i]).try_into().unwrap());
    }
    let mut result = vec![];
    convert_to_boxes_rec(
        effective_radius * effective_radius,
        SliderBox {
            bbox: bbox.clone(),
            segments,
        },
        &mut result,
    );
    return (result, bbox);
}

pub fn convert_to_boxes_rec(
    effective_radius_2: f64,
    mut bbox: SliderBox,
    result: &mut Vec<SliderBox>,
) {
    if bbox.segments.len() == 0 {
        return;
    }
    let best_distance2 = {
        let mut best_distance2 = f64::INFINITY;
        for segment in bbox.segments.iter() {
            let d2 = max_d2_box_to_segment(bbox.bbox.clone(), [segment[0].point, segment[1].point]);
            if d2 < best_distance2 {
                best_distance2 = d2;
            }
        }
        best_distance2
    };

    bbox.segments = bbox
        .segments
        .iter()
        .filter(|segment| {
            let d2 = d2_box_to_segment(bbox.bbox.clone(), [segment[0].point, segment[1].point]);
            d2 <= effective_radius_2 && d2 <= best_distance2 + 1e-6
        })
        .copied()
        .collect();
    if bbox.segments.is_empty() {
        return;
    }

    let dx = bbox.bbox.x[1] - bbox.bbox.x[0];
    let dy = bbox.bbox.y[1] - bbox.bbox.y[0];
    let area = dx * dy;

    if area <= effective_radius_2
        || (best_distance2 <= effective_radius_2 && bbox.segments.len() <= 2)
    {
        result.push(bbox);
        return;
    }
    let halves = if dx >= dy {
        let mid_x = (bbox.bbox.x[0] + bbox.bbox.x[1]) / 2.0;
        [
            BBox {
                x: [bbox.bbox.x[0], mid_x],
                y: bbox.bbox.y,
            },
            BBox {
                x: [mid_x, bbox.bbox.x[1]],
                y: bbox.bbox.y,
            },
        ]
    } else {
        let mid_y = (bbox.bbox.y[0] + bbox.bbox.y[1]) / 2.0;
        [
            BBox {
                x: bbox.bbox.x,
                y: [bbox.bbox.y[0], mid_y],
            },
            BBox {
                x: bbox.bbox.x,
                y: [mid_y, bbox.bbox.y[1]],
            },
        ]
    };
    for b in halves.iter() {
        convert_to_boxes_rec(
            effective_radius_2,
            SliderBox {
                bbox: b.clone(),
                segments: bbox.segments.clone(),
            },
            result,
        );
    }
}

pub fn d2_box_to_segment(box_bbox: BBox, segment: [Vec2; 2]) -> f64 {
    if box_bbox.contains(segment[0]) || box_bbox.contains(segment[1]) {
        return 0.0;
    }
    let d0 = d2_segment_to_segment(
        [
            Vec2 {
                x: box_bbox.x[0],
                y: box_bbox.y[0],
            },
            Vec2 {
                x: box_bbox.x[1],
                y: box_bbox.y[0],
            },
        ],
        segment,
    );
    let d1 = d2_segment_to_segment(
        [
            Vec2 {
                x: box_bbox.x[1],
                y: box_bbox.y[0],
            },
            Vec2 {
                x: box_bbox.x[1],
                y: box_bbox.y[1],
            },
        ],
        segment,
    );
    let d2 = d2_segment_to_segment(
        [
            Vec2 {
                x: box_bbox.x[1],
                y: box_bbox.y[1],
            },
            Vec2 {
                x: box_bbox.x[0],
                y: box_bbox.y[1],
            },
        ],
        segment,
    );
    let d3 = d2_segment_to_segment(
        [
            Vec2 {
                x: box_bbox.x[0],
                y: box_bbox.y[1],
            },
            Vec2 {
                x: box_bbox.x[0],
                y: box_bbox.y[0],
            },
        ],
        segment,
    );
    return d0.min(d1).min(d2).min(d3);
}

pub fn max_d2_box_to_segment(box_bbox: BBox, segment: [Vec2; 2]) -> f64 {
    let d0 = d2_segment_to_point(
        segment,
        Vec2 {
            x: box_bbox.x[0],
            y: box_bbox.y[0],
        },
    );
    let d1 = d2_segment_to_point(
        segment,
        Vec2 {
            x: box_bbox.x[1],
            y: box_bbox.y[0],
        },
    );
    let d2 = d2_segment_to_point(
        segment,
        Vec2 {
            x: box_bbox.x[1],
            y: box_bbox.y[1],
        },
    );
    let d3 = d2_segment_to_point(
        segment,
        Vec2 {
            x: box_bbox.x[0],
            y: box_bbox.y[1],
        },
    );
    return d0.max(d1).max(d2).max(d3);
}

pub fn d2_segment_to_segment(seg1: [Vec2; 2], seg2: [Vec2; 2]) -> f64 {
    if segments_intersects(seg1, seg2) {
        return 0.0;
    }

    let v00 = seg2[0] - seg1[0];
    let v01 = seg2[1] - seg1[0];
    let v10 = seg2[0] - seg1[1];
    let v11 = seg2[1] - seg1[1];

    let d_point_point = v00.len2().min(v01.len2()).min(v10.len2()).min(v11.len2());
    let d_point_seg1 = d2_segment_to_point(seg1, seg2[0]).min(d2_segment_to_point(seg1, seg2[1]));
    let d_point_seg2 = d2_segment_to_point(seg2, seg1[0]).min(d2_segment_to_point(seg2, seg1[1]));
    return d_point_point.min(d_point_seg1).min(d_point_seg2);
}

pub fn d2_segment_to_point(segment: [Vec2; 2], point: Vec2) -> f64 {
    let seg_vec = segment[1] - segment[0];
    let pt_vec = point - segment[0];
    let seg_len2 = seg_vec.len2();
    if seg_len2 == 0.0 {
        return pt_vec.len2();
    }
    let t = (pt_vec.dot(seg_vec)) / seg_len2;
    if t < 0.0 {
        return pt_vec.len2();
    } else if t > 1.0 {
        let pt_vec_end = point - segment[1];
        return pt_vec_end.len2();
    } else {
        let projection = segment[0] + seg_vec * t;
        let diff = point - projection;
        return diff.len2();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::vec2::Vec2;

    fn bbox(x0: f64, y0: f64, x1: f64, y1: f64) -> BBox {
        BBox {
            x: [x0, x1],
            y: [y0, y1],
        }
    }

    #[test]
    fn d2_box_to_segment_zero_when_endpoint_inside() {
        let b = bbox(0.0, 0.0, 1.0, 1.0);
        let seg = [Vec2 { x: 0.5, y: 0.5 }, Vec2 { x: 2.0, y: 2.0 }];
        let d2 = d2_box_to_segment(b, seg);
        assert_eq!(d2, 0.0);
    }

    #[test]
    fn d2_box_to_segment_zero_when_crossing() {
        let b = bbox(0.0, 0.0, 1.0, 1.0);
        let seg = [Vec2 { x: -1.0, y: 0.5 }, Vec2 { x: 2.0, y: 0.5 }];
        let d2 = d2_box_to_segment(b, seg);
        assert_eq!(d2, 0.0);
    }

    #[test]
    fn d2_box_to_segment_matches_corner_distance() {
        let b = bbox(0.0, 0.0, 1.0, 1.0);
        let seg = [Vec2 { x: 2.0, y: 2.0 }, Vec2 { x: 3.0, y: 2.0 }];
        let d2 = d2_box_to_segment(b, seg);
        let expected = 2.0;
        assert!((d2 - expected).abs() < 1e-9, "d2={d2} expected={expected}");
    }

    #[test]
    fn d2_segment_to_point_zero_on_segment() {
        let seg = [Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 2.0, y: 0.0 }];
        let p = Vec2 { x: 1.0, y: 0.0 };
        let d2 = d2_segment_to_point(seg, p);
        assert!((d2 - 0.0).abs() < 1e-9, "d2={d2}");
    }

    #[test]
    fn d2_segment_to_point_endpoint_distance() {
        let seg = [Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 2.0, y: 0.0 }];
        let p = Vec2 { x: 3.0, y: 4.0 };
        let d2 = d2_segment_to_point(seg, p);
        let expected = (3.0 - 2.0 as f64).powi(2) + (4.0 - 0.0 as f64).powi(2);
        assert!((d2 - expected).abs() < 1e-9, "d2={d2} expected={expected}");
    }

    #[test]
    fn d2_segment_to_segment_zero_when_intersecting() {
        let s1 = [Vec2 { x: -1.0, y: 0.0 }, Vec2 { x: 1.0, y: 0.0 }];
        let s2 = [Vec2 { x: 0.0, y: -1.0 }, Vec2 { x: 0.0, y: 1.0 }];
        let d2 = d2_segment_to_segment(s1, s2);
        assert!((d2 - 0.0).abs() < 1e-9, "d2={d2}");
    }

    #[test]
    fn d2_segment_to_segment_parallel_distance() {
        let s1 = [Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 2.0, y: 0.0 }];
        let s2 = [Vec2 { x: 0.0, y: 3.0 }, Vec2 { x: 2.0, y: 3.0 }];
        let d2 = d2_segment_to_segment(s1, s2);
        let expected = 9.0;
        assert!((d2 - expected).abs() < 1e-9, "d2={d2} expected={expected}");
    }
}
