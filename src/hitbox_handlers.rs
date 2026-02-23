use std::{
    rc::Rc,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{
    audio::AudioEngine,
    geometry::{atomic_vec2::AtomicVec2, vec2::Vec2, vec2_transform::Vec2Transform},
    gui::{DragEvent, HoverEvent, RectHitbox, SimpleButton, SimpleHitbox},
    state::{DragState, EditState},
};

pub fn wire_point_hit_test<F>(hitbox: &Rc<SimpleHitbox>, contains: F)
where
    F: 'static + Fn(Vec2) -> bool,
{
    hitbox.set_hit_test(move |point| contains(point));
}

pub fn create_drag_select_hitbox(
    hover_state: Arc<AtomicBool>,
    drag_left: Rc<dyn Fn(Vec2)>,
    drag_right: Rc<dyn Fn(Vec2)>,
    drag_stop_left: Rc<dyn Fn()>,
    drag_stop_right: Rc<dyn Fn()>,
) -> Rc<RectHitbox> {
    let dragging_left = Arc::new(AtomicBool::new(true));
    Rc::new(RectHitbox::new(
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 1.0, y: 1.0 },
        Box::new(move |event: DragEvent| match event {
            DragEvent::Move {
                left,
                absolute_cursor_pos,
                ..
            } => {
                dragging_left.store(left, Ordering::Release);
                if left {
                    drag_left(absolute_cursor_pos);
                } else {
                    drag_right(absolute_cursor_pos);
                }
            }
            DragEvent::Stop => {
                if dragging_left.load(Ordering::Acquire) {
                    drag_stop_left();
                } else {
                    drag_stop_right();
                }
            }
        }),
        Box::new(move |event: HoverEvent| match event {
            HoverEvent::Move { .. } => hover_state.store(true, Ordering::Release),
            HoverEvent::Exit => hover_state.store(false, Ordering::Release),
        }),
    ))
}

pub fn create_volume_control_hitbox(
    hover_state: Arc<AtomicBool>,
    on_value_change: Rc<dyn Fn(f64)>,
) -> Rc<RectHitbox> {
    let origin_state = Rc::new(AtomicVec2::new(Vec2 { x: 0.0, y: 0.0 }));
    let size_state = Rc::new(AtomicVec2::new(Vec2 { x: 1.0, y: 1.0 }));
    let origin_state_for_drag = Rc::clone(&origin_state);
    let size_state_for_drag = Rc::clone(&size_state);

    Rc::new(RectHitbox::new_with_states(
        origin_state,
        size_state,
        Box::new(move |event: DragEvent| match event {
            DragEvent::Move {
                absolute_cursor_pos,
                ..
            } => {
                let origin = origin_state_for_drag.load();
                let size = size_state_for_drag.load();
                let width = size.x.max(1.0);
                let value = ((absolute_cursor_pos.x - origin.x) / width).clamp(0.0, 1.0);
                on_value_change(value);
            }
            DragEvent::Stop => {}
        }),
        Box::new(move |event: HoverEvent| match event {
            HoverEvent::Move { .. } => hover_state.store(true, Ordering::Release),
            HoverEvent::Exit => hover_state.store(false, Ordering::Release),
        }),
    ))
}

