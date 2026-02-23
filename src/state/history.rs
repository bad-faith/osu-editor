use std::{sync::Arc, time};

use super::map_state::MapState;

pub struct HistoryNode {
    map_state: Option<Arc<MapState>>,
    parent: Option<usize>,
    childs: Vec<usize>,
    uuid: u128,
    display_name: Option<String>,

    checkpoint: CheckPointInfo,
    created_at: time::Instant,
}

pub enum CheckPointInfo {
    CheckPoint,
    CheckPointAfter(time::Duration),
}

pub struct History {
    nodes: Vec<HistoryNode>,
    reusable_ids: Vec<usize>,
    current_node: usize,
    current_node_depth: usize,
    next_uuid: u128,
}

pub struct StateStats{
    pub uuid: u128,
    pub display_name: Option<String>,
    pub created_at: time::Instant,
}

pub struct UndoRedoInfo {
    pub prev_state: Option<StateStats>,
    pub current_state: StateStats,
    pub next_states: Vec<StateStats>,
}

fn invalidated_state() -> HistoryNode {
    HistoryNode {
        map_state: None,
        parent: None,
        childs: Vec::new(),
        uuid: 0,
        display_name: Some("invalidated".to_string()),
        checkpoint: CheckPointInfo::CheckPoint,
        created_at: time::Instant::now(),
    }
}

impl History {
    pub fn new(map_state: Arc<MapState>) -> Self {
        return Self {
            nodes: vec![HistoryNode {
                map_state: Some(map_state),
                parent: None,
                childs: Vec::new(),
                uuid: 0,
                display_name: "beginning".to_string().into(),
                checkpoint: CheckPointInfo::CheckPoint,
                created_at: time::Instant::now(),
            }],
            reusable_ids: Vec::new(),
            current_node: 0,
            current_node_depth: 0,
            next_uuid: 1,
        };
    }

    pub fn undo(&mut self) -> bool {
        match self.nodes[self.current_node].parent {
            Some(parent_id) => {
                self.current_node = parent_id;
                self.current_node_depth -= 1;
                return true;
            }
            None => return false,
        }
    }

    pub fn undo_redo_info(&self) -> UndoRedoInfo {
        let prev_state = self.nodes[self.current_node].parent.map(|id| StateStats {
            uuid: self.nodes[id].uuid,
            display_name: self.nodes[id].display_name.clone(),
            created_at: self.nodes[id].created_at,
        });
        let current_state = StateStats {
            uuid: self.nodes[self.current_node].uuid,
            display_name: self.nodes[self.current_node].display_name.clone(),
            created_at: self.nodes[self.current_node].created_at,
        };
        let next_states: Vec<StateStats> = self.nodes[self.current_node]
            .childs
            .iter()
            .map(|&id| StateStats {
                uuid: self.nodes[id].uuid,
                display_name: self.nodes[id].display_name.clone(),
                created_at: self.nodes[id].created_at,
            })
            .collect();
        return UndoRedoInfo {
            prev_state,
            current_state,
            next_states,
        };
    }

    pub fn redo(&mut self, uuid: Option<u128>) -> bool {
        for i in self.nodes[self.current_node].childs.iter().rev() {
            if uuid.is_none() || self.nodes[*i].uuid == uuid.unwrap() {
                self.current_node = *i;
                self.current_node_depth += 1;
                return true;
            }
        }
        return false;
    }

    pub fn append(&mut self, map_state: Arc<MapState>, checkpoint: CheckPointInfo) {
        self.pop_uncheckpointed();
        match self.reusable_ids.pop() {
            Some(id) => {
                self.nodes[id] = HistoryNode {
                    map_state: Some(map_state),
                    parent: Some(self.current_node),
                    childs: Vec::new(),
                    checkpoint,
                    created_at: time::Instant::now(),
                    uuid: self.next_uuid,
                    display_name: None,
                };
                self.nodes[self.current_node].childs.push(id);
                self.current_node = id;
                self.current_node_depth += 1;
                self.next_uuid += 1;
                return;
            }
            None => {
                self.nodes.push(HistoryNode {
                    map_state: Some(map_state),
                    parent: Some(self.current_node),
                    childs: Vec::new(),
                    checkpoint,
                    created_at: time::Instant::now(),
                    uuid: self.next_uuid,
                    display_name: None,
                });
                let new_index = self.nodes.len() - 1;
                self.nodes[self.current_node].childs.push(new_index);
                self.current_node = new_index;
                self.current_node_depth += 1;
                self.next_uuid += 1;
                return;
            }
        };
    }

    pub fn get_current_state(&self) -> Arc<MapState> {
        return Arc::clone(self.nodes[self.current_node].map_state.as_ref().unwrap());
    }

    pub fn name_current_state(&mut self, name: String) {
        self.nodes[self.current_node].display_name = Some(name);
    }

    pub fn get_current_state_depth(&self) -> usize {
        return self.current_node_depth;
    }

    pub fn save_checkpoint(&mut self) {
        self.nodes[self.current_node].checkpoint = CheckPointInfo::CheckPoint;
    }

    fn pop_uncheckpointed(&mut self) {
        loop {
            let current_node = &mut self.nodes[self.current_node];
            if current_node.childs.len() != 0 {
                return;
            }
            match current_node.checkpoint {
                CheckPointInfo::CheckPoint => return,
                CheckPointInfo::CheckPointAfter(duration) => {
                    if current_node.created_at.elapsed() >= duration {
                        return;
                    }
                }
            }
            match current_node.parent {
                Some(parent_id) => {
                    self.reusable_ids.push(self.current_node);
                    *current_node = invalidated_state();
                    self.nodes[parent_id]
                        .childs
                        .retain(|&id| id != self.current_node);
                    self.current_node = parent_id;
                    self.current_node_depth -= 1;
                }
                None => return,
            }
        }
    }
}
