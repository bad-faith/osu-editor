use super::super::helpers::get_key_value_pairs;

pub struct DifficultySection {
    pub hp: f64,
    pub cs: f64,
    pub od: f64,
    pub ar: f64,
    pub slider_multiplier: f64,
    pub slider_tick_rate: f64,
}

impl DifficultySection {
    pub fn to_osu_text(&self) -> String {
        format!(
            "HPDrainRate:{}\nCircleSize:{}\nOverallDifficulty:{}\nApproachRate:{}\nSliderMultiplier:{}\nSliderTickRate:{}\n",
            self.hp, self.cs, self.od, self.ar, self.slider_multiplier, self.slider_tick_rate
        )
    }
}

pub fn parse_difficulty_section(section: &str) -> Option<DifficultySection> {
    let pairs = get_key_value_pairs(section);
    let pairs = match pairs {
        Some(p) => p,
        None => {
            println!("Failed to parse difficulty section due to duplicate keys.");
            return None;
        }
    };
    let hp = match pairs.get("HPDrainRate") {
        Some(hp) => match hp.parse::<f64>() {
            Ok(value) => value,
            Err(_) => {
                println!("Difficulty parsing error: 'HPDrainRate' is not a valid number.");
                return None;
            }
        },
        None => {
            println!("Difficulty parsing error: Missing 'HPDrainRate' field.");
            return None;
        }
    };

    let cs = match pairs.get("CircleSize") {
        Some(cs) => match cs.parse::<f64>() {
            Ok(value) => value,
            Err(_) => {
                println!("Difficulty parsing error: 'CircleSize' is not a valid number.");
                return None;
            }
        },
        None => {
            println!("Difficulty parsing error: Missing 'CircleSize' field.");
            return None;
        }
    };

    let od = match pairs.get("OverallDifficulty") {
        Some(od) => match od.parse::<f64>() {
            Ok(value) => value,
            Err(_) => {
                println!("Difficulty parsing error: 'OverallDifficulty' is not a valid number.");
                return None;
            }
        },
        None => {
            println!("Difficulty parsing error: Missing 'OverallDifficulty' field.");
            return None;
        }
    };

    let ar = match pairs.get("ApproachRate") {
        Some(ar) => match ar.parse::<f64>() {
            Ok(value) => value,
            Err(_) => {
                println!("Difficulty parsing error: 'ApproachRate' is not a valid number.");
                return None;
            }
        },
        None => {
            println!("Difficulty parsing error: Missing 'ApproachRate' field.");
            return None;
        }
    };

    let slider_multiplier = match pairs.get("SliderMultiplier") {
        Some(sm) => match sm.parse::<f64>() {
            Ok(value) => value,
            Err(_) => {
                println!("Difficulty parsing error: 'SliderMultiplier' is not a valid number.");
                return None;
            }
        },
        None => {
            println!("Difficulty parsing error: Missing 'SliderMultiplier' field.");
            return None;
        }
    };

    let slider_tick_rate = match pairs.get("SliderTickRate") {
        Some(str) => match str.parse::<f64>() {
            Ok(value) => value,
            Err(_) => {
                println!("Difficulty parsing error: 'SliderTickRate' is not a valid number.");
                return None;
            }
        },
        None => {
            println!("Difficulty parsing error: Missing 'SliderTickRate' field.");
            return None;
        }
    };

    return Some(DifficultySection {
        hp,
        cs,
        od,
        ar,
        slider_multiplier,
        slider_tick_rate,
    });
}