pub fn create_selection_drag_hitbox(
    hover_state: Arc<AtomicBool>,
    dragging_state: Arc<AtomicBool>,
    edit_state: Arc<RwLock<EditState>>,
    target_left_selection: bool,
    snap_distance_px: f64,
    movable_snap_hitbox_radius_px: f64,
    playfield_screen_scale: Arc<AtomicVec2>,
    playfield_screen_top_left: Arc<AtomicVec2>,
) -> Rc<RectHitbox> {
    let mut last_pos = None::<Vec2>;
    let mut last_angle = None::<Vec2>;
    let mut cursor_offset = None::<Vec2>;
    let mut dragged_part_of_object = None::<bool>;
    let mut changed = false;
    let snap_distance2 = snap_distance_px.max(0.0).powi(2);
    let movable_hitbox_distance2 = movable_snap_hitbox_radius_px.max(0.0).powi(2);
    Rc::new(RectHitbox::new(
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 1.0, y: 1.0 },
        Box::new(move |event: DragEvent| match event {
            DragEvent::Move {
                absolute_cursor_pos,
                left,
            } => {
                dragging_state.store(true, Ordering::Release);
                let scale = playfield_screen_scale.load();
                let playfield_top_left = playfield_screen_top_left.load();
                if left {
                    last_angle = None;
                    let cursor_playfield = Vec2 {
                        x: (absolute_cursor_pos.x - playfield_top_left.x) / scale.x.max(1e-9),
                        y: (absolute_cursor_pos.y - playfield_top_left.y) / scale.y.max(1e-9),
                    };

                    let (current_pos, current_offset, current_part_of_object) = if last_pos.is_none() {
                        let state = edit_state.read().expect("edit_state lock poisoned");
                        let origin_locked = if target_left_selection {
                            state
                                .left_selection
                                .as_ref()
                                .map(|s| s.origin_locked)
                                .unwrap_or(false)
                        } else {
                            state
                                .right_selection
                                .as_ref()
                                .map(|s| s.origin_locked)
                                .unwrap_or(false)
                        };
                        let mut best: Option<(f64, Vec2, bool)> = None;
                        for snap in state.snap_positions.positions.iter() {
                            let movable = if target_left_selection {
                                snap.from_left_sel_and_movable
                                    || (origin_locked && snap.is_left_origin)
                            } else {
                                snap.from_right_sel_and_movable
                                    || (origin_locked && snap.is_right_origin)
                            };
                            if !movable {
                                continue;
                            }
                            let snap_screen = Vec2 {
                                x: playfield_top_left.x + snap.pos.x * scale.x,
                                y: playfield_top_left.y + snap.pos.y * scale.y,
                            };
                            let d2 = (snap_screen - absolute_cursor_pos).len2();
                            if d2 > movable_hitbox_distance2 {
                                continue;
                            }
                            match best {
                                Some((best_d2, _, _)) if d2 >= best_d2 => {}
                                _ => best = Some((d2, snap.pos, snap.part_of_object)),
                            }
                        }
                        let (start_pos, part_of_object) = best
                            .map(|(_, pos, po)| (pos, po))
                            .unwrap_or((cursor_playfield, false));
                        (start_pos, cursor_playfield - start_pos, part_of_object)
                    } else {
                        let offset = cursor_offset.unwrap_or(Vec2 { x: 0.0, y: 0.0 });
                        let part_of_object = dragged_part_of_object.unwrap_or(false);
                        let unsnapped = cursor_playfield - offset;
                        let unsnapped_screen = Vec2 {
                            x: playfield_top_left.x + unsnapped.x * scale.x,
                            y: playfield_top_left.y + unsnapped.y * scale.y,
                        };
                        let snapped = {
                            let state = edit_state.read().expect("edit_state lock poisoned");
                            let origin_locked = if target_left_selection {
                                state
                                    .left_selection
                                    .as_ref()
                                    .map(|s| s.origin_locked)
                                    .unwrap_or(false)
                            } else {
                                state
                                    .right_selection
                                    .as_ref()
                                    .map(|s| s.origin_locked)
                                    .unwrap_or(false)
                            };
                            let mut best: Option<(f64, Vec2)> = None;
                            for snap in state.snap_positions.positions.iter() {
                                if snap.virtual_stack && !part_of_object {
                                    continue;
                                }
                                let (from_same_side_selection, from_same_side_origin) =
                                    if target_left_selection {
                                        (snap.from_left_sel_and_movable, snap.is_left_origin)
                                    } else {
                                        (snap.from_right_sel_and_movable, snap.is_right_origin)
                                    };
                                if from_same_side_selection
                                    || (from_same_side_origin && !origin_locked)
                                {
                                    continue;
                                }
                                let snap_screen = Vec2 {
                                    x: playfield_top_left.x + snap.pos.x * scale.x,
                                    y: playfield_top_left.y + snap.pos.y * scale.y,
                                };
                                let d2 = (snap_screen - unsnapped_screen).len2();
                                if d2 > snap_distance2 {
                                    continue;
                                }
                                match best {
                                    Some((best_d2, _)) if d2 >= best_d2 => {}
                                    _ => best = Some((d2, snap.pos)),
                                }
                            }
                            best.map(|(_, pos)| pos).unwrap_or(unsnapped)
                        };
                        (snapped, offset, part_of_object)
                    };

                    if let Some(prev) = last_pos {
                        let delta_playfield = current_pos - prev;
                        if delta_playfield.x.abs() > 0.0 || delta_playfield.y.abs() > 0.0 {
                            let mut state = edit_state.write().expect("edit_state lock poisoned");
                            state.translate_selection(target_left_selection, delta_playfield, false);
                            state.set_selection_drag_state(
                                target_left_selection,
                                Some(DragState {
                                    pos: current_pos,
                                    part_of_object: current_part_of_object,
                                    is_rotation: false,
                                }),
                            );
                            changed = true;
                        } else {
                            let mut state = edit_state.write().expect("edit_state lock poisoned");
                            state.set_selection_drag_state(
                                target_left_selection,
                                Some(DragState {
                                    pos: current_pos,
                                    part_of_object: current_part_of_object,
                                    is_rotation: false,
                                }),
                            );
                        }
                    } else {
                        let mut state = edit_state.write().expect("edit_state lock poisoned");
                        state.set_selection_drag_state(
                            target_left_selection,
                            Some(DragState {
                                pos: current_pos,
                                part_of_object: current_part_of_object,
                                is_rotation: false,
                            }),
                        );
                    }
                    last_pos = Some(current_pos);
                    cursor_offset = Some(current_offset);
                    dragged_part_of_object = Some(current_part_of_object);
                } else {
                    last_pos = None;
                    let cursor_playfield = Vec2 {
                        x: (absolute_cursor_pos.x - playfield_top_left.x) / scale.x.max(1e-9),
                        y: (absolute_cursor_pos.y - playfield_top_left.y) / scale.y.max(1e-9),
                    };
                    let (current_pos, current_offset, current_part_of_object) = if last_angle.is_none() {
                        let state = edit_state.read().expect("edit_state lock poisoned");
                        let origin_locked = if target_left_selection {
                            state
                                .left_selection
                                .as_ref()
                                .map(|s| s.origin_locked)
                                .unwrap_or(false)
                        } else {
                            state
                                .right_selection
                                .as_ref()
                                .map(|s| s.origin_locked)
                                .unwrap_or(false)
                        };
                        let mut best: Option<(f64, Vec2, bool)> = None;
                        for snap in state.snap_positions.positions.iter() {
                            let movable = if target_left_selection {
                                snap.from_left_sel_and_movable
                                    || (origin_locked && snap.is_left_origin)
                            } else {
                                snap.from_right_sel_and_movable
                                    || (origin_locked && snap.is_right_origin)
                            };
                            if !movable {
                                continue;
                            }
                            let snap_screen = Vec2 {
                                x: playfield_top_left.x + snap.pos.x * scale.x,
                                y: playfield_top_left.y + snap.pos.y * scale.y,
                            };
                            let d2 = (snap_screen - absolute_cursor_pos).len2();
                            if d2 > movable_hitbox_distance2 {
                                continue;
                            }
                            match best {
                                Some((best_d2, _, _)) if d2 >= best_d2 => {}
                                _ => best = Some((d2, snap.pos, snap.part_of_object)),
                            }
                        }
                        let (start_pos, part_of_object) = best
                            .map(|(_, pos, po)| (pos, po))
                            .unwrap_or((cursor_playfield, false));
                        (start_pos, cursor_playfield - start_pos, part_of_object)
                    } else {
                        let offset = cursor_offset.unwrap_or(Vec2 { x: 0.0, y: 0.0 });
                        let part_of_object = dragged_part_of_object.unwrap_or(false);
                        let unsnapped = cursor_playfield - offset;
                        let unsnapped_screen = Vec2 {
                            x: playfield_top_left.x + unsnapped.x * scale.x,
                            y: playfield_top_left.y + unsnapped.y * scale.y,
                        };
                        let snapped = {
                            let state = edit_state.read().expect("edit_state lock poisoned");
                            let mut best: Option<(f64, Vec2)> = None;
                            for snap in state.snap_positions.positions.iter() {
                                if snap.virtual_stack && !part_of_object {
                                    continue;
                                }
                                let from_same_side_selection = if target_left_selection {
                                    snap.from_left_sel_and_movable
                                } else {
                                    snap.from_right_sel_and_movable
                                };
                                let from_same_side_origin = if target_left_selection {
                                    snap.is_left_origin
                                } else {
                                    snap.is_right_origin
                                };
                                if from_same_side_selection || from_same_side_origin {
                                    continue;
                                }
                                let snap_screen = Vec2 {
                                    x: playfield_top_left.x + snap.pos.x * scale.x,
                                    y: playfield_top_left.y + snap.pos.y * scale.y,
                                };
                                let d2 = (snap_screen - unsnapped_screen).len2();
                                if d2 > snap_distance2 {
                                    continue;
                                }
                                match best {
                                    Some((best_d2, _)) if d2 >= best_d2 => {}
                                    _ => best = Some((d2, snap.pos)),
                                }
                            }
                            best.map(|(_, pos)| pos).unwrap_or(unsnapped)
                        };
                        (snapped, offset, part_of_object)
                    };

                    let mut state = edit_state.write().expect("edit_state lock poisoned");
                    state.set_selection_drag_state(
                        target_left_selection,
                        Some(DragState {
                            pos: current_pos,
                            part_of_object: current_part_of_object,
                            is_rotation: true,
                        }),
                    );
                    let selection = if target_left_selection {
                        &state.left_selection
                    } else {
                        &state.right_selection
                    };
                    if let Some(selection) = selection {
                        let origin = selection.origin;
                        let origin_screen = Vec2 {
                            x: playfield_top_left.x + origin.x * scale.x,
                            y: playfield_top_left.y + origin.y * scale.y,
                        };
                        let current_pos_screen = Vec2 {
                            x: playfield_top_left.x + current_pos.x * scale.x,
                            y: playfield_top_left.y + current_pos.y * scale.y,
                        };
                        let angle = current_pos_screen - origin_screen;
                        const MIN_ANGLE_LEN2: f64 = 1e-4;
                        if angle.len2() > MIN_ANGLE_LEN2 {
                            match last_angle {
                                Some(prev_angle) => {
                                    let scale_locked = if target_left_selection {
                                        state
                                            .left_selection
                                            .as_ref()
                                            .map(|s| s.scale_locked)
                                            .unwrap_or(false)
                                    } else {
                                        state
                                            .right_selection
                                            .as_ref()
                                            .map(|s| s.scale_locked)
                                            .unwrap_or(false)
                                    };
                                    let angle_diff = if scale_locked {
                                        angle.div_complex_normalized(prev_angle)
                                    } else {
                                        angle.div_complex(prev_angle)
                                    };
                                    let transform = Vec2Transform::transform_at_origin(
                                        Vec2Transform::multiply_by_complex(angle_diff),
                                        origin,
                                    );
                                    state.apply_transform(transform, target_left_selection, false);
                                    state.set_selection_drag_state(
                                        target_left_selection,
                                        Some(DragState {
                                            pos: current_pos,
                                            part_of_object: current_part_of_object,
                                            is_rotation: true,
                                        }),
                                    );
                                    changed = true;
                                    last_angle = Some(angle);
                                }
                                None => {
                                    last_angle = Some(angle);
                                }
                            }
                        }
                    }
                    cursor_offset = Some(current_offset);
                    dragged_part_of_object = Some(current_part_of_object);
                }
            }
            DragEvent::Stop => {
                dragging_state.store(false, Ordering::Release);
                {
                    let mut state = edit_state.write().expect("edit_state lock poisoned");
                    state.set_selection_drag_state(target_left_selection, None);
                }
                if changed {
                    let mut state = edit_state.write().expect("edit_state lock poisoned");
                    state.checkpoint_current_state();
                }
                changed = false;
                last_pos = None;
                last_angle = None;
                cursor_offset = None;
                dragged_part_of_object = None;
            }
        }),
        Box::new(move |event: HoverEvent| match event {
            HoverEvent::Move { .. } => hover_state.store(true, Ordering::Release),
            HoverEvent::Exit => hover_state.store(false, Ordering::Release),
        }),
    ))
}

