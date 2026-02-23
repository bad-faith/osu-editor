use serde::{Deserialize, Serialize};

use crate::dotosu::sections::general::GeneralSection;

#[derive(Serialize, Deserialize, Clone)]
pub struct General {
    pub countdown: bool,
    pub sample_set: String,
    pub mode: u8,
    pub letterbox_in_breaks: bool,
    pub epilepsy_warning: bool,
    pub widescreen_storyboard: bool,
}

impl General {
    pub fn from_osu_format(general: &GeneralSection) -> Self {
        General {
            countdown: general.countdown,
            sample_set: general.sample_set.clone(),
            mode: general.mode,
            letterbox_in_breaks: general.letterbox_in_breaks,
            epilepsy_warning: general.epilepsy_warning,
            widescreen_storyboard: general.widescreen_storyboard,
        }
    }
}
