use crate::dotosu::helpers::get_key_value_pairs;

pub struct ColoursSection {
    pub colors: Vec<Colour>,
}

impl ColoursSection {
    pub fn to_osu_text(&self) -> String {
        let mut text = String::new();
        for (i, color) in self.colors.iter().enumerate() {
            text.push_str(&format!(
                "Combo{} : {},{},{}\n",
                i + 1,
                color.r,
                color.g,
                color.b
            ));
        }
        return text;
    }
}

pub fn parse_colours_section(section_text: &str) -> Option<ColoursSection> {
    let pairs = get_key_value_pairs(section_text);
    let pairs = match pairs {
        Some(p) => p,
        None => {
            println!("Failed to parse key-value pairs in [Colours] section.");
            return None;
        }
    };
    let mut colours: Vec<Colour> = Vec::new();
    for i in 0.. {
        let key = format!("Combo{}", i + 1);
        match pairs.get(&key) {
            Some(value) => match parse_colour(value) {
                Some(colour) => colours.push(colour),
                None => {
                    println!(
                        "Failed to parse colour value '{}' for key '{}'.",
                        value, key
                    );
                    return None;
                }
            },
            None => break,
        }
    }
    return Some(ColoursSection { colors: colours });
}

pub struct Colour {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

pub fn parse_colour(colour_str: &str) -> Option<Colour> {
    let parts: Vec<&str> = colour_str.split(',').collect();
    if parts.len() < 3 {
        return None;
    }
    let r = parts[0].trim().parse::<f64>().ok()?;
    let g = parts[1].trim().parse::<f64>().ok()?;
    let b = parts[2].trim().parse::<f64>().ok()?;
    Some(Colour { r, g, b })
}
