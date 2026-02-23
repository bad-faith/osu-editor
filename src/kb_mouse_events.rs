use std::sync::atomic::Ordering;

use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{editor::EditorApp, geometry::vec2::Vec2};

impl EditorApp {
    pub fn handle_keyboard_input(&mut self, event: &KeyEvent) {
        if event.state == ElementState::Pressed {
            if self.is_current_state_rename_active() {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                        self.commit_current_state_rename();
                        return;
                    }
                    PhysicalKey::Code(KeyCode::Escape) => {
                        self.cancel_current_state_rename();
                        return;
                    }
                    PhysicalKey::Code(KeyCode::Backspace) => {
                        self.backspace_current_state_rename();
                        return;
                    }
                    _ => {}
                }

                if let Some(text) = event.text.as_ref() {
                    self.append_current_state_rename_text(text.as_str());
                }
                return;
            }
        }

        if event.state == ElementState::Pressed && !event.repeat {
            match event.physical_key {
                PhysicalKey::Code(KeyCode::Space) => {
                    // SPACE: PLAY / PAUSE TOGGLE
                    if self.audio.is_playing() {
                        self.audio.pause();
                    } else {
                        self.audio.play();
                    }
                }
                PhysicalKey::Code(KeyCode::Escape) => {
                    self.clear_selections();
                }
                PhysicalKey::Code(KeyCode::F11) => {
                    // F11: TOGGLE FULLSCREEN
                    self.toggle_fullscreen();
                }
                PhysicalKey::Code(KeyCode::Comma) => {
                    // <: ROTATE SELECTION LEFT 90° AROUND PLAYFIELD CENTER
                    self.rotate_selection_left_90(true);
                }
                PhysicalKey::Code(KeyCode::Period) => {
                    // >: ROTATE SELECTION RIGHT 90° AROUND PLAYFIELD CENTER
                    self.rotate_selection_right_90(true);
                }

                PhysicalKey::Code(KeyCode::Numpad3) | PhysicalKey::Code(KeyCode::Digit3) => {
                    self.audio.set_speed(0.5);
                }
                PhysicalKey::Code(KeyCode::Numpad4) | PhysicalKey::Code(KeyCode::Digit4) => {
                    self.audio.set_speed(0.75);
                }
                PhysicalKey::Code(KeyCode::Numpad5) | PhysicalKey::Code(KeyCode::Digit5) => {
                    self.audio.set_speed(1.0);
                }
                PhysicalKey::Code(KeyCode::Numpad6) | PhysicalKey::Code(KeyCode::Digit6) => {
                    self.audio.set_speed(1.25);
                }
                PhysicalKey::Code(KeyCode::Numpad7) | PhysicalKey::Code(KeyCode::Digit7) => {
                    self.audio.set_speed(1.5);
                }
                PhysicalKey::Code(KeyCode::Numpad8) | PhysicalKey::Code(KeyCode::Digit8) => {
                    self.audio.set_speed(1.75);
                }
                PhysicalKey::Code(KeyCode::Numpad9) | PhysicalKey::Code(KeyCode::Digit9) => {
                    self.audio.set_speed(2.0);
                }

                PhysicalKey::Code(KeyCode::KeyP) => {
                    self.desired_fix_pitch = !self.desired_fix_pitch;
                    self.audio.set_fix_pitch(self.desired_fix_pitch);
                }
                PhysicalKey::Code(KeyCode::KeyA) => {
                    self.select_all_to_left();
                }
                PhysicalKey::Code(KeyCode::KeyD) => {
                    self.select_visible_to_left();
                }
                PhysicalKey::Code(KeyCode::KeyS) => {
                    self.swap_selections();
                }
                PhysicalKey::Code(KeyCode::KeyI) => {
                    self.toggle_selection_position_lock(true);
                }
                PhysicalKey::Code(KeyCode::KeyO) => {
                    self.toggle_selection_scale_lock(true);
                }
                PhysicalKey::Code(KeyCode::KeyK) => {
                    self.toggle_selection_position_lock(false);
                }
                PhysicalKey::Code(KeyCode::KeyL) => {
                    self.toggle_selection_scale_lock(false);
                }
                PhysicalKey::Code(KeyCode::KeyZ) => {
                    self.undo();
                }
                PhysicalKey::Code(KeyCode::KeyX) => {
                    self.redo(None);
                }
                PhysicalKey::Code(KeyCode::KeyH) => {
                    self.flip_selection_horizontal();
                }
                PhysicalKey::Code(KeyCode::KeyV) => {
                    self.flip_selection_vertical();
                }
                PhysicalKey::Code(KeyCode::KeyQ) => {
                    self.flip_left_selection_coordinates();
                }
                PhysicalKey::Code(KeyCode::KeyW) => {
                    self.swap_left_selection_xy();
                }
                PhysicalKey::Code(KeyCode::KeyE) => {
                    self.swap_left_selection_xy_2();
                }
                PhysicalKey::Code(KeyCode::KeyR) => {
                    self.swap_left_selection_xy_3();
                }
                PhysicalKey::Code(KeyCode::KeyT) => {
                    self.swap_left_selection_xy_4();
                }
                PhysicalKey::Code(KeyCode::ArrowRight) => {
                    self.translate_selection(true, Vec2 { x: 1.0, y: 0.0 }, true);
                }
                PhysicalKey::Code(KeyCode::ArrowLeft) => {
                    self.translate_selection(true, Vec2 { x: -1.0, y: 0.0 }, true);
                }
                PhysicalKey::Code(KeyCode::ArrowUp) => {
                    self.translate_selection(true, Vec2 { x: 0.0, y: -1.0 }, true);
                }
                PhysicalKey::Code(KeyCode::ArrowDown) => {
                    self.translate_selection(true, Vec2 { x: 0.0, y: 1.0 }, true);
                }
                _ => {}
            };
        }
    }

    pub fn handle_kb_or_mouse_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input(event);
            }
            WindowEvent::Focused(focused) => {
                self.mouse_handler.handle_focused_change(*focused);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_handler.handle_cursor_move(Vec2 {
                    x: position.x,
                    y: position.y,
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if self.is_current_state_rename_active() {
                    self.cancel_current_state_rename();
                }
                match (state, button) {
                    (ElementState::Pressed, winit::event::MouseButton::Forward) => {
                        self.redo(None);
                    }
                    (ElementState::Pressed, winit::event::MouseButton::Back) => {
                        self.undo();
                    }
                    _ => {}
                }
                self.mouse_handler.handle_mouse_input(state, button);
            }

            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                let sign = if match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => *y > 0.0,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y > 0.0,
                } {
                    1.0
                } else {
                    -1.0
                };

                if self.selection_left_origin_hovered.load(Ordering::Acquire) {
                    self.rotate_selection_degrees(true, sign, false);
                    return;
                }
                if self.selection_right_origin_hovered.load(Ordering::Acquire) {
                    self.rotate_selection_degrees(false, sign, false);
                    return;
                }
                if self.selection_left_bbox_hovered.load(Ordering::Acquire) {
                    self.scale_selection_percent(true, 0.01 * sign, false);
                    return;
                }
                if self.selection_right_bbox_hovered.load(Ordering::Acquire) {
                    self.scale_selection_percent(false, 0.01 * sign, false);
                    return;
                }

                if self.sound_volume_hitbox_hovered.load(Ordering::Acquire) {
                    self.desired_sound_volume =
                        (self.audio.get_volume() + 0.05 * sign).clamp(0.0, 1.0);
                    self.audio.set_volume(self.desired_sound_volume);
                }
                if self.hitsound_volume_hitbox_hovered.load(Ordering::Acquire) {
                    self.desired_hitsound_volume =
                        (self.audio.get_hitsound_volume() + 0.05 * sign).clamp(0.0, 1.0);
                    self.audio.set_hitsound_volume(self.desired_hitsound_volume);
                }
                if self.playfield_scale_hitbox_hovered.load(Ordering::Acquire) {
                    let next = (self.current_playfield_scale() + 0.01 * sign).clamp(0.01, 1.0);
                    self.set_playfield_scale(next);
                }
                if self.global_interaction_hitbox_hovered.load(Ordering::Acquire)
                    || self.progress_bar_hitbox_hovered.load(Ordering::Acquire)
                {
                    let current_ms = self.audio.current_time_ms();
                    let song_total_ms = self.audio.song_total_ms();
                    let target_ms = (current_ms - sign * 1000.0).clamp(0.0, song_total_ms);
                    self.audio.seek_map_time_ms(target_ms);
                }
            }
            _ => {}
        }
    }
}
