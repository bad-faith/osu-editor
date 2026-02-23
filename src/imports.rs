use std::{
    fs,
    path::{Path, PathBuf},
};
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage, imageops::FilterType};
use winit::event_loop::EventLoop;

use crate::{
    EDITOR_VERSION,
    dotosu::osu_file::{OsuFile, parse_osu_file},
    files::{extract_zip, sanitize_name, write_bytes_to_file},
    dialogue_app::DialogueApp,
    map_format::{
        beatmap::Beatmap, beatmapset::Beatmapset,
        convert_from_osu_format::convert_osu_beatmapset_to_internal,
    },
    scan_folder,
};

pub fn select_and_import_map(event_loop: &mut EventLoop<()>, selector: &mut DialogueApp) {
    println!("Importing map...");
    let imports_path = Path::new("imports");
    if !imports_path.exists() {
        println!("No imports/ directory found.");
        return;
    }

    let entries = scan_folder(imports_path, Some(false), Some(&vec![".osz"]));

    if entries.is_empty() {
        println!("No maps found in imports/");
        return;
    }

    // --- Step 2: console selection ---
    println!("Available maps:");
    let selection = match selector.select(event_loop, "Import map (.osz)", &entries) {
        Some(idx) => idx,
        None => {
            println!("Import cancelled.");
            return;
        }
    };
    let selected_map = &entries[selection];
    println!("Importing: {}", selected_map);
    import_osz(selected_map, event_loop, selector);
}

pub fn select_and_import_skin(event_loop: &mut EventLoop<()>, selector: &mut DialogueApp) {
    println!("Importing skin...");
    let imports_path = Path::new("imports");
    if !imports_path.exists() {
        println!("No imports/ directory found.");
        return;
    }

    let entries = scan_folder(imports_path, Some(false), Some(&vec![".osk"]));

    if entries.is_empty() {
        println!("No maps found in imports/");
        return;
    }

    // --- Step 2: console selection ---
    println!("Available skins:");
    let selection = match selector.select(event_loop, "Import skin (.osk)", &entries) {
        Some(idx) => idx,
        None => {
            println!("Import cancelled.");
            return;
        }
    };
    let selected_skin = &entries[selection];
    println!("Importing: {}", selected_skin);
    import_osk(selected_skin, event_loop, selector);
}

fn import_osk(selected_skin: &str, event_loop: &mut EventLoop<()>, selector: &mut DialogueApp) {
    let import_path = Path::new("imports/").join(selected_skin);
    let osk_bytes = match fs::read(&import_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!("Failed to read file {}: {}", selected_skin, err);
            return;
        }
    };

    let extracted = match extract_zip(osk_bytes) {
        Some(files) => files,
        None => {
            println!("Failed to extract .osk file: {}", selected_skin);
            return;
        }
    };
    let selected_skin = selected_skin.trim_end_matches(".osk");
    import_osk_files(selected_skin, extracted, event_loop, selector);
}

fn import_osz(selected_map: &str, event_loop: &mut EventLoop<()>, selector: &mut DialogueApp) {
    let import_path = Path::new("imports/").join(selected_map);
    let osz_bytes = match fs::read(&import_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!("Failed to read file {}: {}", selected_map, err);
            return;
        }
    };

    let extracted = match extract_zip(osz_bytes) {
        Some(files) => files,
        None => {
            println!("Failed to extract .osz file: {}", selected_map);
            return;
        }
    };

    let osu_files: Vec<(String, Vec<u8>)> = extracted
        .iter()
        .filter(|(name, _)| name.to_ascii_lowercase().ends_with(".osu"))
        .cloned()
        .collect();

    let assets: Vec<(String, Vec<u8>)> = extracted
        .iter()
        .filter(|(name, _)| !name.to_ascii_lowercase().ends_with(".osu"))
        .cloned()
        .collect();

    let parsed_osu_files = match parse_osu_files(osu_files.clone(), event_loop, selector) {
        Some(files) => files,
        None => {
            println!("Failed to parse .osu files in {}", selected_map);
            return;
        }
    };
    let (beatmapset, beatmaps) = match convert_osu_beatmapset_to_internal(&parsed_osu_files) {
        Some((beatmapset, beatmaps)) => (beatmapset, beatmaps),
        None => {
            println!(
                "Failed to convert .osu files to internal format for {}",
                selected_map
            );
            return;
        }
    };
    import_osz_files(beatmapset, beatmaps, osu_files, assets, event_loop, selector);
}

