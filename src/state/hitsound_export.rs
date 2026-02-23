use crate::map_format::objects::HitsoundInfo;

use super::map_state::MapState;

pub struct HitsoundExport {
    pub hitsounds: Vec<(f64, f64, HitsoundInfo)>,
}

impl HitsoundExport {
    pub fn from_map_state(map_state: &MapState) -> Self {
        let mut exported = HitsoundExport {
            hitsounds: Vec::new(),
        };
        map_state.export_hitsounds(&mut exported);
        exported
    }
}
