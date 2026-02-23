use std::sync::atomic::{AtomicU64, Ordering};

use crate::geometry::vec2::Vec2;


pub struct AtomicVec2 {
    x_bits: AtomicU64,
    y_bits: AtomicU64,
}

impl AtomicVec2 {
    pub fn new(value: Vec2) -> Self {
        Self {
            x_bits: AtomicU64::new(value.x.to_bits()),
            y_bits: AtomicU64::new(value.y.to_bits()),
        }
    }

    pub fn load(&self) -> Vec2 {
        Vec2 {
            x: f64::from_bits(self.x_bits.load(Ordering::Acquire)),
            y: f64::from_bits(self.y_bits.load(Ordering::Acquire)),
        }
    }

    pub fn store(&self, value: Vec2) {
        self.x_bits.store(value.x.to_bits(), Ordering::Release);
        self.y_bits.store(value.y.to_bits(), Ordering::Release);
    }
}
