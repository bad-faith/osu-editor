use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::JoinHandle,
    time,
};

use crate::{
    geometry::{vec2::Vec2, vec2_transform::Vec2Transform},
    layout::Layout,
    map_format::slider_boxing::{BBox, BBox4},
    render::{is_object_currently_visible, select_visible_objects_in_rect},
    state::history::{CheckPointInfo, History, UndoRedoInfo},
};

use super::{
    drag_state::DragState, export_thread_state::ExportThreadState, hitsound_export::HitsoundExport,
    hitsound_thread_config::HitsoundThreadConfig, map_state::MapState, selection::Selection,
    snap_position::SnapPosition, snap_positions::SnapPositions,
};

pub struct EditState {
    history: History,

    export_thread_state: Arc<ExportThreadState>,
    export_needs_recalc: bool,
    export_request_tx: SyncSender<()>,
    export_thread_stop: Arc<AtomicBool>,
    hitsound_needs_recalc: bool,
    hitsound_request_tx: SyncSender<()>,
    hitsound_thread_stop: Arc<AtomicBool>,
    pub left_selection: Option<Selection>,
    pub right_selection: Option<Selection>,
    pub snap_positions: Arc<SnapPositions>,
}

impl EditState {
    pub fn undo_depth(&self) -> usize {
        self.history.get_current_state_depth()
    }

    pub fn undo_redo_info_for_hud(&self) -> UndoRedoInfo {
        self.history.undo_redo_info()
    }

    pub fn rename_current_state(&mut self, display_name: String) {
        self.history.name_current_state(display_name);
    }

    pub fn get_latest_export(&self) -> Arc<MapState> {
        return Arc::clone(&self.export_thread_state.latest_export.read().unwrap());
    }

    pub fn new(
        map_state: MapState,
        hitsound_thread_config: HitsoundThreadConfig,
    ) -> Arc<RwLock<EditState>> {
        let (tx, rx) = mpsc::sync_channel::<()>(1);
        let (hitsound_tx, hitsound_rx) = mpsc::sync_channel::<()>(1);

        map_state.export();
        let export0 = map_state.clone();

        let export_thread_state = Arc::new(ExportThreadState::new(export0));
        let state = EditState {
            export_thread_state,
            history: History::new(Arc::new(map_state)),
            export_needs_recalc: false,
            export_request_tx: tx,
            export_thread_stop: Arc::new(AtomicBool::new(false)),
            hitsound_needs_recalc: true,
            hitsound_request_tx: hitsound_tx,
            hitsound_thread_stop: Arc::new(AtomicBool::new(false)),
            left_selection: None,
            right_selection: None,
            snap_positions: Arc::new(SnapPositions::new()),
        };

        let state = Arc::new(RwLock::new(state));
        EditState::spawn_export_thread(Arc::clone(&state), rx);
        EditState::spawn_hitsound_thread(Arc::clone(&state), hitsound_rx, hitsound_thread_config);
        {
            let state_guard = state.read().unwrap();
            let _ = state_guard.hitsound_request_tx.try_send(());
        }
        return state;
    }

    fn spawn_export_thread(
        edit_state: Arc<RwLock<EditState>>,
        rx: mpsc::Receiver<()>,
    ) -> JoinHandle<()> {
        let stop_flag = {
            let edit_state_guard = edit_state.read().unwrap();
            Arc::clone(&edit_state_guard.export_thread_stop)
        };

        std::thread::Builder::new()
            .name("edit-state-export".to_string())
            .spawn(move || {
                loop {
                    if rx.recv().is_err() {
                        break;
                    }

                    if stop_flag.load(Ordering::Acquire) {
                        break;
                    }

                    {
                        let mut edit_state_guard = edit_state.write().unwrap();
                        if !edit_state_guard.export_needs_recalc {
                            continue;
                        }
                        edit_state_guard.export_needs_recalc = false;
                    }

                    let current_state = {
                        let edit_state_guard = edit_state.read().unwrap();
                        edit_state_guard.history.get_current_state()
                    };

                    current_state.export();

                    {
                        let edit_state_guard = edit_state.read().unwrap();
                        let mut export_guard = edit_state_guard
                            .export_thread_state
                            .latest_export
                            .write()
                            .unwrap();
                        *export_guard = Arc::clone(&current_state);
                    }
                }
            })
            .expect("failed to spawn edit-state export thread")
    }

