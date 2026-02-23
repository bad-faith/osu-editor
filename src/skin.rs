use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

use crate::files::scan_folder;

#[derive(Serialize, Deserialize, Clone)]
pub struct Skin {
    pub cursor: Texture,

    pub hit_circle: Texture,
    pub hit_circle_overlay: Texture,

    pub slider_start_circle: Texture,
    pub slider_start_circle_overlay: Texture,

    pub reverse_arrow: Texture,
    pub slider_ball: Vec<Texture>,
    pub slider_follow_circle: Texture,

    pub sliderend_circle: Texture,
    pub sliderend_circle_overlay: Texture,
    pub spinner_circle: Texture,

    pub follow_point: Texture,

    pub approach_circle: Texture,
    pub numbers: Vec<Texture>,

    pub hitsounds: HashMap<String, Vec<u8>>,
}

impl Skin {
    pub fn load_from_path(path: &Path, default_path: &Path) -> Option<Self> {
        let hit_circle = load_skin_texture(path, default_path, "hitcircle")?;
        let hit_circle_overlay = load_skin_texture(path, default_path, "hitcircleoverlay")?;

        let slider_start_circle = match load_skin_texture(path, default_path, "sliderstartcircle") {
            Some(tex) => tex,
            None => hit_circle.clone(),
        };

        let slider_start_circle_overlay =
            match load_skin_texture(path, default_path, "sliderstartcircleoverlay") {
                Some(tex) => tex,
                None => hit_circle_overlay.clone(),
            };

        let sliderend_circle = load_skin_texture(path, default_path, "sliderendcircle");
        let sliderend_circle_overlay =
            match load_skin_texture(path, default_path, "sliderendcircleoverlay") {
                Some(tex) => tex,
                None => {
                    if sliderend_circle.is_some() {
                        hit_circle_overlay.clone()
                    } else {
                        Texture {
                            rgba: vec![],
                            width: 1,
                            height: 1,
                            is_2x: false,
                        }
                    }
                }
            };
        let sliderend_circle = match sliderend_circle {
            Some(tex) => tex,
            None => Texture {
                rgba: vec![],
                width: 1,
                height: 1,
                is_2x: false,
            },
        };

        let default_hitsound_files = scan_folder(
            default_path,
            Some(false),
            Some(&vec![".wav", ".ogg", ".mp3"]),
        );
        let default_hitsound_files = default_hitsound_files
            .into_iter()
            .filter_map(|p| {
                let path = default_path.join(&p);
                let bytes = std::fs::read(&path).ok()?;
                Some((p, bytes))
            })
            .collect::<HashMap<String, Vec<u8>>>();

        let hitsound_files = scan_folder(path, Some(false), Some(&vec![".wav", ".ogg", ".mp3"]));
        let mut hitsound_files = hitsound_files
            .into_iter()
            .filter_map(|p| {
                let path = path.join(&p);
                let bytes = std::fs::read(&path).ok()?;
                Some((p, bytes))
            })
            .collect::<HashMap<String, Vec<u8>>>();

        for (name, bytes) in &default_hitsound_files {
            let name_no_ext = match name.rfind('.') {
                Some(idx) => &name[..idx],
                None => name.as_str(),
            };
            if !hitsound_files.contains_key(&(name_no_ext.to_string() + ".wav"))
                && !hitsound_files.contains_key(&(name_no_ext.to_string() + ".ogg"))
                && !hitsound_files.contains_key(&(name_no_ext.to_string() + ".mp3"))
            {
                log!(
                    "Hitsound {} not found in skin, using default skin hitsound",
                    name
                );
                hitsound_files.insert(name.clone(), bytes.clone());
            }
        }

        println!("Loaded {} hitsounds from skin.", hitsound_files.len());

        Some(Skin {
            cursor: load_skin_texture(path, default_path, "cursor")?,
            hit_circle: hit_circle,
            hit_circle_overlay: hit_circle_overlay,
            approach_circle: load_skin_texture(path, default_path, "approachcircle")?,
            numbers: load_skin_animation_texture(path, default_path, "default")?,
            reverse_arrow: load_skin_texture(path, default_path, "reversearrow")?,
            follow_point: load_skin_texture(path, default_path, "followpoint-0")
                .or(load_skin_texture(path, default_path, "followpoint"))?,
            slider_ball: load_skin_animation_texture(path, default_path, "sliderb")?,
            slider_follow_circle: load_skin_texture(path, default_path, "sliderfollowcircle")?,
            slider_start_circle: slider_start_circle,
            slider_start_circle_overlay: slider_start_circle_overlay,
            sliderend_circle: sliderend_circle,
            sliderend_circle_overlay: sliderend_circle_overlay,
            spinner_circle: load_skin_texture(path, default_path, "spinner-circle").unwrap_or(Texture {
                rgba: vec![],
                width: 1,
                height: 1,
                is_2x: false,
            }),
            hitsounds: hitsound_files,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Texture {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub is_2x: bool,
}

fn load_skin_animation_texture(
    skin_path: &Path,
    fallback_path: &Path,
    texture_name: &str,
) -> Option<Vec<Texture>> {
    match try_load_skin_animation_texture(skin_path, texture_name) {
        Some(texs) => return Some(texs),
        None => {
            log!(
                "Falling back to default skin for animated texture {}",
                texture_name
            );
            return try_load_skin_animation_texture(fallback_path, texture_name);
        }
    }
}

fn try_load_skin_animation_texture(skin_path: &Path, texture_name: &str) -> Option<Vec<Texture>> {
    let mut textures = Vec::new();
    for frame_idx in 0.. {
        let frame_texture_name = format!("{}-{}", texture_name, frame_idx);
        let frame_texture_name1 = format!("{}{}", texture_name, frame_idx);
        let tex1 = try_load_skin_texture(skin_path, &frame_texture_name);
        let tex2 = try_load_skin_texture(skin_path, &frame_texture_name1);
        match tex1.or(tex2) {
            Some(tex) => textures.push(tex),
            None => break,
        }
    }
    if textures.is_empty() {
        return match try_load_skin_texture(skin_path, texture_name) {
            Some(tex) => Some(vec![tex]),
            None => None,
        };
    } else {
        return Some(textures);
    }
}

fn load_skin_texture(
    skin_path: &Path,
    fallback_path: &Path,
    texture_name: &str,
) -> Option<Texture> {
    match try_load_skin_texture(skin_path, texture_name) {
        Some(tex) => return Some(tex),
        None => {
            log!("Falling back to default skin for texture {}", texture_name);
            return try_load_skin_texture(fallback_path, texture_name);
        }
    }
}

fn try_load_skin_texture(skin_path: &Path, texture_name: &str) -> Option<Texture> {
    let texture_2x = skin_path.join(format!("{}@2x.png", texture_name));
    match load_texture_from_path(&texture_2x) {
        Some(tex) => {
            return Some(Texture { is_2x: true, ..tex });
        }
        None => {
            log!(
                "2x texture for {} not found, falling back to 1x texture, path = {}",
                texture_name,
                texture_2x.display()
            );
        }
    };
    let texture_1x = skin_path.join(format!("{}.png", texture_name));
    match load_texture_from_path(&texture_1x) {
        Some(tex) => {
            return Some(Texture {
                is_2x: false,
                ..tex
            });
        }
        None => {
            log!(
                "texture {} not found in skin, path = {}",
                texture_name,
                texture_1x.display()
            );
            return None;
        }
    }
}

fn load_texture_from_path(path: &Path) -> Option<Texture> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            log!("Failed to read texture from {}: {}", path.display(), e);
            return None;
        }
    };
    return load_texture(&bytes);
}

pub fn load_texture(bytes: &[u8]) -> Option<Texture> {
    let img = match image::load_from_memory(bytes) {
        Ok(i) => i,
        Err(e) => {
            log!("Failed to load image from memory: {}", e);
            return None;
        }
    };
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    return Some(Texture {
        rgba: rgba.into_raw(),
        width,
        height,
        is_2x: false,
    });
}
