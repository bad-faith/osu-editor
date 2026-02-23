pub struct TimingSection {
    pub timing_points: Vec<TimingPoint>,
}

impl TimingSection {
    pub fn to_osu_text(&self) -> String {
        let mut text = String::new();
        for tp in &self.timing_points {
            text = format!("{}{}", text, tp.to_osu_text());
        }
        return text;
    }
}

pub fn parse_timing_section(section_text: &str) -> Option<TimingSection> {
    let mut timing_points: Vec<TimingPoint> = Vec::new();
    for line in section_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 8 {
            println!("Invalid timing point line: '{}'", line);
            return None;
        }
        let time = match parts[0].trim().parse::<f64>() {
            Ok(t) => t,
            Err(_) => {
                println!("Invalid time in timing point line: '{}'", line);
                return None;
            }
        };
        let beat_length = match parts[1].trim().parse::<f64>() {
            Ok(bl) => bl,
            Err(_) => {
                println!("Invalid beat length in timing point line: '{}'", line);
                return None;
            }
        };
        let meter = match parts[2].trim().parse::<i32>() {
            Ok(m) => m,
            Err(_) => {
                println!("Invalid meter in timing point line: '{}'", line);
                return None;
            }
        };
        let sample_set = match parts[3].trim().parse::<i32>() {
            Ok(ss) => ss,
            Err(_) => {
                println!("Invalid sample set in timing point line: '{}'", line);
                return None;
            }
        };
        let sample_index = match parts[4].trim().parse::<i32>() {
            Ok(si) => si,
            Err(_) => {
                println!("Invalid sample index in timing point line: '{}'", line);
                return None;
            }
        };
        let volume = match parts[5].trim().parse::<i8>() {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid volume in timing point line: '{}'", line);
                return None;
            }
        };
        let uninherited = match parts[6].trim().parse::<i32>() {
            Ok(ui) => ui == 1,
            Err(_) => {
                println!("Invalid uninherited flag in timing point line: '{}'", line);
                return None;
            }
        };
        let effects = match parts[7].trim().parse::<i32>() {
            Ok(e) => e,
            Err(_) => {
                println!("Invalid effects in timing point line: '{}'", line);
                return None;
            }
        };
        if uninherited {
            timing_points.push(TimingPoint::RedLine(RedLine {
                time,
                beat_length,
                meter,
                sample_set,
                sample_index,
                volume,
                effects: TimingPointEffect::from_int(effects),
            }));
        } else {
            timing_points.push(TimingPoint::GreenLine(GreenLine {
                time,
                sv_multiplier: -100.0 / beat_length,
                sample_set,
                sample_index,
                volume,
                effects: TimingPointEffect::from_int(effects),
            }));
        }
    }
    return Some(TimingSection { timing_points });
}

pub enum TimingPoint {
    RedLine(RedLine),
    GreenLine(GreenLine),
}

impl TimingPoint {
    pub fn to_osu_text(&self) -> String {
        match self {
            TimingPoint::RedLine(rl) => {
                format!(
                    "{},{},{},{},{},{},1,{}\n",
                    rl.time,
                    rl.beat_length,
                    rl.meter,
                    rl.sample_set,
                    rl.sample_index,
                    rl.volume,
                    rl.effects.to_int(),
                )
            }
            TimingPoint::GreenLine(gl) => {
                format!(
                    "{},{},4,{},{},{},0,{}\n",
                    gl.time,
                    -100.0 / gl.sv_multiplier,
                    gl.sample_set,
                    gl.sample_index,
                    gl.volume,
                    gl.effects.to_int(),
                )
            }
        }
    }
}

pub struct RedLine {
    pub time: f64,
    pub beat_length: f64,
    pub meter: i32,
    pub sample_set: i32,
    pub sample_index: i32,
    pub volume: i8,
    pub effects: TimingPointEffect,
}

pub struct GreenLine {
    pub time: f64,
    pub sv_multiplier: f64,
    pub sample_set: i32,
    pub sample_index: i32,
    pub volume: i8,
    pub effects: TimingPointEffect,
}

pub struct TimingPointEffect {
    pub kiai_mode: bool,
    pub omit_first_barline: bool,
}

impl TimingPointEffect {
    pub fn from_int(effects: i32) -> TimingPointEffect {
        TimingPointEffect {
            kiai_mode: (effects & 1) != 0,
            omit_first_barline: (effects & 8) != 0,
        }
    }
    pub fn to_int(&self) -> i32 {
        let mut effects = 0;
        if self.kiai_mode {
            effects |= 1;
        }
        if self.omit_first_barline {
            effects |= 8;
        }
        effects
    }
}
