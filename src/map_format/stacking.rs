// referencing https://github.com/MaxOhn/rosu-pp/blob/main/src/osu/convert.rs

use crate::{geometry::vec2::Vec2, map_format::objects::HitObject};

pub fn apply_stacking(
    object_instances: &[HitObject],
    stacking_period: f64,
    circle_radius: f64,
) -> Vec<HitObject> {
    let mut objects_with_info: Vec<ObjectWithStackingInfo> = object_instances
        .iter()
        .map(|obj| ObjectWithStackingInfo {
            object: obj.clone(),
            stack_height: 0,
        })
        .collect();
    stacking(&mut objects_with_info, stacking_period);
    return objects_with_info
        .into_iter()
        .map(|obj_with_info| {
            if obj_with_info.stack_height == 0 {
                obj_with_info.object
            } else {
                let stack_offset = -0.1 * obj_with_info.stack_height as f64 * circle_radius;
                let stack_offset = Vec2 {
                    x: stack_offset,
                    y: stack_offset,
                };
                obj_with_info.object.move_by_offset(stack_offset)
            }
        })
        .collect();
}

struct ObjectWithStackingInfo {
    object: HitObject,
    stack_height: i32,
}

impl ObjectWithStackingInfo {
    fn is_circle(&self) -> bool {
        matches!(self.object, HitObject::Circle(_))
    }

    fn is_slider(&self) -> bool {
        matches!(self.object, HitObject::Slider(_))
    }

    fn is_spinner(&self) -> bool {
        matches!(self.object, HitObject::Spinner(_))
    }

    fn pos(&self) -> Vec2 {
        match &self.object {
            HitObject::Circle(c) => c.pos,
            HitObject::Slider(s) => s.control_points.start,
            HitObject::Spinner(_) => Vec2 { x: 0.0, y: 0.0 },
        }
    }

    fn end_pos(&self) -> Vec2 {
        match &self.object {
            HitObject::Circle(c) => c.pos,
            HitObject::Slider(s) => {
                let (curve, _) = s.control_points.construct_curve_and_snap_points(s.length_pixels);
                return curve.end_point();
            }
            HitObject::Spinner(_) => Vec2 { x: 0.0, y: 0.0 },
        }
    }

    fn start_time(&self) -> f64 {
        match &self.object {
            HitObject::Circle(c) => c.time,
            HitObject::Slider(s) => s.time,
            HitObject::Spinner(s) => s.time,
        }
    }

    fn end_time(&self) -> f64 {
        match &self.object {
            HitObject::Circle(c) => c.time,
            HitObject::Slider(s) => s.end_time(),
            HitObject::Spinner(s) => s.end_time,
        }
    }
}

const STACK_DISTANCE: f64 = 3.0;

fn stacking(hit_objects: &mut [ObjectWithStackingInfo], stack_threshold: f64) {
    let mut extended_start_idx = 0;

    let Some(extended_end_idx) = hit_objects.len().checked_sub(1) else {
        return;
    };

    // First big `if` in osu!lazer's function can be skipped

    for i in (1..=extended_end_idx).rev() {
        let mut n = i;
        let mut obj_i_idx = i;
        // * We should check every note which has not yet got a stack.
        // * Consider the case we have two interwound stacks and this will make sense.
        // *   o <-1      o <-2
        // *    o <-3      o <-4
        // * We first process starting from 4 and handle 2,
        // * then we come backwards on the i loop iteration until we reach 3 and handle 1.
        // * 2 and 1 will be ignored in the i loop because they already have a stack value.

        if hit_objects[obj_i_idx].stack_height != 0 || hit_objects[obj_i_idx].is_spinner() {
            continue;
        }

        // * If this object is a hitcircle, then we enter this "special" case.
        // * It either ends with a stack of hitcircles only,
        // * or a stack of hitcircles that are underneath a slider.
        // * Any other case is handled by the "is_slider" code below this.
        if hit_objects[obj_i_idx].is_circle() {
            loop {
                n = match n.checked_sub(1) {
                    Some(n) => n,
                    None => break,
                };

                if hit_objects[n].is_spinner() {
                    continue;
                }

                if hit_objects[obj_i_idx].start_time() - hit_objects[n].end_time() > stack_threshold
                {
                    break; // * We are no longer within stacking range of the previous object.
                }

                // * HitObjects before the specified update range haven't been reset yet
                if n < extended_start_idx {
                    hit_objects[n].stack_height = 0;
                    extended_start_idx = n;
                }

                // * This is a special case where hticircles are moved DOWN and RIGHT (negative stacking)
                // * if they are under the *last* slider in a stacked pattern.
                // *    o==o <- slider is at original location
                // *        o <- hitCircle has stack of -1
                // *         o <- hitCircle has stack of -2
                if hit_objects[n].is_slider()
                    && hit_objects[n]
                        .end_pos()
                        .distance(hit_objects[obj_i_idx].pos())
                        < STACK_DISTANCE
                {
                    let offset =
                        hit_objects[obj_i_idx].stack_height - hit_objects[n].stack_height + 1;

                    for j in n + 1..=i {
                        // * For each object which was declared under this slider, we will offset
                        // * it to appear *below* the slider end (rather than above).
                        if hit_objects[n].end_pos().distance(hit_objects[j].pos()) < STACK_DISTANCE
                        {
                            hit_objects[j].stack_height -= offset;
                        }
                    }

                    // * We have hit a slider. We should restart calculation using this as the new base.
                    // * Breaking here will mean that the slider still has StackCount of 0,
                    // * so will be handled in the i-outer-loop.
                    break;
                }

                if hit_objects[n].pos().distance(hit_objects[obj_i_idx].pos()) < STACK_DISTANCE {
                    // * Keep processing as if there are no sliders.
                    // * If we come across a slider, this gets cancelled out.
                    // * NOTE: Sliders with start positions stacking
                    // * are a special case that is also handled here.

                    hit_objects[n].stack_height = hit_objects[obj_i_idx].stack_height + 1;
                    obj_i_idx = n;
                }
            }
        } else if hit_objects[obj_i_idx].is_slider() {
            // * We have hit the first slider in a possible stack.
            // * From this point on, we ALWAYS stack positive regardless.
            loop {
                n = match n.checked_sub(1) {
                    Some(n) => n,
                    None => break,
                };

                if hit_objects[n].is_spinner() {
                    continue;
                }

                if hit_objects[obj_i_idx].start_time() - hit_objects[n].start_time()
                    > stack_threshold
                {
                    break; // * We are no longer within stacking range of the previous object.
                }

                if hit_objects[n]
                    .end_pos()
                    .distance(hit_objects[obj_i_idx].pos())
                    < STACK_DISTANCE
                {
                    hit_objects[n].stack_height = hit_objects[obj_i_idx].stack_height + 1;
                    obj_i_idx = n;
                }
            }
        }
    }
}
