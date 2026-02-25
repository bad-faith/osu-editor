use std::sync::{Arc, OnceLock};

use crate::{
    config::Config,
    geometry::vec2::Vec2,
    gpu::gpu::ObjectInstance,
    map_format::{
        diff_settings::DiffSettings,
        objects::HitObject,
        slider_boxing::BBox,
    },
};

#[derive(Clone)]
pub struct Object {
    pub hit_object: Arc<HitObject>,
    pub instance: Arc<OnceLock<ObjectInstance>>,
}

impl Object {
    pub fn instance(&self) -> Option<&ObjectInstance> {
        self.instance.get()
    }

    pub fn instance_or_calculate(
        &self,
        diff_settings: &DiffSettings,
        config: &Config,
    ) -> &ObjectInstance {
        self.instance
            .get_or_init(|| self.calculate_instance(diff_settings, config))
    }

    pub fn calculate_instance(
        &self,
        diff_settings: &DiffSettings,
        config: &Config,
    ) -> ObjectInstance {
        let combo_info = self.hit_object.combo_info();
        match &*self.hit_object {
            HitObject::Circle(circle) => {
                return ObjectInstance {
                    pos: circle.pos,
                    radius: diff_settings.circle_radius,
                    time: circle.time,
                    preempt: diff_settings.preempt_period,
                    is_new_combo: combo_info.new_combo,
                    is_slider: false,
                    is_spinner: false,
                    slider_path: None,
                    slider_slide_duration_ms: 0.0,
                    slider_length_px: 0.0,
                    slider_end_time_ms: circle.time,
                    slides: 0,
                    bbox_inner: BBox {
                        x: [circle.pos.x, circle.pos.x],
                        y: [circle.pos.y, circle.pos.y],
                    },
                    snap_points: vec![circle.pos],
                    timeline_start_ms: circle.time,
                    timeline_end_ms: circle.time,
                    timeline_repeat_ms: Vec::new(),
                };
            }
            HitObject::Slider(slider) => {
                let (slider_ridge, snap_points) = slider
                    .control_points
                    .construct_curve_and_snap_points(slider.length_pixels);
                let bbox = slider_ridge.calculate_bbox_inner();
                return ObjectInstance {
                    pos: slider.control_points.start,
                    radius: diff_settings.circle_radius,
                    time: slider.time,
                    preempt: diff_settings.preempt_period,
                    is_new_combo: combo_info.new_combo,
                    is_slider: true,
                    is_spinner: false,
                    slider_path: Some(slider_ridge.construct_boxes(
                        diff_settings.circle_radius,
                        config.appearance.layout.slider_outer_thickness,
                    )),
                    slider_length_px: slider.length_pixels,
                    slider_end_time_ms: slider.end_time(),
                    slider_slide_duration_ms: slider.slide_duration(),
                    slides: slider.slides,
                    bbox_inner: bbox,
                    snap_points,
                    timeline_start_ms: slider.time,
                    timeline_end_ms: slider.end_time(),
                    timeline_repeat_ms: {
                        let mut repeats = Vec::new();
                        if slider.slides > 1 {
                            let slide_duration = slider.slide_duration();
                            for repeat_i in 1..slider.slides {
                                repeats.push(slider.time + slide_duration * repeat_i as f64);
                            }
                        }
                        repeats
                    },
                };
            }
            HitObject::Spinner(spinner) => {
                return ObjectInstance {
                    pos: Vec2 { x: 256.0, y: 192.0 },
                    radius: diff_settings.circle_radius,
                    time: spinner.time,
                    preempt: diff_settings.preempt_period,
                    is_new_combo: combo_info.new_combo,
                    is_slider: false,
                    is_spinner: true,
                    slider_path: None,
                    slider_slide_duration_ms: 0.0,
                    slider_length_px: 0.0,
                    slider_end_time_ms: spinner.end_time,
                    slides: 0,
                    bbox_inner: BBox {
                        x: [256.0, 256.0],
                        y: [192.0, 192.0],
                    },
                    snap_points: vec![Vec2 { x: 256.0, y: 192.0 }],
                    timeline_start_ms: spinner.time,
                    timeline_end_ms: spinner.end_time,
                    timeline_repeat_ms: Vec::new(),
                };
            }
        }
    }
}
