use serde::{Deserialize, Serialize};

use crate::dotosu::sections::{difficulty::DifficultySection, general::GeneralSection};

#[derive(Serialize, Deserialize, Clone)]
pub struct DiffSettings {
    pub circle_radius: f64,
    pub preempt_period: f64,
    pub overall_difficulty: f64,
    pub health_drain: f64,
    pub sv_multiplier: f64,
    pub tick_rate: f64,
    pub stacking_period: f64,
}

impl DiffSettings {
    pub fn from_osu_format(diff_settings: &DifficultySection, general: &GeneralSection) -> Self {
        let preempt_period = preempt_period_from_ar(diff_settings.ar);
        DiffSettings {
            circle_radius: circle_radius_from_cs(diff_settings.cs),
            preempt_period: preempt_period,
            overall_difficulty: diff_settings.od,
            health_drain: diff_settings.hp,
            sv_multiplier: diff_settings.slider_multiplier,
            tick_rate: diff_settings.slider_tick_rate,
            stacking_period: general.stack_leniency * preempt_period,
        }
    }
    pub fn to_osu_format(&self) -> DifficultySection {
        DifficultySection {
            cs: circle_radius_to_cs(self.circle_radius),
            ar: preempt_period_to_ar(self.preempt_period),
            od: self.overall_difficulty,
            hp: self.health_drain,
            slider_multiplier: self.sv_multiplier,
            slider_tick_rate: self.tick_rate,
        }
    }
}

pub fn circle_radius_from_cs(cs: f64) -> f64 {
    54.4 - 4.48 * cs
}

pub fn circle_radius_to_cs(circle_radius: f64) -> f64 {
    (54.4 - circle_radius) / 4.48
}

pub fn preempt_period_to_ar(preempt_period: f64) -> f64 {
    if preempt_period > 1200.0 {
        (1800.0 - preempt_period) / 120.0
    } else if preempt_period == 1200.0 {
        5.0
    } else {
        (1200.0 - preempt_period) / 150.0 + 5.0
    }
}

pub fn preempt_period_from_ar(ar: f64) -> f64 {
    if ar < 5.0 {
        1800.0 - ar * 120.0
    } else if ar == 5.0 {
        1200.0
    } else {
        1200.0 - (ar - 5.0) * 150.0
    }
}
