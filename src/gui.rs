use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::geometry::{atomic_vec2::AtomicVec2, vec2::Vec2};

pub struct SimpleHitbox {
    hit_test: RefCell<Box<dyn Fn(Vec2) -> bool>>,
    drag_handler: RefCell<Box<dyn FnMut(DragEvent)>>,
    hover_handler: RefCell<Box<dyn FnMut(HoverEvent)>>,
}

pub enum DragEvent {
    Move {
        left: bool,
        absolute_cursor_pos: Vec2,
    },
    Stop,
}

#[allow(dead_code)]
pub enum HoverEvent {
    Move {
        absolute_cursor_pos: Vec2,
    },
    Exit,
}

impl SimpleHitbox {
    pub fn new(
        hit_test: Box<dyn Fn(Vec2) -> bool>,
        drag_handler: Box<dyn FnMut(DragEvent)>,
        hover_handler: Box<dyn FnMut(HoverEvent)>,
    ) -> Self {
        Self {
            hit_test: RefCell::new(hit_test),
            drag_handler: RefCell::new(drag_handler),
            hover_handler: RefCell::new(hover_handler),
        }
    }

    pub fn set_hit_test<F>(&self, hit_test: F)
    where
        F: 'static + Fn(Vec2) -> bool,
    {
        *self.hit_test.borrow_mut() = Box::new(hit_test);
    }

    fn drag(&self, left: bool, pos: Vec2) {
        (self.drag_handler.borrow_mut())(DragEvent::Move {
            left,
            absolute_cursor_pos: pos,
        });
    }

    fn stop_drag(&self) {
        (self.drag_handler.borrow_mut())(DragEvent::Stop);
    }

    fn hover(&self, pos: Vec2) {
        (self.hover_handler.borrow_mut())(HoverEvent::Move {
            absolute_cursor_pos: pos,
        });
    }

    fn stop_hover(&self) {
        (self.hover_handler.borrow_mut())(HoverEvent::Exit);
    }
}

pub struct RectHitbox {
    hitbox: Rc<SimpleHitbox>,
    origin: Rc<AtomicVec2>,
    size: Rc<AtomicVec2>,
}

impl RectHitbox {
    pub fn new(
        origin: Vec2,
        size: Vec2,
        drag_handler: Box<dyn FnMut(DragEvent)>,
        hover_handler: Box<dyn FnMut(HoverEvent)>,
    ) -> Self {
        let origin_state = Rc::new(AtomicVec2::new(origin));
        let size_state = Rc::new(AtomicVec2::new(size));
        Self::new_with_states(origin_state, size_state, drag_handler, hover_handler)
    }

    pub fn new_with_states(
        origin_state: Rc<AtomicVec2>,
        size_state: Rc<AtomicVec2>,
        drag_handler: Box<dyn FnMut(DragEvent)>,
        hover_handler: Box<dyn FnMut(HoverEvent)>,
    ) -> Self {
        let origin_for_hit_test = Rc::clone(&origin_state);
        let size_for_hit_test = Rc::clone(&size_state);
        let hitbox = Rc::new(SimpleHitbox::new(
            Box::new(move |point| {
                let origin = origin_for_hit_test.load();
                let size = size_for_hit_test.load();
                point.x >= origin.x
                    && point.x <= origin.x + size.x
                    && point.y >= origin.y
                    && point.y <= origin.y + size.y
            }),
            drag_handler,
            hover_handler,
        ));

        Self {
            hitbox,
            origin: origin_state,
            size: size_state,
        }
    }

    pub fn set_bounds(&self, origin: Vec2, size: Vec2) {
        self.origin.store(origin);
        self.size.store(size);
    }

    pub fn bounds(&self) -> (Vec2, Vec2) {
        (self.origin.load(), self.size.load())
    }

    pub fn hitbox(&self) -> Rc<SimpleHitbox> {
        Rc::clone(&self.hitbox)
    }
}

pub struct SimpleButton {
    hitbox: Rc<RectHitbox>,
    hovered: Rc<AtomicBool>,
    clicked: Rc<AtomicBool>,
}

impl SimpleButton {
    pub fn new(top_left: Vec2, size: Vec2, on_click: Box<dyn Fn()>) -> Self {
        let hovered = Rc::new(AtomicBool::new(false));
        let clicked = Rc::new(AtomicBool::new(false));

        let hovered_clone = Rc::clone(&hovered);
        let clicked_clone = Rc::clone(&clicked);

        let hitbox = Rc::new(RectHitbox::new(
            top_left,
            size,
            Box::new(move |event: DragEvent| match event {
                DragEvent::Move { left, .. } => {
                    if !left {
                        return;
                    }
                    if !clicked.swap(true, Ordering::AcqRel) {
                        on_click();
                    }
                }
                DragEvent::Stop => {
                    clicked.store(false, Ordering::Release);
                }
            }),
            Box::new(move |event: HoverEvent| match event {
                HoverEvent::Move { .. } => hovered.store(true, Ordering::Release),
                HoverEvent::Exit => hovered.store(false, Ordering::Release),
            }),
        ));

        Self {
            hitbox,
            hovered: hovered_clone,
            clicked: clicked_clone,
        }
    }

