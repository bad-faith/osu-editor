use serde::{Deserialize, Serialize};

use crate::{
    geometry::{vec2::Vec2, vec2_transform::Vec2Transform},
    map_format::{slider_curve::ControlPoints, stacking::apply_stacking, timing::SampleSet},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Objects {
    pub objects: Vec<HitObject>,
}

impl Objects {
    pub fn from_osu_format(
        osu_objects: &crate::dotosu::sections::objects::HitObjectsSection,
        timing: &crate::map_format::timing::Timing,
        difficulty: &crate::map_format::diff_settings::DiffSettings,
    ) -> Option<Self> {
        let objects = match osu_objects
            .objects
            .iter()
            .map(|x| HitObject::from_osu_format(x, timing, difficulty))
            .collect::<Option<Vec<_>>>()
        {
            Some(objs) => objs,
            None => {
                println!("Failed to convert some hitobjects from osu! format.");
                return None;
            }
        };
        let objects = apply_stacking(
            &objects,
            difficulty.stacking_period,
            difficulty.circle_radius,
        );
        Some(Objects { objects })
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::objects::HitObjectsSection {
        crate::dotosu::sections::objects::HitObjectsSection {
            objects: self.objects.iter().map(HitObject::to_osu_format).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HitObject {
    Circle(Circle),
    Slider(Slider),
    Spinner(Spinner),
}

impl HitObject {
    pub fn apply_transform(&mut self, transform: Vec2Transform) {
        match self {
            HitObject::Circle(c) => {
                c.pos = c.pos * transform;
            }
            HitObject::Slider(s) => {
                let prev_size = s.control_points.size();
                s.control_points = s.control_points.apply_transform(transform);
                let new_size = s.control_points.size();
                let size_ratio = if prev_size > 1e-9 && new_size > 1e-9 {
                    new_size / prev_size
                } else {
                    1.0
                };
                s.length_pixels *= size_ratio;
                s.sv_pixels_per_ms *= size_ratio;
            }
            HitObject::Spinner(_) => {}
        }
    }
    pub fn from_osu_format(
        osu_object: &crate::dotosu::sections::objects::HitObject,
        timing: &crate::map_format::timing::Timing,
        difficulty: &crate::map_format::diff_settings::DiffSettings,
    ) -> Option<Self> {
        match osu_object {
            crate::dotosu::sections::objects::HitObject::Circle(c) => {
                Some(HitObject::Circle(Circle::from_osu_format(c, timing)?))
            }
            crate::dotosu::sections::objects::HitObject::Slider(s) => Some(HitObject::Slider(
                Slider::from_osu_format(s, timing, difficulty)?,
            )),
            crate::dotosu::sections::objects::HitObject::Spinner(sp) => {
                Some(HitObject::Spinner(Spinner::from_osu_format(sp)))
            }
        }
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::objects::HitObject {
        match self {
            HitObject::Circle(c) => {
                crate::dotosu::sections::objects::HitObject::Circle(c.to_osu_format())
            }
            HitObject::Slider(s) => {
                crate::dotosu::sections::objects::HitObject::Slider(s.to_osu_format())
            }
            HitObject::Spinner(sp) => {
                crate::dotosu::sections::objects::HitObject::Spinner(sp.to_osu_format())
            }
        }
    }

    pub fn combo_info(&self) -> &ComboInfo {
        match self {
            HitObject::Circle(c) => &c.combo_info,
            HitObject::Slider(s) => &s.combo_info,
            HitObject::Spinner(sp) => &sp.combo_info,
        }
    }

    pub fn move_by_offset(&self, offset: Vec2) -> HitObject {
        match self {
            HitObject::Circle(c) => {
                let pos = c.pos + offset;

                let mut new_circle = c.clone();
                new_circle.pos = pos;
                HitObject::Circle(new_circle)
            }
            HitObject::Slider(s) => {
                let control_points = s.control_points.move_by_offset(offset);

                let mut new_slider = s.clone();
                new_slider.control_points = control_points;
                return HitObject::Slider(new_slider);
            }
            HitObject::Spinner(sp) => {
                return HitObject::Spinner(sp.clone());
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Circle {
    pub pos: Vec2,
    pub time: f64,
    pub combo_info: ComboInfo,
    pub hitsound_info: HitsoundInfo,
}

impl Circle {
    pub fn from_osu_format(
        osu_circle: &crate::dotosu::sections::objects::Circle,
        timing: &crate::map_format::timing::Timing,
    ) -> Option<Self> {
        let (red_line, green_line) = timing.get_lines_at_time(osu_circle.time);
        let red_line = match red_line {
            Some(rl) => rl,
            None => {
                println!("No red line found at time {} for circle.", osu_circle.time);
                return None;
            }
        };
        let default_sampleset = match &green_line {
            Some(gl) => gl.sample_set.clone(),
            None => red_line.sample_set,
        };

        let default_volume = match &green_line {
            Some(gl) => gl.volume,
            None => red_line.volume,
        };
        let hit_sampleset = match osu_circle.hitsample.normal_set {
            1 => SampleSet::Normal,
            2 => SampleSet::Soft,
            3 => SampleSet::Drum,
            _ => default_sampleset.clone(),
        };
        let additions_sampleset = match osu_circle.hitsample.addition_set {
            1 => SampleSet::Normal,
            2 => SampleSet::Soft,
            3 => SampleSet::Drum,
            _ => hit_sampleset.clone(),
        };
        Some(Circle {
            pos: osu_circle.pos,
            time: osu_circle.time,
            combo_info: ComboInfo::from_osu_format(&osu_circle.combo_info),
            hitsound_info: HitsoundInfo {
                hit_sampleset,
                additions_sampleset,
                volume: if osu_circle.hitsample.volume == 0 {
                    default_volume
                } else {
                    (osu_circle.hitsample.volume as f64) / 100.0
                },
                index: osu_circle.hitsample.index,
                play_whistle: osu_circle.hitsound.whistle,
                play_finish: osu_circle.hitsound.finish,
                play_clap: osu_circle.hitsound.clap,
                filename: if osu_circle.hitsample.filename.is_empty() {
                    None
                } else {
                    Some(osu_circle.hitsample.filename.clone())
                },
            },
        })
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::objects::Circle {
        crate::dotosu::sections::objects::Circle {
            pos: self.pos,
            time: self.time,
            combo_info: crate::dotosu::sections::objects::ComboInfo {
                new_combo: self.combo_info.new_combo,
                color_skip: self.combo_info.color_skip as i8,
            },
            hitsound: crate::dotosu::sections::objects::Hitsound {
                normal: true,
                whistle: self.hitsound_info.play_whistle,
                finish: self.hitsound_info.play_finish,
                clap: self.hitsound_info.play_clap,
            },
            hitsample: crate::dotosu::sections::objects::HitSample {
                normal_set: match self.hitsound_info.hit_sampleset {
                    SampleSet::Normal => 1,
                    SampleSet::Soft => 2,
                    SampleSet::Drum => 3,
                },
                addition_set: match self.hitsound_info.additions_sampleset {
                    SampleSet::Normal => 1,
                    SampleSet::Soft => 2,
                    SampleSet::Drum => 3,
                },
                index: 0,
                volume: ((self.hitsound_info.volume * 100.0).round() as i32).clamp(0, 100),
                filename: match &self.hitsound_info.filename {
                    Some(name) => name.clone(),
                    None => "".to_string(),
                },
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Slider {
    pub time: f64,
    pub slides: u64,
    pub length_pixels: f64,
    pub sv_pixels_per_ms: f64,
    pub combo_info: ComboInfo,

    pub hitsounds: Vec<HitsoundInfo>, // len = slides + 1
    pub sliderbody_hitsound: HitsoundInfo,

    pub control_points: ControlPoints,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HitsoundInfo {
    pub hit_sampleset: SampleSet,
    pub additions_sampleset: SampleSet,
    pub volume: f64,

    pub index: i32,

    pub play_whistle: bool,
    pub play_finish: bool,
    pub play_clap: bool,

    pub filename: Option<String>,
}

impl Slider {
    pub fn slide_duration(&self) -> f64 {
        self.length_pixels / self.sv_pixels_per_ms
    }

    pub fn from_osu_format(
        osu_slider: &crate::dotosu::sections::objects::Slider,
        timing: &crate::map_format::timing::Timing,
        difficulty: &crate::map_format::diff_settings::DiffSettings,
    ) -> Option<Self> {
        let control_points = match ControlPoints::from_osu_format(osu_slider) {
            Some(path) => path,
            None => {
                println!("Failed to convert slider path from osu! format.");
                ControlPoints {
                    start: Vec2 { x: 0.0, y: 0.0 },
                    slider_segments: vec![],
                }
            }
        };

        let (red_line, green_line) = timing.get_lines_at_time(osu_slider.time);

        let red_line = match red_line {
            Some(rl) => rl,
            None => {
                println!("No red line found at time {} for slider.", osu_slider.time);
                return None;
            }
        };

        let base_sv = difficulty.sv_multiplier;
        let sv_multiplier = match &green_line {
            Some(gl) => gl.sv_multiplier,
            None => 1.0,
        };

        let default_sampleset = match &green_line {
            Some(gl) => gl.sample_set.clone(),
            None => red_line.sample_set,
        };

        let default_volume = match &green_line {
            Some(gl) => gl.volume,
            None => red_line.volume,
        };
        let hit_sampleset = match osu_slider.hitsample.normal_set {
            1 => SampleSet::Normal,
            2 => SampleSet::Soft,
            3 => SampleSet::Drum,
            _ => default_sampleset.clone(),
        };
        let additions_sampleset = match osu_slider.hitsample.addition_set {
            1 => SampleSet::Normal,
            2 => SampleSet::Soft,
            3 => SampleSet::Drum,
            _ => hit_sampleset.clone(),
        };
        let sliderbody_hitsound_info = HitsoundInfo {
            hit_sampleset,
            additions_sampleset,
            volume: if osu_slider.hitsample.volume == 0 {
                default_volume
            } else {
                (osu_slider.hitsample.volume as f64) / 100.0
            },
            index: osu_slider.hitsample.index,
            play_whistle: osu_slider.hitsound.whistle,
            play_finish: osu_slider.hitsound.finish,
            play_clap: osu_slider.hitsound.clap,
            filename: if osu_slider.hitsample.filename.is_empty() {
                None
            } else {
                Some(osu_slider.hitsample.filename.clone())
            },
        };
        let sv_pixels_per_ms = (base_sv * 100.0 * sv_multiplier) / red_line.beat_length;

        let mut hitsounds = vec![];
        for i in 0..=osu_slider.slides {
            let time_ms =
                osu_slider.time + (osu_slider.length_pixels / sv_pixels_per_ms) * (i as f64);
            let (red_line, green_line) = timing.get_lines_at_time(time_ms + 0.5); // small offset to avoid edge cases
            let red_line = match red_line {
                Some(rl) => rl,
                None => {
                    println!(
                        "No red line found at time {} for slider hitobject.",
                        time_ms
                    );
                    return None;
                }
            };
            let default_sampleset = match &green_line {
                Some(gl) => gl.sample_set.clone(),
                None => red_line.sample_set.clone(),
            };
            let osu_edge_sound = osu_slider.edge_sounds.get(i as usize);
            let osu_edge_set = osu_slider.edge_sets.get(i as usize);
            let hit_sampleset = match osu_edge_set {
                Some(es) => match es.normal_set {
                    1 => SampleSet::Normal,
                    2 => SampleSet::Soft,
                    3 => SampleSet::Drum,
                    _ => default_sampleset.clone(),
                },
                None => default_sampleset.clone(),
            };
            let additions_sampleset = match osu_edge_set {
                Some(es) => match es.addition_set {
                    1 => SampleSet::Normal,
                    2 => SampleSet::Soft,
                    3 => SampleSet::Drum,
                    _ => hit_sampleset.clone(),
                },
                None => hit_sampleset.clone(),
            };
            let hitsound_info = HitsoundInfo {
                hit_sampleset,
                additions_sampleset,
                volume: match &green_line {
                    Some(gl) => gl.volume,
                    None => red_line.volume,
                },
                index: match green_line {
                    Some(gl) => gl.sample_index,
                    None => red_line.sample_index,
                },
                play_whistle: match osu_edge_sound {
                    Some(es) => es.whistle,
                    None => false,
                },
                play_finish: match osu_edge_sound {
                    Some(es) => es.finish,
                    None => false,
                },
                play_clap: match osu_edge_sound {
                    Some(es) => es.clap,
                    None => false,
                },
                filename: None,
            };
            hitsounds.push(hitsound_info);
        }

        Some(Slider {
            time: osu_slider.time,
            control_points,
            slides: osu_slider.slides,
            length_pixels: osu_slider.length_pixels,
            sv_pixels_per_ms: sv_pixels_per_ms,
            combo_info: ComboInfo::from_osu_format(&osu_slider.combo_info),
            hitsounds,
            sliderbody_hitsound: sliderbody_hitsound_info,
        })
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::objects::Slider {
        let (slider_type, curve_points) = match self.control_points.to_osu_format() {
            Some((stype, points)) => (stype, points),
            None => {
                println!("Failed to convert slider path to osu! format.");
                ("B".to_string(), vec![])
            }
        };
        let edge_sets: Vec<crate::dotosu::sections::objects::EdgeSet> = self
            .hitsounds
            .iter()
            .map(|x| crate::dotosu::sections::objects::EdgeSet {
                normal_set: match x.hit_sampleset {
                    SampleSet::Normal => 1,
                    SampleSet::Soft => 2,
                    SampleSet::Drum => 3,
                },
                addition_set: match x.additions_sampleset {
                    SampleSet::Normal => 1,
                    SampleSet::Soft => 2,
                    SampleSet::Drum => 3,
                },
            })
            .collect();
        let edge_sounds = self
            .hitsounds
            .iter()
            .map(|x| crate::dotosu::sections::objects::Hitsound {
                normal: true,
                whistle: x.play_whistle,
                finish: x.play_finish,
                clap: x.play_clap,
            })
            .collect();
        crate::dotosu::sections::objects::Slider {
            pos: self.control_points.start,
            time: self.time,
            curve_type: slider_type,
            curve_points: curve_points,
            slides: self.slides,
            length_pixels: self.length_pixels,
            edge_sounds: edge_sounds,
            edge_sets: edge_sets,
            combo_info: ComboInfo::to_osu_format(&self.combo_info),
            hitsound: crate::dotosu::sections::objects::Hitsound {
                normal: true,
                whistle: self.sliderbody_hitsound.play_whistle,
                finish: self.sliderbody_hitsound.play_finish,
                clap: self.sliderbody_hitsound.play_clap,
            },
            hitsample: crate::dotosu::sections::objects::HitSample {
                normal_set: match self.sliderbody_hitsound.hit_sampleset {
                    SampleSet::Normal => 1,
                    SampleSet::Soft => 2,
                    SampleSet::Drum => 3,
                },
                addition_set: match self.sliderbody_hitsound.additions_sampleset {
                    SampleSet::Normal => 1,
                    SampleSet::Soft => 2,
                    SampleSet::Drum => 3,
                },
                index: self.sliderbody_hitsound.index,
                volume: (self.sliderbody_hitsound.volume * 100.0).round() as i32,
                filename: match &self.sliderbody_hitsound.filename {
                    Some(name) => name.clone(),
                    None => "".to_string(),
                },
            },
        }
    }

    pub fn end_time(&self) -> f64 {
        return self.time + self.slide_duration() * (self.slides as f64);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Spinner {
    pub x: f64,
    pub y: f64,
    pub time: f64,
    pub end_time: f64,
    pub combo_info: ComboInfo,
    pub hitsound: Hitsound,
    pub hitsample: HitSample,
}

impl Spinner {
    pub fn from_osu_format(osu_spinner: &crate::dotosu::sections::objects::Spinner) -> Self {
        let mut combo_info = ComboInfo::from_osu_format(&osu_spinner.combo_info);
        combo_info.new_combo = true; // Spinners always start a new combo
        Spinner {
            x: osu_spinner.x,
            y: osu_spinner.y,
            time: osu_spinner.time,
            end_time: osu_spinner.end_time,
            combo_info: combo_info,
            hitsound: Hitsound::from_osu_format(&osu_spinner.hitsound),
            hitsample: HitSample::from_osu_format(&osu_spinner.hitsample),
        }
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::objects::Spinner {
        crate::dotosu::sections::objects::Spinner {
            x: self.x,
            y: self.y,
            time: self.time,
            end_time: self.end_time,
            combo_info: ComboInfo::to_osu_format(&self.combo_info),
            hitsound: Hitsound::to_osu_format(&self.hitsound),
            hitsample: HitSample::to_osu_format(&self.hitsample),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ComboInfo {
    pub new_combo: bool,
    pub color_skip: i64,
}

impl ComboInfo {
    pub fn from_osu_format(osu_combo_info: &crate::dotosu::sections::objects::ComboInfo) -> Self {
        ComboInfo {
            new_combo: osu_combo_info.new_combo,
            color_skip: osu_combo_info.color_skip as i64,
        }
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::objects::ComboInfo {
        crate::dotosu::sections::objects::ComboInfo {
            new_combo: self.new_combo,
            color_skip: self.color_skip as i8,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hitsound {
    pub normal: bool,
    pub whistle: bool,
    pub finish: bool,
    pub clap: bool,
}

impl Hitsound {
    pub fn from_osu_format(osu_hitsound: &crate::dotosu::sections::objects::Hitsound) -> Self {
        Hitsound {
            normal: osu_hitsound.normal,
            whistle: osu_hitsound.whistle,
            finish: osu_hitsound.finish,
            clap: osu_hitsound.clap,
        }
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::objects::Hitsound {
        crate::dotosu::sections::objects::Hitsound {
            normal: self.normal,
            whistle: self.whistle,
            finish: self.finish,
            clap: self.clap,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HitSample {
    pub normal_set: i32,
    pub addition_set: i32,
    pub index: i32,
    pub volume: i32,
    pub filename: String,
}

impl HitSample {
    pub fn from_osu_format(osu_hitsample: &crate::dotosu::sections::objects::HitSample) -> Self {
        HitSample {
            normal_set: osu_hitsample.normal_set,
            addition_set: osu_hitsample.addition_set,
            index: osu_hitsample.index,
            volume: osu_hitsample.volume,
            filename: osu_hitsample.filename.clone(),
        }
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::objects::HitSample {
        crate::dotosu::sections::objects::HitSample {
            normal_set: self.normal_set,
            addition_set: self.addition_set,
            index: self.index,
            volume: self.volume,
            filename: self.filename.clone(),
        }
    }
}
