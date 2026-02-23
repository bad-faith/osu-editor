use serde::{Deserialize, Serialize};

use crate::{
    geometry::{
        vec2::{Vec2, circle_center},
        vec2_transform::Vec2Transform,
    },
    map_format::slider_boxing::{BBox, SliderBox, convert_to_boxes},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ControlPoints {
    pub start: Vec2,
    pub slider_segments: Vec<ControlPointSegment>,
}

impl ControlPoints {
    pub fn size(&self) -> f64 {
        let mut total = 0.0;
        let mut start = self.start;
        for segment in &self.slider_segments {
            match segment {
                ControlPointSegment::Bezier(points, end) => {
                    for point in points {
                        total += (*point - start).len();
                        start = *point;
                    }
                    total += (*end - start).len();
                    start = *end;
                }
                ControlPointSegment::Linear(points, end) => {
                    for point in points {
                        total += (*point - start).len();
                        start = *point;
                    }
                    total += (*end - start).len();
                    start = *end;
                }
                ControlPointSegment::PerfectCircle(points) => {
                    total += (points[0] - start).len();
                    total += (points[1] - points[0]).len();
                    start = points[1];
                }
                ControlPointSegment::Catmull(points, end) => {
                    for point in points {
                        total += (*point - start).len();
                        start = *point;
                    }
                    total += (*end - start).len();
                    start = *end;
                }
            }
        }
        return total;
    }

    pub fn move_by_offset(&self, offset: Vec2) -> Self {
        ControlPoints {
            start: self.start + offset,
            slider_segments: self
                .slider_segments
                .iter()
                .map(|seg| seg.move_by_offset(offset))
                .collect(),
        }
    }

    pub fn apply_transform(&self, transform: Vec2Transform) -> Self {
        ControlPoints {
            start: self.start * transform,
            slider_segments: self
                .slider_segments
                .iter()
                .map(|seg| seg.apply_transform(transform))
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ControlPointSegment {
    Bezier(Vec<Vec2>, Vec2),
    Linear(Vec<Vec2>, Vec2),
    PerfectCircle([Vec2; 2]),
    Catmull(Vec<Vec2>, Vec2),
}

impl ControlPointSegment {
    pub fn apply_transform(&self, transform: Vec2Transform) -> Self {
        match self {
            ControlPointSegment::Bezier(points, end) => ControlPointSegment::Bezier(
                points.iter().map(|p| *p * transform).collect(),
                *end * transform,
            ),
            ControlPointSegment::Linear(points, end) => ControlPointSegment::Linear(
                points.iter().map(|p| *p * transform).collect(),
                *end * transform,
            ),
            ControlPointSegment::PerfectCircle(points) => {
                ControlPointSegment::PerfectCircle([points[0] * transform, points[1] * transform])
            }
            ControlPointSegment::Catmull(points, end) => ControlPointSegment::Catmull(
                points.iter().map(|p| *p * transform).collect(),
                *end * transform,
            ),
        }
    }

    pub fn move_by_offset(&self, offset: Vec2) -> Self {
        match self {
            ControlPointSegment::Bezier(points, end) => ControlPointSegment::Bezier(
                points.iter().map(|p| *p + offset).collect(),
                *end + offset,
            ),
            ControlPointSegment::Linear(points, end) => ControlPointSegment::Linear(
                points.iter().map(|p| *p + offset).collect(),
                *end + offset,
            ),
            ControlPointSegment::PerfectCircle(points) => {
                ControlPointSegment::PerfectCircle([points[0] + offset, points[1] + offset])
            }
            ControlPointSegment::Catmull(points, end) => ControlPointSegment::Catmull(
                points.iter().map(|p| *p + offset).collect(),
                *end + offset,
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SliderCurveRidge {
    pub ridge: Vec<PointWithProgress>,
}

impl SliderCurveRidge {
    pub fn calculate_bbox_inner(&self) -> BBox {
        let mut bbox = BBox {
            x: [std::f64::INFINITY, std::f64::NEG_INFINITY],
            y: [std::f64::INFINITY, std::f64::NEG_INFINITY],
        };
        for point in &self.ridge {
            bbox.x[0] = bbox.x[0].min(point.point.x);
            bbox.x[1] = bbox.x[1].max(point.point.x);
            bbox.y[0] = bbox.y[0].min(point.point.y);
            bbox.y[1] = bbox.y[1].max(point.point.y);
        }
        return bbox;
    }
    pub fn start_point(&self) -> Vec2 {
        if self.ridge.is_empty() {
            Vec2 { x: 0.0, y: 0.0 }
        } else {
            self.ridge[0].point
        }
    }
    pub fn start_rotation(&self) -> Vec2 {
        if self.ridge.len() < 2 {
            Vec2 { x: 1.0, y: 0.0 }
        } else {
            let dir = self.ridge[1].point - self.ridge[0].point;
            dir.normalize()
        }
    }
    pub fn end_point(&self) -> Vec2 {
        if self.ridge.is_empty() {
            Vec2 { x: 0.0, y: 0.0 }
        } else {
            self.ridge.last().unwrap().point
        }
    }
    pub fn end_rotation(&self) -> Vec2 {
        let len = self.ridge.len();
        if len < 2 {
            Vec2 { x: -1.0, y: 0.0 }
        } else {
            let dir = self.ridge[len - 2].point - self.ridge[len - 1].point;
            dir.normalize()
        }
    }
    pub fn get_position_and_direction_at_length(&self, length: f64) -> (Vec2, Vec2, bool) {
        let mut total_distance = 0.0;
        for i in 1..self.ridge.len() {
            let p1 = &self.ridge[i];
            let p0 = &self.ridge[i - 1];
            let dist = (p1.point - p0.point).len();
            if total_distance + dist >= length {
                let segment_progress = (length - total_distance) / dist;
                let pos = p0.point + (p1.point - p0.point) * segment_progress;
                return (pos, (p1.point - p0.point).normalize(), false);
            } else {
                total_distance += dist;
            }
        }
        if self.ridge.len() == 1 {
            let only_point = self.ridge[0].point;
            (only_point, Vec2 { x: 1.0, y: 0.0 }, true)
        } else {
            let p0 = self.ridge[self.ridge.len() - 2].point;
            let p1 = self.ridge[self.ridge.len() - 1].point;
            (p1, (p1 - p0).normalize(), true)
        }
    }

    pub fn construct_boxes(
        &self,
        radius: f64,
        slider_outer_thickness: f64,
    ) -> SliderCurveWithBoxes {
        let effective_radius = radius * (1.0 + slider_outer_thickness) + 1.0; // +1.0 for rounding errors
        let (boxes, bbox) = convert_to_boxes(&self.ridge, effective_radius);
        return SliderCurveWithBoxes {
            ridge: self.clone(),
            boxes,
            bbox,
        };
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SliderCurveWithBoxes {
    pub ridge: SliderCurveRidge,
    pub boxes: Vec<SliderBox>,
    pub bbox: BBox,
}

impl ControlPoints {
    pub fn new(start: Vec2, slider_segments: Vec<ControlPointSegment>) -> Self {
        ControlPoints {
            start,
            slider_segments,
        }
    }

    pub fn from_osu_format(osu_slider: &crate::dotosu::sections::objects::Slider) -> Option<Self> {
        match (
            osu_slider.curve_type.as_str(),
            osu_slider.curve_points.len(),
        ) {
            ("L", _) | (_, 0..=1) => {
                if osu_slider.curve_points.len() < 1 {
                    println!("Linear slider must have at least 1 control point.");
                    return None;
                }
                let inner = osu_slider.curve_points[..osu_slider.curve_points.len() - 1].to_vec();
                let end = *osu_slider.curve_points.last().unwrap();
                Some(ControlPoints::new(
                    osu_slider.pos,
                    vec![ControlPointSegment::Linear(inner, end)],
                ))
            }
            ("P", _) => {
                if osu_slider.curve_points.len() != 2 {
                    println!("Perfect circle slider must have exactly 2 control points.");
                    return None;
                }
                Some(ControlPoints::new(
                    osu_slider.pos,
                    vec![ControlPointSegment::PerfectCircle([
                        osu_slider.curve_points[0],
                        osu_slider.curve_points[1],
                    ])],
                ))
            }
            ("B", _) => {
                let (start, segments) =
                    split_up_osu_format_control_points(osu_slider.pos, &osu_slider.curve_points);
                let segments = segments
                    .into_iter()
                    .map(|(points, end)| ControlPointSegment::Bezier(points, end))
                    .collect();
                Some(ControlPoints::new(start, segments))
            }
            ("C", _) => {
                let (start, segments) =
                    split_up_osu_format_control_points(osu_slider.pos, &osu_slider.curve_points);
                let segments = segments
                    .into_iter()
                    .map(|(points, end)| ControlPointSegment::Catmull(points, end))
                    .collect();
                Some(ControlPoints::new(start, segments))
            }
            _ => {
                println!("Unsupported slider curve type: {}", osu_slider.curve_type);
                None
            }
        }
    }

    pub fn to_osu_format(&self) -> Option<(String, Vec<Vec2>)> {
        if self.slider_segments.is_empty() {
            println!("SliderPath has no segments.");
            return None;
        }
        if self.slider_segments.len() == 1 {
            match &self.slider_segments[0] {
                ControlPointSegment::Linear(points, end) => {
                    let points = {
                        let mut v = points.clone();
                        v.push(*end);
                        v
                    };
                    return Some(("L".to_string(), points));
                }
                ControlPointSegment::PerfectCircle(samples) => {
                    return Some(("P".to_string(), vec![samples[0], samples[1]]));
                }
                ControlPointSegment::Catmull(points, end) => {
                    let points = {
                        let mut v = points.clone();
                        v.push(*end);
                        v
                    };
                    return Some(("C".to_string(), points));
                }
                ControlPointSegment::Bezier(points, end) => {
                    let points = {
                        let mut v = points.clone();
                        v.push(*end);
                        v
                    };
                    return Some(("B".to_string(), points));
                }
            }
        }
        let mut prev_segment_end = self.start;
        let mut points: Vec<Vec2> = vec![];
        for segment in &self.slider_segments {
            points.push(prev_segment_end);
            match segment {
                ControlPointSegment::Bezier(seg_points, end) => {
                    points.extend(normalize_segment_inner_points(prev_segment_end, seg_points, *end));
                    points.push(*end);
                    prev_segment_end = *end;
                }
                ControlPointSegment::Linear(seg_points, end) => {
                    points.extend_from_slice(seg_points);
                    points.push(*end);
                    prev_segment_end = *end;
                }
                ControlPointSegment::PerfectCircle(_) => {
                    println!(
                        "Conversion of PerfectCircle slider segments to osu! format is not supported."
                    );
                }
                ControlPointSegment::Catmull(_, _) => {
                    println!(
                        "Conversion of Catmull slider segments to osu! format is not supported."
                    );
                }
            }
        }
        Some(("B".to_string(), points))
    }

    fn construct_untruncated(&self, length_px: f64) -> (Vec<Vec2>, Vec<Vec2>) {
        let mut path_points: Vec<Vec2> = vec![self.start];
        let mut snap_points = vec![self.start];
        let mut total_len = 0.0;
        for segment in &self.slider_segments {
            let last_point = *path_points.last().unwrap();
            match segment {
                ControlPointSegment::Bezier(points, end) => {
                    let normalized = normalize_segment_inner_points(last_point, points, *end);
                    let mut vec = Vec::with_capacity(normalized.len() + 2);
                    vec.push(last_point);
                    vec.extend(normalized);
                    vec.push(*end);
                    let bezier_points = create_bezier_curve(vec.as_slice());
                    for i in 1..bezier_points.len() {
                        let dir = bezier_points[i] - bezier_points[i - 1];
                        total_len += dir.len();
                        path_points.push(bezier_points[i]);
                        if total_len > length_px {
                            return (path_points, snap_points);
                        }
                    }
                    snap_points.push(*end);
                }
                ControlPointSegment::Linear(points, end) => {
                    let mut last_point = last_point;
                    for point in points {
                        let dir = *point - last_point;
                        total_len += dir.len();
                        path_points.push(*point);
                        if total_len > length_px {
                            return (path_points, snap_points);
                        }
                        snap_points.push(*point);
                        last_point = *point;
                    }
                    let dir = *end - last_point;
                    total_len += dir.len();
                    path_points.push(*end);
                    if total_len > length_px {
                        return (path_points, snap_points);
                    }
                    snap_points.push(*end);
                }
                ControlPointSegment::PerfectCircle(points) => {
                    let (circle_points, center) =
                        create_circular_arc_curve([last_point, points[0], points[1]]);
                    snap_points.push(center);
                    for i in 1..circle_points.len() {
                        let dir = circle_points[i] - circle_points[i - 1];
                        total_len += dir.len();
                        path_points.push(circle_points[i]);
                        if total_len > length_px {
                            return (path_points, snap_points);
                        }
                    }
                    snap_points.push(circle_points[circle_points.len() - 1]);
                }
                ControlPointSegment::Catmull(points, end) => {
                    let normalized = normalize_segment_inner_points(last_point, points, *end);
                    let mut vec = Vec::with_capacity(normalized.len() + 2);
                    vec.push(last_point);
                    vec.extend(normalized);
                    vec.push(*end);
                    let bezier_points = create_catmull_curve(vec.as_slice());
                    for i in 1..bezier_points.len() {
                        let dir = bezier_points[i] - bezier_points[i - 1];
                        total_len += dir.len();
                        path_points.push(bezier_points[i]);
                        if total_len > length_px {
                            return (path_points, snap_points);
                        }
                    }
                    snap_points.push(*end);
                }
            }
        }
        snap_points.push(*path_points.last().unwrap());
        return (path_points, snap_points);
    }

    pub fn construct_curve_and_snap_points(&self, length_px: f64) -> (SliderCurveRidge, Vec<Vec2>) {
        let (path_points, mut snap_points) = self.construct_untruncated(length_px);
        let truncated_path = truncate_slider_curve(&path_points, length_px);
        snap_points.push(truncated_path.last().unwrap().point);
        return (
            SliderCurveRidge {
                ridge: truncated_path,
            },
            snap_points,
        );
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct PointWithProgress {
    pub point: Vec2,
    pub progress: f64,
}

fn truncate_slider_curve(path: &[Vec2], target_length: f64) -> Vec<PointWithProgress> {
    let mut accumulated_length = 0.0;
    if path.is_empty() {
        return vec![
            PointWithProgress {
                point: Vec2 { x: 0.0, y: 0.0 },
                progress: 0.0,
            },
            PointWithProgress {
                point: Vec2 {
                    x: 0.0,
                    y: target_length,
                },
                progress: 1.0,
            },
        ];
    }
    let mut truncated_path: Vec<PointWithProgress> = vec![PointWithProgress {
        point: path[0],
        progress: 0.0,
    }];
    for i in 1..path.len() {
        let p0 = truncated_path.last().unwrap().point;
        let p1 = path[i];
        let segment_vec = p1 - p0;
        let segment_length = segment_vec.len();
        if segment_length < 1e-6 {
            continue;
        }
        if accumulated_length + segment_length + 1e-4 >= target_length || i == path.len() - 1 {
            let remaining_length = target_length - accumulated_length;
            let t = remaining_length / segment_length;
            let new_point = p0 + segment_vec * t;
            truncated_path.push(PointWithProgress {
                point: new_point,
                progress: 1.0,
            });
            return truncated_path;
        } else {
            accumulated_length += segment_length;
            truncated_path.push(PointWithProgress {
                point: p1,
                progress: accumulated_length / target_length,
            });
        }
    }
    assert!(!truncated_path.is_empty());
    if truncated_path.len() == 1 {
        println!("Warning: Slider curve path has only one point, which may be invalid.");
        let p0 = truncated_path[0].point;
        truncated_path.push(PointWithProgress {
            point: p0
                + (Vec2 {
                    x: 0.0,
                    y: target_length - accumulated_length,
                }),
            progress: 1.0,
        });
    } else {
        let p0 = truncated_path[truncated_path.len() - 2].point;
        let p1 = truncated_path[truncated_path.len() - 1].point;
        let p01 = p1 - p0;
        let p01_length = p01.len();
        let remaining_length = target_length - accumulated_length;
        let t = remaining_length / p01_length;
        let len = truncated_path.len();
        truncated_path[len - 1] = PointWithProgress {
            point: p1 + p01 * t,
            progress: 1.0,
        };
    }
    truncated_path
}

fn split_up_osu_format_control_points(
    start: Vec2,
    points: &[Vec2],
) -> (Vec2, Vec<(Vec<Vec2>, Vec2)>) {
    let mut result: Vec<(Vec<Vec2>, Vec2)> = vec![];
    let mut current_segment: Vec<Vec2> = vec![start];

    for point in points.iter().copied() {
        if point == *current_segment.last().unwrap() {
            if current_segment.len() >= 2 {
                let end = *current_segment.last().unwrap();
                let inner = if current_segment.len() > 2 {
                    current_segment[1..current_segment.len() - 1].to_vec()
                } else {
                    vec![]
                };
                result.push((inner, end));
            }
            current_segment = vec![point];
            continue;
        }
        current_segment.push(point);
    }

    if current_segment.len() >= 2 {
        let end = *current_segment.last().unwrap();
        let inner = if current_segment.len() > 2 {
            current_segment[1..current_segment.len() - 1].to_vec()
        } else {
            vec![]
        };
        result.push((inner, end));
    }

    (start, result)
}

fn normalize_segment_inner_points<'a>(segment_start: Vec2, mut inner: &'a [Vec2], segment_end: Vec2) -> &'a [Vec2] {
    while !inner.is_empty() && inner[0] == segment_start {
        inner = &inner[1..];
    }
    while !inner.is_empty() && inner[inner.len() - 1] == segment_end {
        inner = &inner[..inner.len() - 1];
    }
    inner
}

fn create_circular_arc_curve(points: [Vec2; 3]) -> (Vec<Vec2>, Vec2) {
    let center = circle_center(points);
    let radius = (points[0] - center).len();

    let v0 = points[0] - center;
    let v1 = points[1] - center;
    let v2 = points[2] - center;

    let v0v2 = v2.div_complex(v0);
    let v0v1 = v1.div_complex(v0);

    let mut v0v1_angle = v0v1.y.atan2(v0v1.x);
    if v0v1_angle < 0.0 {
        v0v1_angle += 2.0 * std::f64::consts::PI;
    }
    let mut v0v2_angle = v0v2.y.atan2(v0v2.x);
    if v0v2_angle < 0.0 {
        v0v2_angle += 2.0 * std::f64::consts::PI;
    }

    if v0v2_angle < v0v1_angle {
        v0v2_angle = -(2.0 * std::f64::consts::PI - v0v2_angle);
    }
    let start_angle = v0.y.atan2(v0.x);

    let mut path_points = vec![points[0]];

    let num_samples =
        ((v0v2_angle.abs() * 100.0 / (2.0 * std::f64::consts::PI)).ceil() as usize).clamp(10, 200);

    for i in 1..=num_samples {
        let t = (i as f64) / ((num_samples + 1) as f64);
        let angle = start_angle + t * v0v2_angle;
        let sample_point = Vec2 {
            x: center.x + radius * angle.cos(),
            y: center.y + radius * angle.sin(),
        };
        path_points.push(sample_point);
    }

    path_points.push(points[2]);

    return (path_points, center);
}

fn create_bezier_curve(points: &[Vec2]) -> Vec<Vec2> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let start = points[0];
    let end = points[points.len() - 1];
    let try_pruning_at_dt = 1.0 / (points.len().min(50) as f64);
    let mut ret = vec![start];
    create_bezier_curve_rec(points, 0.0, 1.0, try_pruning_at_dt, start, end, &mut ret);
    if ret.len() > 100 {
        println!(
            "Warning: Bezier slider path generated {} points, which may be excessive.",
            ret.len()
        );
    }
    return ret;
}

fn create_bezier_curve_rec(
    points: &[Vec2],
    t0: f64,
    t1: f64,
    try_pruning_at_dt: f64,
    p0: Vec2, // already pushed into ret
    p1: Vec2,
    ret: &mut Vec<Vec2>,
) {
    if ((p0 - p1).len2() < 0.3 && (t1-t0) < 0.05) || (t1 - t0) < 1e-3 {
        ret.push(p1);
        return;
    }

    let tm = (t0 + t1) / 2.0;
    let pm = sample_bezier(points, tm);

    if (t1 - t0) < try_pruning_at_dt {
        let v0 = pm - p0;
        let v1 = p1 - pm;
        let cross_product = v0.cross(v1);
        let err = cross_product * cross_product / (v0.len2() * v1.len2());
        if err < 1e-2 {
            ret.push(pm);
            ret.push(p1);
            return;
        }
    }

    create_bezier_curve_rec(points, t0, tm, try_pruning_at_dt, p0, pm, ret);
    create_bezier_curve_rec(points, tm, t1, try_pruning_at_dt, pm, p1, ret);
}

fn create_catmull_curve(points: &[Vec2]) -> Vec<Vec2> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let start = points[0];
    let end = points[points.len() - 1];
    let try_pruning_at_dt = 1.0 / (points.len().min(50) as f64);
    let mut ret = vec![start];
    create_catmull_curve_rec(points, 0.0, 1.0, try_pruning_at_dt, start, end, &mut ret);
    println!(
        "Warning: Catmull slider path generated {} points.",
        ret.len()
    );
    return ret;
}

fn create_catmull_curve_rec(
    points: &[Vec2],
    t0: f64,
    t1: f64,
    try_pruning_at_dt: f64,
    p0: Vec2, // already pushed into ret
    p1: Vec2,
    ret: &mut Vec<Vec2>,
) {
    if (p0 - p1).len2() < 0.3 || (t1 - t0) < 1e-3 {
        ret.push(p1);
        return;
    }

    let tm = (t0 + t1) / 2.0;
    let pm = sample_catmull(points, tm);

    if (t1 - t0) < try_pruning_at_dt {
        let v0 = pm - p0;
        let v1 = p1 - pm;
        let cross_product = v0.cross(v1);
        let err = cross_product * cross_product / (v0.len2() * v1.len2());
        if err < 1e-2 {
            ret.push(pm);
            ret.push(p1);
            return;
        }
    }

    create_catmull_curve_rec(points, t0, tm, try_pruning_at_dt, p0, pm, ret);
    create_catmull_curve_rec(points, tm, t1, try_pruning_at_dt, pm, p1, ret);
}

fn sample_bezier(points: &[Vec2], t: f64) -> Vec2 {
    let n = points.len();
    let mut result = Vec2 { x: 0.0, y: 0.0 };
    for i in 0..n {
        let coeff = BINOMIALS[n - 1][i] * t.powi(i as i32) * (1.0 - t).powi((n - 1 - i) as i32);
        result = result + points[i] * coeff;
    }
    return result;
}

// Catmull-Rom spline sampling
fn sample_catmull(points: &[Vec2], t: f64) -> Vec2 {
    let n = points.len();
    if n == 0 {
        return Vec2 { x: 0.0, y: 0.0 };
    }
    if n == 1 {
        return points[0];
    }

    let t = t.clamp(0.0, 1.0);
    let seg_f = t * ((n - 1) as f64);
    let seg = seg_f.floor() as usize;
    let local_t = seg_f - (seg as f64);

    let p1 = points[seg.min(n - 1)];
    let p2 = points[(seg + 1).min(n - 1)];
    let p0 = if seg == 0 { p1 } else { points[seg - 1] };
    let p3 = if seg + 2 >= n { p2 } else { points[seg + 2] };

    let t2 = local_t * local_t;
    let t3 = t2 * local_t;

    // Standard Catmull-Rom spline (uniform parameterization), matching osu! style.
    // 0.5 * (2p1 + (-p0 + p2)t + (2p0 - 5p1 + 4p2 - p3)t^2 + (-p0 + 3p1 - 3p2 + p3)t^3)
    let term0 = p1 * 2.0;
    let term1 = (p2 - p0) * local_t;
    let term2 = (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2;
    let term3 = (-p0 + p1 * 3.0 - p2 * 3.0 + p3) * t3;

    (term0 + term1 + term2 + term3) * 0.5
}

const BINOMIALS_SIZE: usize = 100;
const BINOMIALS: [[f64; BINOMIALS_SIZE]; BINOMIALS_SIZE] = precompute_binomials::<BINOMIALS_SIZE>();

const fn precompute_binomials<const SIZE: usize>() -> [[f64; SIZE]; SIZE] {
    let mut binomials: [[f64; SIZE]; SIZE] = [[0.0; SIZE]; SIZE];
    let mut n = 0;
    while n < SIZE {
        binomials[n][0] = 1.0;
        binomials[n][n] = 1.0;
        let mut k = 1;
        while k < n {
            binomials[n][k] = binomials[n - 1][k - 1] + binomials[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    binomials
}