fn parse_osu_files(
    osu_files: Vec<(String, Vec<u8>)>,
    event_loop: &mut EventLoop<()>,
    selector: &mut DialogueApp,
) -> Option<Vec<OsuFile>> {
    let mut parsed_osu_files = vec![];
    for (name, data) in osu_files {
        let mut prompt_missing_value = |prompt: &str| -> Option<String> {
            selector.prompt_text(event_loop, "Missing metadata", prompt)
        };
        let osu_file = parse_osu_file(name.clone(), data.as_slice(), &mut prompt_missing_value);
        match osu_file {
            Some(osu_file) => {
                parsed_osu_files.push(osu_file);
            }
            None => {
                println!("Failed to parse .osu file: {}", name);
                return None;
            }
        }
    }
    Some(parsed_osu_files)
}

fn import_osz_assets(save_path: PathBuf, assets: Vec<(String, Vec<u8>)>) {
    let assets_path = save_path.join("assets");
    for (name, data) in assets {
        let asset_file_path = assets_path.join(name);
        if let Err(err) = write_bytes_to_file(&asset_file_path, data.as_slice()) {
            println!(
                "Failed to write asset file {}: {}",
                asset_file_path.display(),
                err
            );
            return;
        }
    }

    log!("Successfully imported assets to {}", assets_path.display());
}

fn import_osz_osu_files(save_path: PathBuf, osu_files: Vec<(String, Vec<u8>)>) {
    let diffs_path = save_path.join("imported_diffs");
    for (name, contents) in osu_files {
        let osu_file_path = diffs_path.join(&name);
        if let Err(err) = write_bytes_to_file(&osu_file_path, &contents) {
            println!(
                "Failed to write .osu file {}: {}",
                osu_file_path.display(),
                err
            );
            return;
        }
    }

    log!(
        "Successfully imported .osu files to {}",
        diffs_path.display()
    );
}

fn import_beatmapset(save_path: PathBuf, beatmapset: Beatmapset) {
    let beatmapset_path = save_path.join("beatmapset.json");
    let beatmapset_json = match serde_json::to_string_pretty(&beatmapset) {
        Ok(json) => json,
        Err(err) => {
            println!("Failed to serialize beatmapset to JSON: {}", err);
            return;
        }
    };
    if let Err(err) = write_bytes_to_file(&beatmapset_path, beatmapset_json.as_bytes()) {
        println!(
            "Failed to write beatmapset file {}: {}",
            beatmapset_path.display(),
            err
        );
        return;
    }
    println!(
        "Successfully imported beatmapset to {}",
        beatmapset_path.display()
    );
}

fn normalize_asset_name(name: &str) -> String {
    let mut normalized = name.trim().replace('\\', "/");
    if normalized.starts_with('"') && normalized.ends_with('"') && normalized.len() >= 2 {
        normalized = normalized[1..normalized.len() - 1].to_string();
    }
    normalized.strip_prefix("./").unwrap_or(&normalized).to_string()
}

fn find_asset_bytes_by_name<'a>(assets: &'a [(String, Vec<u8>)], file_name: &str) -> Option<&'a [u8]> {
    let normalized = normalize_asset_name(file_name);
    let normalized_lower = normalized.to_ascii_lowercase();
    let normalized_base = Path::new(&normalized)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(normalized.as_str())
        .to_ascii_lowercase();

    for (asset_name, asset_bytes) in assets {
        let asset_normalized = normalize_asset_name(asset_name);
        if asset_normalized.eq_ignore_ascii_case(&normalized) {
            return Some(asset_bytes.as_slice());
        }
        if asset_normalized.to_ascii_lowercase() == normalized_lower {
            return Some(asset_bytes.as_slice());
        }

        let asset_base = Path::new(&asset_normalized)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(asset_normalized.as_str())
            .to_ascii_lowercase();
        if asset_base == normalized_base {
            return Some(asset_bytes.as_slice());
        }
    }

    None
}

fn make_bg_small_png(background_bytes: Option<&[u8]>) -> Vec<u8> {
    const OUT_SIZE: u32 = 128;

    let mut canvas = RgbaImage::from_pixel(OUT_SIZE, OUT_SIZE, image::Rgba([0, 0, 0, 255]));

    if let Some(bytes) = background_bytes {
        if let Ok(decoded) = image::load_from_memory(bytes) {
            let (src_w, src_h) = decoded.dimensions();
            if src_w > 0 && src_h > 0 {
                let filled = decoded
                    .resize_to_fill(OUT_SIZE, OUT_SIZE, FilterType::Triangle)
                    .to_rgba8();
                image::imageops::overlay(&mut canvas, &filled, 0, 0);
            }
        }
    }

    let mut out = std::io::Cursor::new(Vec::new());
    let _ = DynamicImage::ImageRgba8(canvas).write_to(&mut out, ImageFormat::Png);
    out.into_inner()
}

