use crate::geometry::vec2::Vec2;

pub struct HitObjectsSection {
    pub objects: Vec<HitObject>,
}

impl HitObjectsSection {
    pub fn to_osu_text(&self) -> String {
        let mut text = String::new();
        for object in &self.objects {
            text = format!("{}{}", text, object.to_osu_text());
        }
        return text;
    }
}

pub fn parse_objects_section(section_text: &str) -> Option<HitObjectsSection> {
    let mut objects: Vec<HitObject> = Vec::new();
    for line in section_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 5 {
            println!("Invalid hit object line: '{}'", line);
            return None;
        }
        let x = match parts[0].trim().parse::<f64>() {
            Ok(x) => x,
            Err(_) => {
                println!("Invalid x coordinate in hit object line: '{}'", line);
                return None;
            }
        };
        let y = match parts[1].trim().parse::<f64>() {
            Ok(y) => y,
            Err(_) => {
                println!("Invalid y coordinate in hit object line: '{}'", line);
                return None;
            }
        };
        let time = match parts[2].trim().parse::<f64>() {
            Ok(t) => t,
            Err(_) => {
                println!("Invalid time in hit object line: '{}'", line);
                return None;
            }
        };
        let obj_type = match parts[3].trim().parse::<u8>() {
            Ok(type_bits) => match Type::from_bits(type_bits) {
                Some(t) => t,
                None => {
                    println!("Invalid type in hit object line: '{}'", line);
                    return None;
                }
            },
            Err(_) => {
                println!("Invalid type in hit object line: '{}'", line);
                return None;
            }
        };
        let hitsound = match parts[4].trim().parse::<u8>() {
            Ok(hitsound_bits) => Hitsound::from_bits(hitsound_bits),
            Err(_) => {
                println!("Invalid hitsound in hit object line: '{}'", line);
                return None;
            }
        };
        if obj_type.circle {
            let hitsample = if parts.len() >= 6 {
                match parse_hit_sample(parts[5]) {
                    Some(hs) => hs,
                    None => {
                        println!("Invalid hit sample in hit object line: '{}'", line);
                        return None;
                    }
                }
            } else {
                HitSample {
                    normal_set: 0,
                    addition_set: 0,
                    index: 0,
                    volume: 0,
                    filename: "".to_string(),
                }
            };
            objects.push(HitObject::Circle(Circle {
                pos: Vec2 { x, y },
                time,
                hitsound,
                hitsample,
                combo_info: obj_type.combo_info,
            }));
        } else if obj_type.slider {
            if parts.len() < 8 {
                println!("Invalid slider hit object line: '{}'", line);
                return None;
            }
            let curve_str = parts[5].trim();
            let curve_parts: Vec<&str> = curve_str.split('|').collect();
            if curve_parts.len() < 2 {
                println!("Invalid slider curve in hit object line: '{}'", line);
                return None;
            }
            let curve_type = curve_parts[0].to_string();
            let mut curve_points: Vec<Vec2> = Vec::new();
            for point_str in &curve_parts[1..] {
                let coords: Vec<&str> = point_str.split(':').collect();
                if coords.len() != 2 {
                    println!("Invalid slider curve point in hit object line: '{}'", line);
                    return None;
                }
                let px = match coords[0].trim().parse::<f64>() {
                    Ok(px) => px,
                    Err(_) => {
                        println!(
                            "Invalid slider curve x coordinate in hit object line: '{}'",
                            line
                        );
                        return None;
                    }
                };
                let py = match coords[1].trim().parse::<f64>() {
                    Ok(py) => py,
                    Err(_) => {
                        println!(
                            "Invalid slider curve y coordinate in hit object line: '{}'",
                            line
                        );
                        return None;
                    }
                };
                curve_points.push(Vec2 { x: px, y: py });
            }

            let slides = match parts[6].trim().parse::<u64>() {
                Ok(s) => s,
                Err(_) => {
                    println!("Invalid slider slides in hit object line: '{}'", line);
                    return None;
                }
            };

            let length = match parts[7].trim().parse::<f64>() {
                Ok(l) => l,
                Err(_) => {
                    println!("Invalid slider length in hit object line: '{}'", line);
                    return None;
                }
            };

            let mut edge_sounds = Vec::new();
            let mut edge_sets = Vec::new();
            if parts.len() >= 9 {
                for edge_sound in parts[8].trim().split('|') {
                    match edge_sound.trim().parse::<u8>() {
                        Ok(bits) => {
                            edge_sounds.push(Hitsound::from_bits(bits));
                        }
                        Err(err) => {
                            println!(
                                "Invalid slider edge sound in hit object line '{}': edge_sound={} err={}",
                                line, edge_sound, err
                            );
                            return None;
                        }
                    }
                }
            } else {
                println!(
                    "Cannot find edge sounds; Only {} parts in slider hit object line: '{}'",
                    parts.len(),
                    line
                );
            }

            if parts.len() >= 10 {
                for edge_set in parts[9].trim().split('|') {
                    let sets: Vec<&str> = edge_set.split(':').collect();
                    if sets.len() != 2 {
                        println!("Invalid slider edge set in hit object line: '{}'", line);
                        return None;
                    }
                    let normal_set = match sets[0].trim().parse::<i32>() {
                        Ok(ns) => ns,
                        Err(_) => {
                            println!(
                                "Invalid slider edge set normal set in hit object line: '{}'",
                                line
                            );
                            return None;
                        }
                    };
                    let addition_set = match sets[1].trim().parse::<i32>() {
                        Ok(aset) => aset,
                        Err(_) => {
                            println!(
                                "Invalid slider edge set addition set in hit object line: '{}'",
                                line
                            );
                            return None;
                        }
                    };
                    edge_sets.push(EdgeSet {
                        normal_set,
                        addition_set,
                    });
                }
            } else {
                println!(
                    "Cannot find edge sets; Only {} parts in slider hit object line: '{}'",
                    parts.len(),
                    line
                );
            }

            let hitsample = if parts.len() < 11 {
                HitSample {
                    normal_set: 0,
                    addition_set: 0,
                    index: 0,
                    volume: 0,
                    filename: "".to_string(),
                }
            } else {
                match parse_hit_sample(parts[10]) {
                    Some(hs) => hs,
                    None => {
                        println!("Invalid hit sample in hit object line: '{}'", line);
                        return None;
                    }
                }
            };

            objects.push(HitObject::Slider(Slider {
                pos: Vec2 { x, y },
                time,
                curve_type,
                curve_points,
                slides,
                length_pixels: length,
                hitsound,
                edge_sounds,
                edge_sets,
                hitsample,
                combo_info: obj_type.combo_info,
            }));
        } else if obj_type.spinner {
            if parts.len() < 6 {
                println!("Invalid spinner hit object line: '{}'", line);
                return None;
            }
            let end_time = match parts[5].trim().parse::<f64>() {
                Ok(et) => et,
                Err(_) => {
                    println!("Invalid spinner end time in hit object line: '{}'", line);
                    return None;
                }
            };
            let hitsample = if parts.len() >= 7 {
                match parse_hit_sample(parts[6]) {
                    Some(hs) => hs,
                    None => {
                        println!("Invalid hit sample in hit object line: '{}'", line);
                        return None;
                    }
                }
            } else {
                HitSample {
                    normal_set: 0,
                    addition_set: 0,
                    index: 0,
                    volume: 0,
                    filename: "".to_string(),
                }
            };
            objects.push(HitObject::Spinner(Spinner {
                x,
                y,
                time,
                end_time,
                hitsound,
                hitsample,
                combo_info: obj_type.combo_info,
            }));
        } else {
            println!("Unsupported hit object type in line: '{}'", line);
            return None;
        }
    }
    return Some(HitObjectsSection { objects });
}