    fn spawn_hitsound_thread(
        edit_state: Arc<RwLock<EditState>>,
        rx: mpsc::Receiver<()>,
        hitsound_thread_config: HitsoundThreadConfig,
    ) -> JoinHandle<()> {
        let stop_flag = {
            let edit_state_guard = edit_state.read().unwrap();
            Arc::clone(&edit_state_guard.hitsound_thread_stop)
        };

        std::thread::Builder::new()
            .name("edit-state-hitsounds".to_string())
            .spawn(move || {
                let mut prev_hitsounds = HitsoundExport {
                    hitsounds: Vec::new(),
                };
                loop {
                    if rx.recv().is_err() {
                        break;
                    }

                    if stop_flag.load(Ordering::Acquire) {
                        break;
                    }

                    {
                        let mut edit_state_guard = edit_state.write().unwrap();
                        if !edit_state_guard.hitsound_needs_recalc {
                            continue;
                        }
                        edit_state_guard.hitsound_needs_recalc = false;
                    }

                    let current_state = {
                        let edit_state_guard = edit_state.read().unwrap();
                        edit_state_guard.history.get_current_state()
                    };

                    let hitsound_export = HitsoundExport::from_map_state(&current_state);

                    let mut prev_counts: HashMap<(u64, usize, u64, u64), usize> = HashMap::new();
                    for (map_time_ms, position_x, hitsound_info) in prev_hitsounds.hitsounds.iter()
                    {
                        for (index, volume, event_x) in hitsound_thread_config
                            .routing
                            .resolve_audio_events(hitsound_info, *position_x)
                        {
                            *prev_counts
                                .entry((
                                    map_time_ms.to_bits(),
                                    index,
                                    volume.to_bits(),
                                    event_x.to_bits(),
                                ))
                                .or_insert(0) += 1;
                        }
                    }

                    let mut next_counts: HashMap<(u64, usize, u64, u64), usize> = HashMap::new();
                    for (map_time_ms, position_x, hitsound_info) in hitsound_export.hitsounds.iter()
                    {
                        for (index, volume, event_x) in hitsound_thread_config
                            .routing
                            .resolve_audio_events(hitsound_info, *position_x)
                        {
                            *next_counts
                                .entry((
                                    map_time_ms.to_bits(),
                                    index,
                                    volume.to_bits(),
                                    event_x.to_bits(),
                                ))
                                .or_insert(0) += 1;
                        }
                    }

                    #[derive(Copy, Clone, Eq, PartialEq)]
                    enum HitsoundActionKind {
                        Remove,
                        Add,
                    }

                    let mut actions: Vec<(f64, HitsoundActionKind, usize, f64, f64)> = Vec::new();

                    for ((map_time_ms_bits, index, volume_bits, x_bits), prev_count) in
                        prev_counts.iter()
                    {
                        let next_count = next_counts
                            .get(&(*map_time_ms_bits, *index, *volume_bits, *x_bits))
                            .copied()
                            .unwrap_or(0);
                        let remove_count = prev_count.saturating_sub(next_count);
                        for _ in 0..remove_count {
                            actions.push((
                                f64::from_bits(*map_time_ms_bits),
                                HitsoundActionKind::Remove,
                                *index,
                                f64::from_bits(*volume_bits),
                                f64::from_bits(*x_bits),
                            ));
                        }
                    }

                    for ((map_time_ms_bits, index, volume_bits, x_bits), next_count) in
                        next_counts.iter()
                    {
                        let prev_count = prev_counts
                            .get(&(*map_time_ms_bits, *index, *volume_bits, *x_bits))
                            .copied()
                            .unwrap_or(0);
                        let add_count = next_count.saturating_sub(prev_count);
                        for _ in 0..add_count {
                            actions.push((
                                f64::from_bits(*map_time_ms_bits),
                                HitsoundActionKind::Add,
                                *index,
                                f64::from_bits(*volume_bits),
                                f64::from_bits(*x_bits),
                            ));
                        }
                    }

                    actions.sort_by(|a, b| {
                        let (a_time, a_kind, _, _, _) = a;
                        let (b_time, b_kind, _, _, _) = b;

                        match a_time
                            .partial_cmp(b_time)
                            .unwrap_or(std::cmp::Ordering::Equal)
                        {
                            std::cmp::Ordering::Equal => match (a_kind, b_kind) {
                                (HitsoundActionKind::Remove, HitsoundActionKind::Add) => {
                                    std::cmp::Ordering::Less
                                }
                                (HitsoundActionKind::Add, HitsoundActionKind::Remove) => {
                                    std::cmp::Ordering::Greater
                                }
                                _ => std::cmp::Ordering::Equal,
                            },
                            non_eq => non_eq,
                        }
                    });

                    for (map_time_ms, action, index, volume, x) in actions {
                        match action {
                            HitsoundActionKind::Remove => {
                                hitsound_thread_config.audio.remove_hitsound(
                                    map_time_ms,
                                    index,
                                    volume,
                                    x,
                                );
                            }
                            HitsoundActionKind::Add => {
                                hitsound_thread_config.audio.add_hitsound(
                                    map_time_ms,
                                    index,
                                    volume,
                                    x,
                                );
                            }
                        }
                    }

                    prev_hitsounds = hitsound_export;
                }
            })
            .expect("failed to spawn edit-state hitsound thread")
    }

