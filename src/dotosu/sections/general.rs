use crate::dotosu::helpers::get_key_value_pairs;

pub struct GeneralSection {
    pub audio_filename: String,
    pub audio_lead_in: f64,
    pub preview_time: i64,
    pub countdown: bool,
    pub sample_set: String,
    pub stack_leniency: f64,
    pub mode: u8,
    pub letterbox_in_breaks: bool,
    pub epilepsy_warning: bool,
    pub widescreen_storyboard: bool,
}

impl GeneralSection {
    pub fn to_osu_text(&self) -> String {
        format!(
            "AudioFilename:{}\nAudioLeadIn:{}\nPreviewTime:{}\nCountdown:{}\nSampleSet:{}\nStackLeniency:{}\nMode:{}\nLetterboxInBreaks:{}\nEpilepsyWarning:{}\nWidescreenStoryboard:{}\n",
            self.audio_filename,
            self.audio_lead_in,
            self.preview_time,
            if self.countdown { 1 } else { 0 },
            self.sample_set,
            self.stack_leniency,
            self.mode,
            if self.letterbox_in_breaks { 1 } else { 0 },
            if self.epilepsy_warning { 1 } else { 0 },
            if self.widescreen_storyboard { 1 } else { 0 }
        )
    }
}

pub fn parse_general_section(section: &str) -> Option<GeneralSection> {
    let pairs = get_key_value_pairs(section);
    let pairs = match pairs {
        Some(p) => p,
        None => {
            println!("Failed to parse general section due to duplicate keys.");
            return None;
        }
    };
    let audio_filename = match pairs.get("AudioFilename") {
        None => {
            println!("General parsing error: Missing 'AudioFilename' field.");
            return None;
        }
        Some(audio_filename) => audio_filename.to_string(),
    };
    let audio_lead_in = match pairs.get("AudioLeadIn") {
        None => {
            println!("General parsing error: Missing 'AudioLeadIn' field.");
            return None;
        }
        Some(audio_lead_in_str) => match audio_lead_in_str.parse::<f64>() {
            Ok(val) => val,
            Err(err) => {
                println!(
                    "General parsing error: 'AudioLeadIn'={} is not a valid f64: {}",
                    audio_lead_in_str, err
                );
                return None;
            }
        },
    };
    let preview_time = match pairs.get("PreviewTime") {
        None => {
            println!("General parsing error: Missing 'PreviewTime' field.");
            return None;
        }
        Some(preview_time_str) => match preview_time_str.parse::<i64>() {
            Ok(val) => val,
            Err(err) => {
                println!(
                    "General parsing error: 'PreviewTime'={} is not a valid i64: {}",
                    preview_time_str, err
                );
                return None;
            }
        },
    };
    let countdown = match pairs.get("Countdown") {
        None => {
            println!("General parsing error: Missing 'Countdown' field.");
            return None;
        }
        Some(countdown_str) => match countdown_str.parse::<u8>() {
            Ok(val) => val != 0,
            Err(err) => {
                println!(
                    "General parsing error: 'Countdown'={} is not a valid u8: {}",
                    countdown_str, err
                );
                return None;
            }
        },
    };
    let sample_set = match pairs.get("SampleSet") {
        None => {
            println!("General parsing error: Missing 'SampleSet' field.");
            return None;
        }
        Some(sample_set) => sample_set.to_string(),
    };
    let stack_leniency = match pairs.get("StackLeniency") {
        None => {
            println!("General parsing error: Missing 'StackLeniency' field.");
            return None;
        }
        Some(stack_leniency_str) => match stack_leniency_str.parse::<f64>() {
            Ok(val) => val,
            Err(err) => {
                println!(
                    "General parsing error: 'StackLeniency'={} is not a valid f64: {}",
                    stack_leniency_str, err
                );
                return None;
            }
        },
    };
    let mode = match pairs.get("Mode") {
        None => {
            println!("General parsing error: Missing 'Mode' field.");
            return None;
        }
        Some(mode_str) => match mode_str.parse::<u8>() {
            Ok(val) => val,
            Err(err) => {
                println!(
                    "General parsing error: 'Mode'={} is not a valid u8: {}",
                    mode_str, err
                );
                return None;
            }
        },
    };
    let letterbox_in_breaks = match pairs.get("LetterboxInBreaks") {
        Some(val) => match val.parse::<u8>() {
            Ok(v) => v != 0,
            Err(err) => {
                println!(
                    "General parsing error: 'LetterboxInBreaks'={} is not a valid u8: {}",
                    val, err
                );
                return None;
            }
        },
        None => false,
    };
    let epilepsy_warning = match pairs.get("EpilepsyWarning") {
        Some(val) => match val.parse::<u8>() {
            Ok(v) => v != 0,
            Err(err) => {
                println!(
                    "General parsing error: 'EpilepsyWarning'={} is not a valid u8: {}",
                    val, err
                );
                return None;
            }
        },
        None => false,
    };
    let widescreen_storyboard = match pairs.get("WidescreenStoryboard") {
        Some(val) => match val.parse::<u8>() {
            Ok(v) => v != 0,
            Err(err) => {
                println!(
                    "General parsing error: 'WidescreenStoryboard'={} is not a valid u8: {}",
                    val, err
                );
                return None;
            }
        },
        None => false,
    };
    return Some(GeneralSection {
        audio_filename,
        audio_lead_in,
        preview_time,
        countdown,
        sample_set,
        stack_leniency: stack_leniency,
        mode: mode,
        letterbox_in_breaks: letterbox_in_breaks,
        epilepsy_warning: epilepsy_warning,
        widescreen_storyboard: widescreen_storyboard,
    });
}