pub enum HitObject {
    Circle(Circle),
    Slider(Slider),
    Spinner(Spinner),
}

impl HitObject {
    pub fn to_osu_text(&self) -> String {
        let object_type = match self {
            HitObject::Circle(c) => Type {
                circle: true,
                slider: false,
                spinner: false,
                mania_hold: false,
                combo_info: c.combo_info.clone(),
            },
            HitObject::Slider(s) => Type {
                circle: false,
                slider: true,
                spinner: false,
                mania_hold: false,
                combo_info: s.combo_info.clone(),
            },
            HitObject::Spinner(s) => Type {
                circle: false,
                slider: false,
                spinner: true,
                mania_hold: false,
                combo_info: s.combo_info.clone(),
            },
        };
        match self {
            HitObject::Circle(c) => {
                return format!(
                    "{},{},{},{},{},{}\n",
                    c.pos.x,
                    c.pos.y,
                    c.time,
                    object_type.to_bits(),
                    c.hitsound.to_bits(),
                    c.hitsample.to_osu_text()
                );
            }
            HitObject::Slider(s) => {
                let mut curve_str = s.curve_type.clone();
                for pos in &s.curve_points {
                    curve_str = format!("{}|{}:{}", curve_str, pos.x, pos.y);
                }
                let mut edge_sounds_str = String::new();
                for (i, es) in s.edge_sounds.iter().enumerate() {
                    if i > 0 {
                        edge_sounds_str.push('|');
                    }
                    edge_sounds_str.push_str(&es.to_bits().to_string());
                }
                let mut edge_sets_str = String::new();
                for (i, es) in s.edge_sets.iter().enumerate() {
                    if i > 0 {
                        edge_sets_str.push('|');
                    }
                    edge_sets_str.push_str(&format!("{}:{}", es.normal_set, es.addition_set));
                }
                return format!(
                    "{},{},{},{},{},{},{},{},{},{},{}\n",
                    s.pos.x,
                    s.pos.y,
                    s.time,
                    object_type.to_bits(),
                    s.hitsound.to_bits(),
                    curve_str,
                    s.slides,
                    s.length_pixels,
                    edge_sounds_str,
                    edge_sets_str,
                    s.hitsample.to_osu_text()
                );
            }
            HitObject::Spinner(sp) => {
                return format!(
                    "{},{},{},{},{},{},{}\n",
                    sp.x,
                    sp.y,
                    sp.time,
                    object_type.to_bits(),
                    sp.hitsound.to_bits(),
                    sp.end_time,
                    sp.hitsample.to_osu_text()
                );
            }
        }
    }
}

