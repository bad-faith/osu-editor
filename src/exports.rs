use std::{fs, path::Path};
use winit::event_loop::EventLoop;

use crate::{
    dotosu::osu_file::OsuFile,
    files::{create_zip, open_beatmapset_folder, sanitize_name, scan_folder, write_bytes_to_file},
    dialogue_app::DialogueApp,
    map_format::convert_to_osu_format::convert_internal_to_osu_format,
};

pub fn select_and_export_map(event_loop: &mut EventLoop<()>, selector: &mut DialogueApp) {
    println!("Exporting map...");

    // --- Step 1: scan saves/ ---
    let saves_path = Path::new("saves");
    if !saves_path.exists() {
        println!("No saves/ directory found.");
        return;
    }

    let entries = scan_folder(saves_path, Some(true), None);

    if entries.is_empty() {
        println!("No maps found in saves/");
        return;
    }

    println!("Available maps:");
    let selection = match selector.select(event_loop, "Export map", &entries) {
        Some(idx) => idx,
        None => {
            println!("Export cancelled.");
            return;
        }
    };
    let selected_map = &entries[selection];
    export_map(event_loop, selector, selected_map);
}

pub fn export_map(event_loop: &mut EventLoop<()>, selector: &mut DialogueApp, map_name: &String) {
    println!("Exporting map: {}", map_name);

    let beatmapset_folder = match open_beatmapset_folder(map_name) {
        Some(beatmapset_folder) => beatmapset_folder,
        None => {
            println!("Failed to open beatmapset folder for {}", map_name);
            return;
        }
    };

    let osu_files: Vec<OsuFile> = beatmapset_folder
        .beatmaps
        .into_iter()
        .map(|b| convert_internal_to_osu_format(beatmapset_folder.beatmapset.clone(), b))
        .collect();

    let export_path = format!("saves/{}/exports", map_name);
    let export_path = Path::new(&export_path);
    if export_path.exists() {
        match selector.confirm(
            event_loop,
            &format!("Export path {} already exists. Overwrite?", export_path.display()),
        ) {
            true => {
                if let Err(err) = fs::remove_dir_all(&export_path) {
                    println!(
                        "Failed to remove existing export directory {}: {}",
                        export_path.display(),
                        err
                    );
                    return;
                }
            }
            false => {
                println!("Export cancelled.");
                return;
            }
        }
    }

    let mut all_files = beatmapset_folder.assets.clone();

    for osu_file in osu_files {
        let file_name = format!(
            "{} ({}).osu",
            &osu_file.metadata.version, osu_file.metadata.beatmap_id,
        );
        let file_name = sanitize_name(&file_name);
        let osu_file_content = osu_file.to_osu_text();
        all_files.insert(file_name, osu_file_content.into_bytes());
    }

    for (asset_name, asset_bytes) in all_files.clone_map() {
        let asset_path = export_path.join(&asset_name);
        if let Err(err) = write_bytes_to_file(&asset_path, &asset_bytes) {
            println!(
                "Failed to write asset file {}: {}",
                asset_path.display(),
                err
            );
            return;
        }
    }

    match create_zip(all_files.clone_map()) {
        Some(zip_bytes) => {
            let zip_path = format!("saves/{}/exports/{}.osz", map_name, map_name);
            let zip_path = Path::new(&zip_path);
            if let Err(err) = write_bytes_to_file(&zip_path, &zip_bytes) {
                println!("Failed to write zip file {}: {}", zip_path.display(), err);
                return;
            }
            println!("Exported map to {}", zip_path.display());
        }
        None => {
            println!("Failed to create zip file for {}", map_name);
        }
    };
}
