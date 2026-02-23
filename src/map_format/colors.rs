use serde::{Deserialize, Serialize};

use crate::dotosu::sections::colours::ColoursSection;

#[derive(Serialize, Deserialize, Clone)]
pub struct Colors {
    pub combo_colors: Vec<Color>,
}

impl Colors {
    pub fn from_osu_format(osu_colors: &ColoursSection) -> Self {
        Colors {
            combo_colors: osu_colors
                .colors
                .iter()
                .map(|c| Color {
                    r: c.r,
                    g: c.g,
                    b: c.b,
                })
                .collect(),
        }
    }
    pub fn to_osu_format(&self) -> ColoursSection {
        ColoursSection {
            colors: self
                .combo_colors
                .iter()
                .map(|c| crate::dotosu::sections::colours::Colour {
                    r: c.r,
                    g: c.g,
                    b: c.b,
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}