pub struct Circle {
    pub pos: Vec2,
    pub time: f64,
    pub combo_info: ComboInfo,
    pub hitsound: Hitsound,
    pub hitsample: HitSample,
}

pub struct Slider {
    pub pos: Vec2,
    pub time: f64,
    pub curve_type: String,
    pub curve_points: Vec<Vec2>,
    pub slides: u64,
    pub length_pixels: f64,
    pub edge_sounds: Vec<Hitsound>,
    pub edge_sets: Vec<EdgeSet>,
    pub combo_info: ComboInfo,
    pub hitsound: Hitsound,
    pub hitsample: HitSample,
}

pub struct EdgeSet {
    pub normal_set: i32,
    pub addition_set: i32,
}

pub struct Spinner {
    pub x: f64,
    pub y: f64,
    pub time: f64,
    pub end_time: f64,
    pub combo_info: ComboInfo,
    pub hitsound: Hitsound,
    pub hitsample: HitSample,
}

#[derive(Clone)]
pub struct ComboInfo {
    pub new_combo: bool,
    pub color_skip: i8,
}

struct Type {
    pub circle: bool,
    pub slider: bool,
    pub spinner: bool,
    pub mania_hold: bool,
    pub combo_info: ComboInfo,
}

impl Type {
    pub fn from_bits(bits: u8) -> Option<Type> {
        let ret = Type {
            circle: bits & 1 != 0,
            slider: bits & 2 != 0,
            spinner: bits & 8 != 0,
            mania_hold: bits & 128 != 0,
            combo_info: ComboInfo {
                new_combo: bits & 4 != 0,
                color_skip: ((bits >> 4) & 7) as i8,
            },
        };
        if ret.circle as i32 + ret.slider as i32 + ret.spinner as i32 + ret.mania_hold as i32 != 1 {
            println!("Invalid hit object type bits: {}", bits);
            return None;
        }
        return Some(ret);
    }
    pub fn to_bits(&self) -> u8 {
        let mut bits: u8 = 0;
        if self.circle {
            bits |= 1;
        }
        if self.slider {
            bits |= 2;
        }
        if self.combo_info.new_combo {
            bits |= 4;
        }
        if self.spinner {
            bits |= 8;
        }
        bits |= (self.combo_info.color_skip as u8 & 7) << 4;
        if self.mania_hold {
            bits |= 128;
        }
        bits
    }
}

pub struct Hitsound {
    pub normal: bool,
    pub whistle: bool,
    pub finish: bool,
    pub clap: bool,
}

impl Hitsound {
    pub fn from_bits(bits: u8) -> Hitsound {
        Hitsound {
            normal: bits & 1 != 0,
            whistle: bits & 2 != 0,
            finish: bits & 4 != 0,
            clap: bits & 8 != 0,
        }
    }
    pub fn to_bits(&self) -> u8 {
        let mut bits: u8 = 0;
        if self.normal {
            bits |= 1;
        }
        if self.whistle {
            bits |= 2;
        }
        if self.finish {
            bits |= 4;
        }
        if self.clap {
            bits |= 8;
        }
        bits
    }
}

pub struct HitSample {
    pub normal_set: i32,
    pub addition_set: i32,
    pub index: i32,
    pub volume: i32,
    pub filename: String,
}

impl HitSample {
    pub fn to_osu_text(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.normal_set, self.addition_set, self.index, self.volume, self.filename
        )
    }
}

pub fn parse_hit_sample(text: &str) -> Option<HitSample> {
    let text = text.split(":").collect::<Vec<&str>>();
    if text.len() < 4 {
        return None;
    }
    let normal_set = match text[0].trim().parse::<i32>() {
        Ok(ns) => ns,
        Err(_) => {
            println!("Invalid normal set in hit sample: '{}'", text[0]);
            return None;
        }
    };
    let addition_set = match text[1].trim().parse::<i32>() {
        Ok(aset) => aset,
        Err(_) => {
            println!("Invalid addition set in hit sample: '{}'", text[1]);
            return None;
        }
    };
    let index = match text[2].trim().parse::<i32>() {
        Ok(idx) => idx,
        Err(_) => {
            println!("Invalid index in hit sample: '{}'", text[2]);
            return None;
        }
    };
    let volume = match text[3].trim().parse::<i32>() {
        Ok(vol) => vol,
        Err(_) => {
            println!("Invalid volume in hit sample: '{}'", text[3]);
            return None;
        }
    };
    let filename = if text.len() >= 5 {
        text[4].trim().to_string()
    } else {
        "".to_string()
    };
    return Some(HitSample {
        normal_set: normal_set,
        addition_set: addition_set,
        index: index,
        volume: volume,
        filename: filename,
    });
}
