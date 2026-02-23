use std::sync::{Arc, OnceLock};

use crate::{
    config::Config,
    geometry::vec2_transform::Vec2Transform,
    map_format::{
        colors::Color,
        diff_settings::DiffSettings,
        objects::HitObject,
        timing::TimingPoint,
    },
    treap::Treap,
};

use super::{hitsound_export::HitsoundExport, object::Object};

#[derive(Clone)]
pub struct MapState {
    pub objects: Treap<Object>,
    pub red_lines: Treap<f64>,
    pub bookmarks: Treap<f64>,
    pub kiai_times: Treap<(f64, f64)>,
    pub break_times: Treap<(f64, f64)>,
    pub combo_colors: Vec<Color>,
    pub diff_settings: DiffSettings,
    pub config: Config,
}

impl MapState {
    pub fn new(
        objects: Vec<HitObject>,
        timing: Vec<TimingPoint>,
        bookmarks: Vec<f64>,
        kiai_times: Vec<(f64, f64)>,
        break_times: Vec<(f64, f64)>,
        combo_colors: Vec<Color>,
        diff_settings: DiffSettings,
        config: Config,
    ) -> Self {
        let objects: Vec<Object> = objects
            .into_iter()
            .map(|hit_object| {
                let object = Object {
                    hit_object: Arc::new(hit_object),
                    instance: Arc::new(OnceLock::new()),
                };
                object.instance_or_calculate(&diff_settings, &config);
                return object;
            })
            .collect();
        let red_lines: Vec<f64> = timing
            .iter()
            .filter_map(|f| match f {
                TimingPoint::RedLine(r) => Some(r.time),
                _ => None,
            })
            .collect();
        Self {
            objects: Treap::from_slice(objects.as_slice()),
            red_lines: Treap::from_slice(red_lines.as_slice()),
            bookmarks: Treap::from_slice(bookmarks.as_slice()),
            kiai_times: Treap::from_slice(kiai_times.as_slice()),
            break_times: Treap::from_slice(break_times.as_slice()),
            combo_colors: combo_colors.clone(),
            diff_settings,
            config,
        }
    }

    pub fn export(&self) {
        for object in self.objects.iter() {
            object.instance_or_calculate(&self.diff_settings, &self.config);
        }
    }

    pub fn export_hitsounds(&self, export_into: &mut HitsoundExport) {
        export_into.hitsounds.clear();
        for object in self.objects.iter() {
            let instance = object.instance_or_calculate(&self.diff_settings, &self.config);
            match &*object.hit_object {
                HitObject::Circle(circle) => {
                    export_into.hitsounds.push((
                        circle.time,
                        instance.pos.x / 512.0,
                        circle.hitsound_info.clone(),
                    ));
                }
                HitObject::Slider(slider) => {
                    let start_x = instance.pos.x / 512.0;
                    let end_x = instance
                        .slider_path
                        .as_ref()
                        .map(|path| path.ridge.end_point().x / 512.0)
                        .unwrap_or(start_x);

                    for (i, hitsound) in slider.hitsounds.iter().enumerate() {
                        let position_x = if i % 2 == 0 { start_x } else { end_x };
                        export_into.hitsounds.push((
                            slider.time + slider.slide_duration() * i as f64,
                            position_x,
                            hitsound.clone(),
                        ));
                    }
                }
                HitObject::Spinner(_) => {}
            }
        }
    }

    pub fn transform_objects(&self, transform: Vec2Transform, ids: &[usize]) -> MapState {
        let mut map_state = self.clone();
        for id in ids {
            map_state.objects = map_state.objects.mutate(*id, |object| {
                let mut object = object.clone();
                let mut hit_object = (*object.hit_object).clone();
                hit_object.apply_transform(transform);
                object.hit_object = Arc::new(hit_object);
                object.instance = Arc::new(OnceLock::new());
                return object;
            });
        }
        return map_state;
    }
}