    pub fn prepare_for_render(
        &mut self,
        layout: &Layout,
        time_ms: f64,
        overlay_rect_left: Option<[f32; 4]>,
        overlay_rect_right: Option<[f32; 4]>,
        cursor_pos_screen: [f32; 2],
        left_rect_dragged: bool,
        right_rect_dragged: bool,
        _left_origin_dragged: bool,
        _right_origin_dragged: bool,
    ) -> (
        Vec<usize>,
        Vec<usize>,
        Option<BBox4>,
        Option<BBox4>,
        Option<Vec2>,
        Option<Vec2>,
        Vec2,
        Vec2,
        bool,
        bool,
        f64,
        f64,
        f64,
        f64,
        bool,
        bool,
        bool,
        bool,
        Option<Vec2>,
        Option<Vec2>,
    ) {
        let active_export = self.get_latest_export();
        let circle_radius = self.history.get_current_state().diff_settings.circle_radius;
        let stack_offset = circle_radius * 0.1;
        let stack_near_distance = circle_radius * 2.0;
        let stack_near_distance_2 = stack_near_distance * stack_near_distance;

        let cursor_playfield = {
            let w = layout.playfield_rect.x1 - layout.playfield_rect.x0;
            let h = layout.playfield_rect.y1 - layout.playfield_rect.y0;
            if w.abs() > 1e-6 && h.abs() > 1e-6 {
                Some(Vec2 {
                    x: (cursor_pos_screen[0] as f64 - layout.playfield_rect.x0) * (512.0 / w),
                    y: (cursor_pos_screen[1] as f64 - layout.playfield_rect.y0) * (384.0 / h),
                })
            } else {
                None
            }
        };

        self.snap_positions = {
            let left_sel_set = match self.left_selection {
                Some(ref left_selection) => left_selection.objects.iter().copied().collect(),
                None => HashSet::new(),
            };
            let right_sel_set = match self.right_selection {
                Some(ref right_selection) => right_selection.objects.iter().copied().collect(),
                None => HashSet::new(),
            };

            let mut snap_positions = SnapPositions::new();
            for (index, obj) in active_export.objects.iter().enumerate() {
                let instance = obj.instance().unwrap();
                let from_left = left_sel_set.contains(&index);
                let from_right = right_sel_set.contains(&index);

                if (!from_left && !from_right && is_object_currently_visible(instance, time_ms))
                    || from_left
                    || from_right
                {
                    for snap_pos in instance.snap_points.iter() {
                        snap_positions.positions.push(SnapPosition {
                            pos: *snap_pos,
                            virtual_stack: false,
                            part_of_object: true,
                            from_left_sel_and_movable: from_left && !right_rect_dragged,
                            from_right_sel_and_movable: from_right && !left_rect_dragged,
                            is_left_origin: false,
                            is_right_origin: false,
                        });
                        if let Some(cursor_pf) = cursor_playfield {
                            if cursor_pf.distance2(*snap_pos) <= stack_near_distance_2 {
                                if (!from_left && !from_right)
                                    || (from_left && !left_rect_dragged)
                                    || (from_right && !right_rect_dragged)
                                {
                                    let stack_vec = Vec2 {
                                        x: stack_offset,
                                        y: stack_offset,
                                    };
                                    snap_positions.positions.push(SnapPosition {
                                        pos: *snap_pos + stack_vec,
                                        virtual_stack: true,
                                        part_of_object: true,
                                        from_left_sel_and_movable: false,
                                        from_right_sel_and_movable: false,
                                        is_left_origin: false,
                                        is_right_origin: false,
                                    });
                                    snap_positions.positions.push(SnapPosition {
                                        pos: *snap_pos - stack_vec,
                                        virtual_stack: true,
                                        part_of_object: true,
                                        from_left_sel_and_movable: false,
                                        from_right_sel_and_movable: false,
                                        is_left_origin: false,
                                        is_right_origin: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            match self.left_selection {
                Some(ref left_selection) => {
                    snap_positions.positions.push(SnapPosition {
                        pos: left_selection.curr_center,
                        virtual_stack: false,
                        part_of_object: false,
                        from_left_sel_and_movable: !right_rect_dragged,
                        from_right_sel_and_movable: false,
                        is_left_origin: false,
                        is_right_origin: false,
                    });
                    snap_positions.positions.push(SnapPosition {
                        pos: left_selection.origin,
                        virtual_stack: false,
                        part_of_object: false,
                        from_left_sel_and_movable: false,
                        from_right_sel_and_movable: false,
                        is_left_origin: true,
                        is_right_origin: false,
                    });
                }
                None => {}
            }

            match self.right_selection {
                Some(ref right_selection) => {
                    snap_positions.positions.push(SnapPosition {
                        pos: right_selection.curr_center,
                        virtual_stack: false,
                        part_of_object: false,
                        from_left_sel_and_movable: false,
                        from_right_sel_and_movable: !left_rect_dragged,
                        is_left_origin: false,
                        is_right_origin: false,
                    });
                    snap_positions.positions.push(SnapPosition {
                        pos: right_selection.origin,
                        virtual_stack: false,
                        part_of_object: false,
                        from_left_sel_and_movable: false,
                        from_right_sel_and_movable: false,
                        is_left_origin: false,
                        is_right_origin: true,
                    });
                }
                None => {}
            }

            Arc::new(snap_positions)
        };

        let (next_left_selection, next_right_selection) = {
            let next_left = overlay_rect_left.map(|rect| {
                select_visible_objects_in_rect(
                    rect,
                    active_export.objects.iter(),
                    &layout.playfield_rect,
                    time_ms,
                    Self::selection_objects(&self.right_selection),
                    &[],
                )
            });

            let next_right = overlay_rect_right.map(|rect| {
                select_visible_objects_in_rect(
                    rect,
                    active_export.objects.iter(),
                    &layout.playfield_rect,
                    time_ms,
                    Self::selection_objects(&self.left_selection),
                    match self.right_selection {
                        Some(ref right_selection) => right_selection.objects.as_slice(),
                        None => &[],
                    },
                )
            });

            (next_left, next_right)
        };

        if let Some((next_left, left_bbox)) = next_left_selection {
            self.left_selection = Self::selection_from_parts(
                next_left,
                left_bbox,
                self.history.get_current_state().diff_settings.circle_radius,
            );
        }

        if let Some((next_right, right_bbox)) = next_right_selection {
            self.right_selection = Self::selection_from_parts(
                next_right,
                right_bbox,
                self.history.get_current_state().diff_settings.circle_radius,
            );
        }

        let (left_exists, left_scale, left_rotation, left_origin_locked, left_scale_locked) =
            if let Some(left) = self.left_selection.as_ref() {
                (
                    true,
                    left.total_scale,
                    left.total_rotation_degrees,
                    left.origin_locked,
                    left.scale_locked,
                )
            } else {
                (false, 1.0, 0.0, false, false)
            };
        let (right_exists, right_scale, right_rotation, right_origin_locked, right_scale_locked) =
            if let Some(right) = self.right_selection.as_ref() {
                (
                    true,
                    right.total_scale,
                    right.total_rotation_degrees,
                    right.origin_locked,
                    right.scale_locked,
                )
            } else {
                (false, 1.0, 0.0, false, false)
            };
        let left_moved = self
            .left_selection
            .as_ref()
            .map(|s| s.moved)
            .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
        let right_moved = self
            .right_selection
            .as_ref()
            .map(|s| s.moved)
            .unwrap_or(Vec2 { x: 0.0, y: 0.0 });

        return (
            Self::selection_objects(&self.left_selection).to_vec(),
            Self::selection_objects(&self.right_selection).to_vec(),
            self.left_selection.as_ref().map(|s| s.bbox_outer.clone()),
            self.right_selection.as_ref().map(|s| s.bbox_outer.clone()),
            self.left_selection.as_ref().map(|s| s.origin),
            self.right_selection.as_ref().map(|s| s.origin),
            left_moved,
            right_moved,
            left_exists,
            right_exists,
            left_scale,
            right_scale,
            left_rotation,
            right_rotation,
            left_origin_locked,
            right_origin_locked,
            left_scale_locked,
            right_scale_locked,
            self.left_selection
                .as_ref()
                .and_then(|s| s.drag_state.as_ref().map(|d| d.pos)),
            self.right_selection
                .as_ref()
                .and_then(|s| s.drag_state.as_ref().map(|d| d.pos)),
        );
    }

    pub fn set_selection_drag_state(&mut self, left: bool, drag_state: Option<DragState>) {
        if left {
            if let Some(selection) = self.left_selection.as_mut() {
                selection.drag_state = drag_state;
            }
        } else if let Some(selection) = self.right_selection.as_mut() {
            selection.drag_state = drag_state;
        }
    }

    pub fn toggle_selection_origin_lock(&mut self, left: bool) {
        if left {
            if let Some(selection) = self.left_selection.as_mut() {
                selection.origin_locked = !selection.origin_locked;
            }
        } else if let Some(selection) = self.right_selection.as_mut() {
            selection.origin_locked = !selection.origin_locked;
        }
    }

    pub fn toggle_selection_scale_lock(&mut self, left: bool) {
        if left {
            if let Some(selection) = self.left_selection.as_mut() {
                selection.scale_locked = !selection.scale_locked;
            }
        } else if let Some(selection) = self.right_selection.as_mut() {
            selection.scale_locked = !selection.scale_locked;
        }
    }

    pub fn request_export_thread_stop(&self) {
        self.export_thread_stop.store(true, Ordering::Release);
        match self.export_request_tx.try_send(()) {
            Ok(())
            | Err(mpsc::TrySendError::Full(_))
            | Err(mpsc::TrySendError::Disconnected(_)) => {}
        }
        self.hitsound_thread_stop.store(true, Ordering::Release);
        match self.hitsound_request_tx.try_send(()) {
            Ok(())
            | Err(mpsc::TrySendError::Full(_))
            | Err(mpsc::TrySendError::Disconnected(_)) => {}
        }
    }

    pub fn apply_transform(
        &mut self,
        transform: Vec2Transform,
        left_selection: bool,
        checkpoint: bool,
    ) {
        let current_map_state = self.history.get_current_state().clone();
        let selection = if left_selection {
            Self::selection_objects(&self.left_selection).to_vec()
        } else {
            Self::selection_objects(&self.right_selection).to_vec()
        };
        let new_map_state = current_map_state.transform_objects(transform, selection.as_slice());
        let checkpoint = if checkpoint {
            CheckPointInfo::CheckPoint
        } else {
            CheckPointInfo::CheckPointAfter(time::Duration::from_millis(50))
        };
        self.history.append(Arc::new(new_map_state), checkpoint);
        if left_selection {
            if let Some(selection) = self.left_selection.as_mut() {
                selection.apply_transform(transform);
            }
        } else {
            if let Some(selection) = self.right_selection.as_mut() {
                selection.apply_transform(transform);
            }
        }
        self.export_needs_recalc = true;
        self.hitsound_needs_recalc = true;
        let _ = self.export_request_tx.try_send(());
        let _ = self.hitsound_request_tx.try_send(());
    }

    pub fn undo(&mut self) {
        if self.history.undo() {
            self.export_needs_recalc = true;
            self.hitsound_needs_recalc = true;
            self.left_selection = None;
            self.right_selection = None;
            let _ = self.export_request_tx.try_send(());
            let _ = self.hitsound_request_tx.try_send(());
        }
    }

    pub fn redo(&mut self, uuid: Option<u128>) {
        if self.history.redo(uuid) {
            self.export_needs_recalc = true;
            self.hitsound_needs_recalc = true;
            self.left_selection = None;
            self.right_selection = None;
            let _ = self.export_request_tx.try_send(());
            let _ = self.hitsound_request_tx.try_send(());
        }
    }

    pub fn checkpoint_current_state(&mut self) {
        self.history.save_checkpoint();
    }

    pub fn clear_selections(&mut self) {
        self.left_selection = None;
        self.right_selection = None;
    }

    pub fn select_all_to_left(&mut self) {
        let state = self.history.get_current_state();
        let object_count = state.objects.len();

        let mut left_selected_objects: Vec<usize> = Vec::with_capacity(object_count);

        let right_set = match &self.right_selection {
            Some(right_selection) => right_selection.objects.iter().copied().collect(),
            None => HashSet::new(),
        };

        for i in 0..object_count {
            if right_set.contains(&i) {
                continue;
            }
            left_selected_objects.push(i);
        }

        self.left_selection = Self::selection_from_objects(&state, left_selected_objects);
    }

    pub fn select_visible_to_left(&mut self, time_ms: f64) {
        const FADE_OUT_MS: f64 = 250.0;

        let current_state = self.history.get_current_state();
        current_state.export();

        let mut left_selected_set = HashSet::new();
        let right_set = match &self.right_selection {
            Some(right_selection) => right_selection.objects.iter().copied().collect(),
            None => HashSet::new(),
        };

        for (idx, object) in current_state.objects.iter().enumerate() {
            if right_set.contains(&idx) {
                continue;
            }
            let object = object.instance().unwrap();
            let appear_ms = if object.is_spinner {
                object.time
            } else {
                object.time - object.preempt
            };
            let end_ms = if object.is_slider || object.is_spinner {
                object.slider_end_time_ms
            } else {
                object.time
            };
            let disappear_ms = end_ms + FADE_OUT_MS;

            if time_ms >= appear_ms && time_ms <= disappear_ms {
                left_selected_set.insert(idx);
            }
        }

        let mut left_selected_objects: Vec<usize> = Vec::with_capacity(left_selected_set.len());
        let mut bbox: Option<BBox> = None;
        for (idx, object) in current_state.objects.iter().enumerate() {
            let object = object.instance().unwrap();
            if !left_selected_set.contains(&idx) {
                continue;
            }

            left_selected_objects.push(idx);
            if let Some(object_bbox) = object.get_bbox() {
                bbox = Some(match &bbox {
                    Some(current_bbox) => BBox {
                        x: [
                            current_bbox.x[0].min(object_bbox.x[0]),
                            current_bbox.x[1].max(object_bbox.x[1]),
                        ],
                        y: [
                            current_bbox.y[0].min(object_bbox.y[0]),
                            current_bbox.y[1].max(object_bbox.y[1]),
                        ],
                    },
                    None => object_bbox,
                });
            }
        }

        self.left_selection = Self::selection_from_parts(
            left_selected_objects,
            bbox,
            self.history.get_current_state().diff_settings.circle_radius,
        );
    }

    pub fn swap_selections(&mut self) {
        std::mem::swap(&mut self.left_selection, &mut self.right_selection);
    }

    pub fn rotate_selection_left_90(&mut self, left_selection: bool) {
        self.rotate_selected_around_bbox_origin_90(false, left_selection);
    }

    pub fn rotate_selection_right_90(&mut self, left_selection: bool) {
        self.rotate_selected_around_bbox_origin_90(true, left_selection);
    }

    pub fn flip_selection_coordinates(&mut self, left: bool) {
        let selection = if left {
            &self.left_selection
        } else {
            &self.right_selection
        };
        if let Some(selection) = selection {
            let origin = selection.origin;
            let transform = Vec2Transform::transform_at_origin(
                Vec2Transform::multiply_by_complex(Vec2 { x: -1.0, y: 0.0 }),
                origin,
            );
            self.apply_transform(transform, left, true);
        }
    }

    pub fn rotate_selection_degrees(&mut self, left: bool, degrees: f64, checkpoint: bool) {
        let selection = if left {
            &self.left_selection
        } else {
            &self.right_selection
        };
        if let Some(selection) = selection {
            let origin = selection.origin;
            let radians = degrees.to_radians();
            let transform = Vec2Transform::transform_at_origin(
                Vec2Transform::multiply_by_complex(Vec2 {
                    x: radians.cos(),
                    y: radians.sin(),
                }),
                origin,
            );
            self.apply_transform(transform, left, checkpoint);
        }
    }

    pub fn scale_selection_percent(&mut self, left: bool, percent_delta: f64, checkpoint: bool) {
        let selection = if left {
            &self.left_selection
        } else {
            &self.right_selection
        };
        if let Some(selection) = selection {
            let origin = selection.origin;
            let scale = (1.0 + percent_delta).max(0.01);
            let transform = Vec2Transform::transform_at_origin(
                Vec2Transform::multiply_by_complex(Vec2 { x: scale, y: 0.0 }),
                origin,
            );
            self.apply_transform(transform, left, checkpoint);
        }
    }

    pub fn flip_selection_horizontal(&mut self) {
        if let Some(left_selection) = &self.left_selection {
            let origin = left_selection.origin;
            let transform = Vec2Transform::flip_around_axis_line([
                Vec2 {
                    x: origin.x - 1.0,
                    y: origin.y,
                },
                Vec2 {
                    x: origin.x + 1.0,
                    y: origin.y,
                },
            ]);
            self.apply_transform(transform, true, true);
        }
    }

    pub fn flip_selection_vertical(&mut self) {
        if let Some(left_selection) = &self.left_selection {
            let origin = left_selection.origin;
            let transform = Vec2Transform::flip_around_axis_line([
                Vec2 {
                    x: origin.x,
                    y: origin.y - 1.0,
                },
                Vec2 {
                    x: origin.x,
                    y: origin.y + 1.0,
                },
            ]);
            self.apply_transform(transform, true, true);
        }
    }

    pub fn swap_selection_xy(&mut self, left: bool) {
        let selection = if left {
            &self.left_selection
        } else {
            &self.right_selection
        };
        if let Some(selection) = selection {
            let origin = selection.origin;
            let transform =
                Vec2Transform::transform_at_origin(Vec2Transform::transpose_1(), origin);
            self.apply_transform(transform, left, true);
        }
    }

    pub fn swap_selection_xy_2(&mut self, left: bool) {
        let selection = if left {
            &self.left_selection
        } else {
            &self.right_selection
        };
        if let Some(selection) = selection {
            let origin = selection.origin;
            let transform =
                Vec2Transform::transform_at_origin(Vec2Transform::transpose_2(), origin);
            self.apply_transform(transform, left, true);
        }
    }

    pub fn swap_selection_xy_3(&mut self, left: bool) {
        let selection = if left {
            &self.left_selection
        } else {
            &self.right_selection
        };
        if let Some(selection) = selection {
            let origin = selection.origin;
            let transform =
                Vec2Transform::transform_at_origin(Vec2Transform::transpose_3(), origin);
            self.apply_transform(transform, left, true);
        }
    }

    pub fn swap_selection_xy_4(&mut self, left: bool) {
        let selection = if left {
            &self.left_selection
        } else {
            &self.right_selection
        };
        if let Some(selection) = selection {
            let origin = selection.origin;
            let transform =
                Vec2Transform::transform_at_origin(Vec2Transform::transpose_4(), origin);
            self.apply_transform(transform, left, true);
        }
    }

    pub fn rotate_selected_around_bbox_origin_90(&mut self, clockwise: bool, left_selection: bool) {
        let rotation_vector = if clockwise {
            Vec2 { x: 0.0, y: 1.0 }
        } else {
            Vec2 { x: 0.0, y: -1.0 }
        };
        let selection = if left_selection {
            &self.left_selection
        } else {
            &self.right_selection
        };
        match selection {
            Some(selection) => {
                let center = selection.origin;
                let transform = Vec2Transform::transform_at_origin(
                    Vec2Transform::multiply_by_complex(rotation_vector),
                    center,
                );
                self.apply_transform(transform, left_selection, true);
            }
            None => {}
        }
    }

    pub fn translate_selection(&mut self, left: bool, vec: Vec2, checkpoint: bool) {
        let transform = Vec2Transform::translate(vec);
        if left {
            self.apply_transform(transform, true, checkpoint);
        } else {
            self.apply_transform(transform, false, checkpoint);
        }
    }

    pub fn translate_selection_origin(&mut self, left: bool, vec: Vec2) {
        if left {
            if let Some(selection) = self.left_selection.as_mut() {
                selection.origin_locked = false;
                selection.origin = selection.origin + vec;
            }
        } else if let Some(selection) = self.right_selection.as_mut() {
            selection.origin_locked = false;
            selection.origin = selection.origin + vec;
        }
    }

    pub fn set_selection_origin(&mut self, left: bool, origin: Vec2) {
        if left {
            if let Some(selection) = self.left_selection.as_mut() {
                selection.origin = origin;
                selection.origin_locked = false;
            }
        } else if let Some(selection) = self.right_selection.as_mut() {
            selection.origin = origin;
            selection.origin_locked = false;
        }
    }

    pub fn lock_selection_origin_to_center(&mut self, left: bool) {
        let center = Vec2 { x: 256.0, y: 192.0 };
        self.set_selection_origin(left, center);
        if left {
            if let Some(selection) = self.left_selection.as_mut() {
                selection.origin_locked = true;
            }
        } else if let Some(selection) = self.right_selection.as_mut() {
            selection.origin_locked = true;
        }
    }

    fn selection_objects(selection: &Option<Selection>) -> &[usize] {
        match selection {
            Some(selection) => selection.objects.as_slice(),
            None => &[],
        }
    }

    fn selection_from_parts(
        objects: Vec<usize>,
        bbox_inner: Option<BBox>,
        radius: f64,
    ) -> Option<Selection> {
        match bbox_inner {
            Some(bbox_inner) => {
                let bbox_inner = BBox4::from_bbox(bbox_inner);
                let bbox_outer = bbox_inner.expand(radius);
                let origin = bbox_inner.center();
                Some(Selection {
                    radius,
                    objects,
                    bbox_inner,
                    bbox_outer,
                    origin,
                    origin_locked: false,
                    scale_locked: false,
                    orig_center: origin,
                    curr_center: origin,
                    curr_center_plus1: Vec2 {
                        x: origin.x + 1.0,
                        y: origin.y,
                    },
                    total_rotation_degrees: 0.0,
                    total_scale: 1.0,
                    moved: Vec2 { x: 0.0, y: 0.0 },
                    drag_state: None,
                })
            }
            None => None,
        }
    }

    fn selection_from_objects(map_state: &MapState, objects: Vec<usize>) -> Option<Selection> {
        let mut bbox: Option<BBox> = None;
        for idx in objects.iter() {
            let object = map_state.objects.get(*idx);
            let Some(instance) = object.instance() else {
                continue;
            };
            let Some(object_bbox) = instance.get_bbox() else {
                continue;
            };
            bbox = Some(match &bbox {
                Some(current_bbox) => BBox {
                    x: [
                        current_bbox.x[0].min(object_bbox.x[0]),
                        current_bbox.x[1].max(object_bbox.x[1]),
                    ],
                    y: [
                        current_bbox.y[0].min(object_bbox.y[0]),
                        current_bbox.y[1].max(object_bbox.y[1]),
                    ],
                },
                None => object_bbox,
            });
        }
        Self::selection_from_parts(objects, bbox, map_state.diff_settings.circle_radius)
    }
}
