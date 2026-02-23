use std::sync::{Arc, RwLock};

use super::map_state::MapState;

pub struct ExportThreadState {
    pub latest_export: RwLock<Arc<MapState>>,
}

impl ExportThreadState {
    pub fn new(initial: MapState) -> Self {
        Self {
            latest_export: RwLock::new(Arc::new(initial)),
        }
    }
}