fn import_beatmaps(save_path: PathBuf, beatmaps: Vec<Beatmap>, assets: &[(String, Vec<u8>)]) {
    let diffs_path = save_path.join("diffs");
    for beatmap in beatmaps {
        let diff_path = diffs_path.join(format!("{}", sanitize_name(&beatmap.version)));
        let bg_name = beatmap.events.background_name();
        let bg_bytes = if bg_name.is_empty() {
            None
        } else {
            find_asset_bytes_by_name(assets, bg_name.as_str())
        };
        let bg_small_png = make_bg_small_png(bg_bytes);
        import_beatmap(diff_path, beatmap, bg_small_png.as_slice());
    }
}

fn import_beatmap(diff_path: PathBuf, beatmap: Beatmap, bg_small_png: &[u8]) {
    let beatmap_path = diff_path.join("beatmap.json");
    let beatmap_json = match serde_json::to_string_pretty(&beatmap) {
        Ok(json) => json,
        Err(err) => {
            println!("Failed to serialize beatmap to JSON: {}", err);
            return;
        }
    };
    if let Err(err) = write_bytes_to_file(&beatmap_path, beatmap_json.as_bytes()) {
        println!(
            "Failed to write beatmap file {}: {}",
            beatmap_path.display(),
            err
        );
        return;
    }

    let bg_small_path = diff_path.join("bg_small.png");
    if let Err(err) = write_bytes_to_file(&bg_small_path, bg_small_png) {
        println!(
            "Failed to write diff background preview {}: {}",
            bg_small_path.display(),
            err
        );
        return;
    }

    log!(
        "Successfully imported beatmap to {}",
        beatmap_path.display()
    );
}

fn import_osz_files(
    beatmapset: Beatmapset,
    beatmaps: Vec<Beatmap>,
    osu_files: Vec<(String, Vec<u8>)>,
    assets: Vec<(String, Vec<u8>)>,
    event_loop: &mut EventLoop<()>,
    selector: &mut DialogueApp,
) {
    let artist = beatmapset.artist.clone();
    let title = beatmapset.title.clone();
    let creator = beatmapset.creator.clone();
    let map_dir_name_raw = format!("v{} {} - {} ({})", EDITOR_VERSION, artist, title, creator);
    let map_dir_name = sanitize_name(&map_dir_name_raw);
    let save_path = Path::new("saves/").join(&map_dir_name);
    if save_path.exists() {
        match selector.confirm(
            event_loop,
            &format!("Map directory {} already exists. Overwrite?", map_dir_name),
        ) {
            true => {
                println!("Overwriting existing map directory...");
                if let Err(err) = fs::remove_dir_all(&save_path) {
                    println!(
                        "Failed to remove existing map directory {}: {}",
                        map_dir_name, err
                    );
                    return;
                } else {
                    log!(
                        "Successfully removed existing map directory {}.",
                        map_dir_name
                    );
                }
            }
            false => {
                println!("Import cancelled.");
                return;
            }
        }
    }

    let assets_for_bg_small = assets.clone();
    import_osz_assets(save_path.clone(), assets);
    import_osz_osu_files(save_path.clone(), osu_files);
    import_beatmapset(save_path.clone(), beatmapset);
    import_beatmaps(save_path.clone(), beatmaps, &assets_for_bg_small);
}

fn import_osk_files(
    skin_name: &str,
    assets: Vec<(String, Vec<u8>)>,
    event_loop: &mut EventLoop<()>,
    selector: &mut DialogueApp,
) {
    let skin_path = Path::new("skins/").join(skin_name);
    if skin_path.exists() {
        match selector.confirm(
            event_loop,
            &format!("Skin directory {} already exists. Overwrite?", skin_name),
        ) {
            true => {
                println!("Overwriting existing skin directory...");
                if let Err(err) = fs::remove_dir_all(&skin_path) {
                    println!(
                        "Failed to remove existing skin directory {}: {}",
                        skin_name, err
                    );
                    return;
                } else {
                    log!(
                        "Successfully removed existing skin directory {}.",
                        skin_name
                    );
                }
            }
            false => {
                println!("Import cancelled.");
                return;
            }
        }
    }
    for (name, data) in assets {
        let asset_file_path = skin_path.join(name);
        if let Err(err) = write_bytes_to_file(&asset_file_path, data.as_slice()) {
            println!(
                "Failed to write asset file {}: {}",
                asset_file_path.display(),
                err
            );
            return;
        }
    }

    println!("Successfully imported a skin to {}", skin_path.display());
}
