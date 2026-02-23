use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Timing {
    pub timing_points: Vec<TimingPoint>,
}

impl Timing {
    pub fn from_osu_format(
        osu_timing: &crate::dotosu::sections::timing::TimingSection,
        general: &crate::dotosu::sections::general::GeneralSection,
    ) -> Self {
        let default_sample_set = match general.sample_set.as_str() {
            "Normal" => SampleSet::Normal,
            "Soft" => SampleSet::Soft,
            "Drum" => SampleSet::Drum,
            _ => SampleSet::Normal,
        };
        Timing {
            timing_points: osu_timing
                .timing_points
                .iter()
                .map(|tp| TimingPoint::from_osu_format(tp, default_sample_set.clone()))
                .collect(),
        }
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::timing::TimingSection {
        crate::dotosu::sections::timing::TimingSection {
            timing_points: self
                .timing_points
                .iter()
                .map(|tp| tp.to_osu_format())
                .collect(),
        }
    }

    pub fn get_lines_at_time(&self, time: f64) -> (Option<RedLine>, Option<GreenLine>) {
        let mut red_line: Option<RedLine> = None;
        let mut green_line: Option<GreenLine> = None;

        for tp in &self.timing_points {
            match tp {
                TimingPoint::RedLine(rl) => {
                    if rl.time <= time {
                        red_line = Some(rl.clone());
                        green_line = None; // Red line resets green line
                    }
                }
                TimingPoint::GreenLine(gl) => {
                    if gl.time <= time {
                        green_line = Some(gl.clone());
                    }
                }
            }
        }

        (red_line, green_line)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum TimingPoint {
    RedLine(RedLine),
    GreenLine(GreenLine),
}

impl TimingPoint {
    pub fn from_osu_format(
        tp: &crate::dotosu::sections::timing::TimingPoint,
        default_sample_set: SampleSet,
    ) -> Self {
        match tp {
            crate::dotosu::sections::timing::TimingPoint::RedLine(rl) => {
                TimingPoint::RedLine(RedLine::from_osu_format(rl, default_sample_set))
            }
            crate::dotosu::sections::timing::TimingPoint::GreenLine(gl) => {
                TimingPoint::GreenLine(GreenLine::from_osu_format(gl, default_sample_set))
            }
        }
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::timing::TimingPoint {
        match self {
            TimingPoint::RedLine(rl) => {
                crate::dotosu::sections::timing::TimingPoint::RedLine(rl.to_osu_format())
            }
            TimingPoint::GreenLine(gl) => {
                crate::dotosu::sections::timing::TimingPoint::GreenLine(gl.to_osu_format())
            }
        }
    }
    pub fn time(&self) -> f64 {
        match self {
            TimingPoint::RedLine(rl) => rl.time,
            TimingPoint::GreenLine(gl) => gl.time,
        }
    }
    pub fn effects(&self) -> &TimingPointEffect {
        match self {
            TimingPoint::RedLine(rl) => &rl.effects,
            TimingPoint::GreenLine(gl) => &gl.effects,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RedLine {
    pub time: f64,
    pub beat_length: f64,
    pub meter: i32,
    pub sample_set: SampleSet,
    pub sample_index: i32,
    pub volume: f64,
    pub effects: TimingPointEffect,
}

impl RedLine {
    pub fn from_osu_format(
        rl: &crate::dotosu::sections::timing::RedLine,
        default_sample_set: SampleSet,
    ) -> Self {
        RedLine {
            time: rl.time,
            beat_length: rl.beat_length,
            meter: rl.meter,
            sample_set: match rl.sample_set {
                1 => SampleSet::Normal,
                2 => SampleSet::Soft,
                3 => SampleSet::Drum,
                _ => default_sample_set,
            },
            sample_index: rl.sample_index,
            volume: (rl.volume as f64) / 100.0,
            effects: TimingPointEffect::from_osu_format(&rl.effects),
        }
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::timing::RedLine {
        crate::dotosu::sections::timing::RedLine {
            time: self.time,
            beat_length: self.beat_length,
            meter: self.meter,
            sample_set: match self.sample_set {
                SampleSet::Normal => 1,
                SampleSet::Soft => 2,
                SampleSet::Drum => 3,
            },
            sample_index: self.sample_index,
            volume: (self.volume * 100.0).round() as i8,
            effects: self.effects.to_osu_format(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GreenLine {
    pub time: f64,
    pub sv_multiplier: f64,
    pub sample_set: SampleSet,
    pub sample_index: i32,
    pub volume: f64,
    pub effects: TimingPointEffect,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SampleSet {
    Normal,
    Soft,
    Drum,
}

impl GreenLine {
    pub fn from_osu_format(
        gl: &crate::dotosu::sections::timing::GreenLine,
        default_sample_set: SampleSet,
    ) -> Self {
        let sample_set = match gl.sample_set {
            1 => SampleSet::Normal,
            2 => SampleSet::Soft,
            3 => SampleSet::Drum,
            _ => default_sample_set,
        };
        log!("GreenLine sample_set: {:?}", sample_set);
        GreenLine {
            time: gl.time,
            sv_multiplier: gl.sv_multiplier,
            sample_set,
            sample_index: gl.sample_index,
            volume: (gl.volume as f64) / 100.0,
            effects: TimingPointEffect::from_osu_format(&gl.effects),
        }
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::timing::GreenLine {
        crate::dotosu::sections::timing::GreenLine {
            time: self.time,
            sv_multiplier: self.sv_multiplier,
            sample_set: match self.sample_set {
                SampleSet::Normal => 1,
                SampleSet::Soft => 2,
                SampleSet::Drum => 3,
            },
            sample_index: self.sample_index,
            volume: (self.volume * 100.0).round() as i8,
            effects: self.effects.to_osu_format(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TimingPointEffect {
    pub kiai_mode: bool,
    pub omit_first_barline: bool,
}

impl TimingPointEffect {
    pub fn from_osu_format(t: &crate::dotosu::sections::timing::TimingPointEffect) -> Self {
        TimingPointEffect {
            kiai_mode: t.kiai_mode,
            omit_first_barline: t.omit_first_barline,
        }
    }
    pub fn to_osu_format(&self) -> crate::dotosu::sections::timing::TimingPointEffect {
        crate::dotosu::sections::timing::TimingPointEffect {
            kiai_mode: self.kiai_mode,
            omit_first_barline: self.omit_first_barline,
        }
    }
}