pub fn create_selection_origin_drag_hitbox(
    hover_state: Arc<AtomicBool>,
    dragging_state: Arc<AtomicBool>,
    edit_state: Arc<RwLock<EditState>>,
    target_left_selection: bool,
    snap_distance_px: f64,
    movable_snap_hitbox_radius_px: f64,
    playfield_screen_scale: Arc<AtomicVec2>,
    playfield_screen_top_left: Arc<AtomicVec2>,
) -> Rc<RectHitbox> {
    let mut last_pos = None::<Vec2>;
    let mut cursor_offset = None::<Vec2>;
    let snap_distance2 = snap_distance_px.max(0.0).powi(2);
    let movable_hitbox_distance2 = movable_snap_hitbox_radius_px.max(0.0).powi(2);
    Rc::new(RectHitbox::new(
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 1.0, y: 1.0 },
        Box::new(move |event: DragEvent| match event {
            DragEvent::Move {
                absolute_cursor_pos,
                left,
            } => {
                if !left {
                    let mut state = edit_state.write().expect("edit_state lock poisoned");
                    state.lock_selection_origin_to_center(target_left_selection);
                    return;
                }
                dragging_state.store(true, Ordering::Release);
                let scale = playfield_screen_scale.load();
                let top_left = playfield_screen_top_left.load();
                let cursor_playfield = Vec2 {
                    x: (absolute_cursor_pos.x - top_left.x) / scale.x.max(1e-9),
                    y: (absolute_cursor_pos.y - top_left.y) / scale.y.max(1e-9),
                };

                let (current_pos, current_offset) = if last_pos.is_none() {
                    let state = edit_state.read().expect("edit_state lock poisoned");
                    let mut best: Option<(f64, Vec2)> = None;
                    for snap in state.snap_positions.positions.iter() {
                        let movable = if target_left_selection {
                            snap.is_left_origin
                        } else {
                            snap.is_right_origin
                        };
                        if !movable {
                            continue;
                        }
                        if snap.virtual_stack {
                            continue;
                        }
                        let snap_screen = Vec2 {
                            x: top_left.x + snap.pos.x * scale.x,
                            y: top_left.y + snap.pos.y * scale.y,
                        };
                        let d2 = (snap_screen - absolute_cursor_pos).len2();
                        if d2 > movable_hitbox_distance2 {
                            continue;
                        }
                        match best {
                            Some((best_d2, _)) if d2 >= best_d2 => {}
                            _ => best = Some((d2, snap.pos)),
                        }
                    }
                    let start_pos = best.map(|(_, pos)| pos).unwrap_or(cursor_playfield);
                    (start_pos, cursor_playfield - start_pos)
                } else {
                    let offset = cursor_offset.unwrap_or(Vec2 { x: 0.0, y: 0.0 });
                    let unsnapped = cursor_playfield - offset;
                    let unsnapped_screen = Vec2 {
                        x: top_left.x + unsnapped.x * scale.x,
                        y: top_left.y + unsnapped.y * scale.y,
                    };
                    let snapped = {
                        let state = edit_state.read().expect("edit_state lock poisoned");
                        let mut best: Option<(f64, Vec2)> = None;
                        for snap in state.snap_positions.positions.iter() {
                            if snap.virtual_stack {
                                continue;
                            }
                            let from_same_side_origin = if target_left_selection {
                                snap.is_left_origin
                            } else {
                                snap.is_right_origin
                            };
                            if from_same_side_origin {
                                continue;
                            }
                            let snap_screen = Vec2 {
                                x: top_left.x + snap.pos.x * scale.x,
                                y: top_left.y + snap.pos.y * scale.y,
                            };
                            let d2 = (snap_screen - unsnapped_screen).len2();
                            if d2 > snap_distance2 {
                                continue;
                            }
                            match best {
                                Some((best_d2, _)) if d2 >= best_d2 => {}
                                _ => best = Some((d2, snap.pos)),
                            }
                        }
                        best.map(|(_, pos)| pos).unwrap_or(unsnapped)
                    };
                    (snapped, offset)
                };

                if let Some(prev) = last_pos {
                    let delta_playfield = current_pos - prev;
                    if delta_playfield.x.abs() > 0.0 || delta_playfield.y.abs() > 0.0 {
                        let mut state = edit_state.write().expect("edit_state lock poisoned");
                        state.translate_selection_origin(target_left_selection, delta_playfield);
                    }
                }
                last_pos = Some(current_pos);
                cursor_offset = Some(current_offset);
            }
            DragEvent::Stop => {
                dragging_state.store(false, Ordering::Release);
                last_pos = None;
                cursor_offset = None;
            }
        }),
        Box::new(move |event: HoverEvent| match event {
            HoverEvent::Move { .. } => hover_state.store(true, Ordering::Release),
            HoverEvent::Exit => hover_state.store(false, Ordering::Release),
        }),
    ))
}

