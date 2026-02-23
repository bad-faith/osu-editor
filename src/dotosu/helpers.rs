use std::collections::HashMap;

pub fn get_section<'a>(osu_text: &'a str, section_name: &'a str) -> Option<&'a str> {
    let section_header = format!("[{}]", section_name);
    let start = osu_text.find(&section_header)? + section_header.len();
    let end = osu_text[start..]
        .find('[')
        .map_or(osu_text.len(), |e| start + e);
    Some(&osu_text[start..end].trim())
}

pub fn get_key_value_pairs(section: &str) -> Option<HashMap<String, String>> {
    let mut pairs = HashMap::new();
    for line in section.lines() {
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        match parts.len() {
            0 => continue,
            2 => {
                let key = parts[0].trim();
                let value = parts[1].trim();
                match pairs.insert(key.to_string(), value.to_string()) {
                    Some(_) => {
                        println!("Warning: Duplicate key '{}' found in section.", key);
                        return None;
                    }
                    None => {}
                }
            }
            _ => {
                println!(
                    "Warning: Malformed line '{}' in section, unexpected format.",
                    line
                );
                return None;
            }
        }
    }
    return Some(pairs);
}