    pub fn set_bounds(&self, top_left: Vec2, size: Vec2) {
        self.hitbox.set_bounds(top_left, size);
    }

    pub fn hitbox(&self) -> Rc<SimpleHitbox> {
        self.hitbox.hitbox()
    }

    pub fn is_hovered(&self) -> bool {
        self.hovered.load(Ordering::Acquire)
    }

    pub fn is_clicked(&self) -> bool {
        self.clicked.load(Ordering::Acquire)
    }
}

impl SimpleHitbox {
    pub fn contains_point(&self, pos: Vec2) -> bool {
        (self.hit_test.borrow())(pos)
    }
}

pub struct MouseHandler {
    hitboxes: Vec<Rc<SimpleHitbox>>,
    current_action: Action,
    focused: bool,
    position: Vec2,
}

pub enum Action {
    None,
    Dragging {
        left: bool,
        hitbox: Rc<SimpleHitbox>,
    },
    Hovering {
        hitbox: Rc<SimpleHitbox>,
    },
}

impl MouseHandler {
    pub fn new() -> Self {
        MouseHandler {
            hitboxes: Vec::new(),
            current_action: Action::None,
            focused: false,
            position: Vec2 { x: 0.0, y: 0.0 },
        }
    }

    pub fn handle_focused_change(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn handle_cursor_move(&mut self, position: Vec2) {
        self.position = position;
        self.handle_move();
    }

    pub fn handle_mouse_input(
        &mut self,
        state: &winit::event::ElementState,
        button: &winit::event::MouseButton,
    ) {
        if !self.focused {
            return;
        }
        let left = match button {
            winit::event::MouseButton::Left => true,
            winit::event::MouseButton::Right => false,
            _ => {
                return;
            }
        };
        match state {
            winit::event::ElementState::Pressed => {
                self.handle_click(left);
            }
            winit::event::ElementState::Released => {
                self.handle_release(left);
            }
        }
    }

    pub fn add_hitbox(&mut self, hitbox: Rc<SimpleHitbox>) {
        self.hitboxes.push(hitbox);
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    fn get_hovering_hitbox(&self) -> Option<Rc<SimpleHitbox>> {
        // Iterate in reverse order to prioritize hitboxes added later (which are on top)
        for hitbox in self.hitboxes.iter().rev() {
            if hitbox.contains_point(self.position) {
                return Some(Rc::clone(hitbox));
            }
        }
        None
    }

    fn find_new_hovered(&mut self) {
        match self.get_hovering_hitbox() {
            Some(hitbox) => {
                hitbox.hover(self.position);
                self.current_action = Action::Hovering { hitbox };
            }
            None => {
                self.current_action = Action::None;
            }
        }
    }

    fn handle_move(&mut self) {
        match &self.current_action {
            Action::Dragging { left, hitbox } => {
                hitbox.drag(*left, self.position);
            }
            Action::Hovering { hitbox } => match self.get_hovering_hitbox() {
                Some(current_hitbox) => {
                    if !Rc::ptr_eq(&current_hitbox, hitbox) {
                        hitbox.stop_hover();
                        current_hitbox.hover(self.position);
                        self.current_action = Action::Hovering {
                            hitbox: current_hitbox,
                        };
                    } else {
                        hitbox.hover(self.position);
                    }
                }
                None => {
                    hitbox.stop_hover();
                    self.current_action = Action::None;
                    self.find_new_hovered();
                    return;
                }
            },
            Action::None => {
                self.find_new_hovered();
            }
        }
    }

    fn handle_click(&mut self, left: bool) {
        match &self.current_action {
            Action::Hovering { hitbox } => {
                hitbox.drag(left, self.position);
                self.current_action = Action::Dragging {
                    left,
                    hitbox: Rc::clone(hitbox),
                };
            }
            _ => {}
        }
    }

    fn handle_release(&mut self, action_left: bool) {
        match &self.current_action {
            Action::Dragging {
                left: drag_left,
                hitbox,
            } => {
                if *drag_left != action_left {
                    return;
                }
                hitbox.stop_drag();
                hitbox.stop_hover();
                self.current_action = Action::None;
                self.find_new_hovered();
            }
            _ => {}
        }
    }
}