pub fn create_progress_bar_hitbox(
    audio: Arc<AudioEngine>,
    seek_dragging: Arc<AtomicBool>,
    seek_resume_after_drag: Arc<AtomicBool>,
    progress_bar_hitbox_hovered: Arc<AtomicBool>,
) -> Rc<RectHitbox> {
    let drag_audio = Arc::clone(&audio);
    let drag_seek_dragging = Arc::clone(&seek_dragging);
    let drag_seek_resume_after_drag: Arc<AtomicBool> = Arc::clone(&seek_resume_after_drag);
    Rc::new_cyclic(|weak_hitbox: &std::rc::Weak<RectHitbox>| {
        let weak_for_drag = weak_hitbox.clone();
        RectHitbox::new(
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Box::new(move |event| match event {
                DragEvent::Move {
                    left,
                    absolute_cursor_pos,
                    ..
                } => {
                    if !left {
                        return;
                    }

                    if !drag_seek_dragging.swap(true, Ordering::AcqRel) {
                        let was_playing = drag_audio.is_playing();
                        drag_seek_resume_after_drag.store(was_playing, Ordering::Release);
                        if was_playing {
                            drag_audio.pause();
                        }
                    }

                    let total_ms = drag_audio.song_total_ms();
                    if total_ms <= 0.0 {
                        return;
                    }
                    let Some(hitbox) = weak_for_drag.upgrade() else {
                        return;
                    };
                    let (hitbox_origin, hitbox_size) = hitbox.bounds();
                    let pos = absolute_cursor_pos - hitbox_origin;
                    let frac = (pos.x / hitbox_size.x.max(1.0)).clamp(0.0, 1.0);
                    drag_audio.seek_map_time_ms(frac * total_ms);
                }
                DragEvent::Stop => {
                    if drag_seek_dragging.swap(false, Ordering::AcqRel)
                        && drag_seek_resume_after_drag.swap(false, Ordering::AcqRel)
                    {
                        drag_audio.play();
                    }
                }
            }),
            Box::new(move |event: HoverEvent| match event {
                HoverEvent::Move { .. } => {
                    progress_bar_hitbox_hovered.store(true, Ordering::Release)
                }
                HoverEvent::Exit => progress_bar_hitbox_hovered.store(false, Ordering::Release),
            }),
        )
    })
}

pub fn create_play_pause_button(audio: Arc<AudioEngine>) -> Rc<SimpleButton> {
    let play_pause_button_audio = Arc::clone(&audio);
    Rc::new(SimpleButton::new(
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 1.0, y: 1.0 },
        Box::new(move || {
            if play_pause_button_audio.is_playing() {
                play_pause_button_audio.pause();
            } else {
                play_pause_button_audio.play();
            }
        }),
    ))
}
