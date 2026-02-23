use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::Path,
};

use crate::{
    config::Config,
    map_format::{beatmap::Beatmap, beatmapset::Beatmapset},
};

pub fn scan_folder(path: &Path, dir: Option<bool>, suffix: Option<&Vec<&str>>) -> Vec<String> {
    let mut entries = Vec::new();
    if !path.exists() {
        return entries;
    }
    for entry in fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name().to_string_lossy().to_string();
        match suffix {
            Some(suffixes) if !suffixes.iter().any(|s| file_name.ends_with(s)) => continue,
            _ => {}
        }
        match dir {
            Some(true) if !entry.path().is_dir() => continue,
            Some(false) if entry.path().is_dir() => continue,
            _ => {}
        }
        entries.push(file_name);
    }
    return entries;
}

pub fn sanitize_name(input: &str) -> String {
    // Windows disallows: < > : " / \ | ? * and ASCII control chars.
    // It also disallows trailing spaces/dots and a set of reserved device names.
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        let invalid = matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            || (ch as u32) < 0x20;
        if invalid {
            out.push('_');
        } else {
            out.push(ch);
        }
    }

    while out.ends_with([' ', '.']) {
        out.pop();
    }
    let out = out.trim().to_string();
    if out.is_empty() {
        return "unnamed".to_string();
    }
    return out;
}

pub fn extract_zip(bytes: Vec<u8>) -> Option<Vec<(String, Vec<u8>)>> {
    let reader = std::io::Cursor::new(bytes);
    let mut archive = match zip::ZipArchive::new(reader) {
        Ok(archive) => archive,
        Err(err) => {
            println!("Failed to open archive: {}", err);
            return None;
        }
    };

    let mut extracted: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(file) => file,
            Err(err) => {
                println!("Failed to read entry #{i} from archive: {err}");
                continue;
            }
        };

        if file.is_dir() {
            continue;
        }

        let name = file.name().to_string();
        let mut bytes = Vec::with_capacity(file.size() as usize);
        if let Err(err) = file.read_to_end(&mut bytes) {
            println!("Failed to extract {} from archive: {}", name, err);
            continue;
        }

        extracted.push((name, bytes));
    }
    return Some(extracted);
}

pub fn write_bytes_to_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    match path.parent() {
        Some(parent) => {
            let err = fs::create_dir_all(parent).err();
            if let Some(err) = err {
                return Err(err);
            }
        }
        None => {}
    }
    fs::create_dir_all(path.parent().unwrap())?;
    let mut file = fs::File::create(path)?;
    use std::io::Write;
    file.write_all(bytes)?;
    return Ok(());
}

fn scan_folder_recursive_files(root: &Path) -> Vec<String> {
    fn visit_dir(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_dir() {
                visit_dir(root, &path, out);
                continue;
            }

            let rel = match path.strip_prefix(root) {
                Ok(rel) => rel,
                Err(_) => continue,
            };

            let rel = rel.to_string_lossy().replace('\\', "/");
            out.push(rel);
        }
    }

    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    visit_dir(root, root, &mut out);
    out
}

pub struct BeatmapsetFolder {
    pub map_dir_name: String,
    pub beatmapset: Beatmapset,
    pub beatmaps: Vec<Beatmap>,
    pub assets: AssetsFolder,
}

#[derive(Clone)]
pub struct AssetsFolder {
    assets: HashMap<String, Vec<u8>>,
}

impl AssetsFolder {
    pub fn get(&self, name: &str) -> Option<&Vec<u8>> {
        fn normalize_name(mut name: &str) -> String {
            name = name.trim();
            if name.starts_with('"') && name.ends_with('"') && name.len() >= 2 {
                name = &name[1..name.len() - 1];
            }
            let name = name.trim();
            let name = name.replace('\\', "/");
            name.strip_prefix("./").unwrap_or(&name).to_string()
        }

        let name = normalize_name(name);
        if let Some(v) = self.assets.get(&name) {
            return Some(v);
        }

        // Fallbacks: case-insensitive match (Windows), and match by basename only.
        let target_lower = name.to_ascii_lowercase();
        let target_file = std::path::Path::new(&name)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(name.as_str());
        let target_file_lower = target_file.to_ascii_lowercase();

        for (k, v) in &self.assets {
            if k.eq_ignore_ascii_case(&name) {
                return Some(v);
            }
            if k.to_ascii_lowercase() == target_lower {
                return Some(v);
            }

            if let Some(k_file) = std::path::Path::new(k).file_name().and_then(|s| s.to_str()) {
                if k_file.eq_ignore_ascii_case(target_file) {
                    return Some(v);
                }
                if k_file.to_ascii_lowercase() == target_file_lower {
                    return Some(v);
                }
            }
        }

        None
    }
    pub fn insert(&mut self, name: String, data: Vec<u8>) {
        self.assets.insert(name, data);
    }

    pub fn clone_map(&self) -> HashMap<String, Vec<u8>> {
        self.assets.clone()
    }
}

pub fn open_beatmapset_folder(map_dir_name: &String) -> Option<BeatmapsetFolder> {
    let beatmapset_json =
        match fs::read_to_string(format!("saves/{}/beatmapset.json", map_dir_name)) {
            Ok(content) => content,
            Err(err) => {
                println!("Failed to read beatmapset.json: {}", err);
                return None;
            }
        };
    let beatmapset = match serde_json::from_str::<Beatmapset>(&beatmapset_json) {
        Ok(b) => b,
        Err(err) => {
            println!("Failed to read beatmapset.json: {}", err);
            return None;
        }
    };
    let diffs_folders = format!("saves/{}/diffs", map_dir_name);
    let diffs_folders = Path::new(diffs_folders.as_str());
    let diffs_folders = scan_folder(diffs_folders, Some(true), None);
    if diffs_folders.is_empty() {
        println!("No diffs found in diffs/");
        return None;
    }
    let mut beatmaps = Vec::new();
    for diff in diffs_folders {
        let diff =
            Path::new(&format!("saves/{}/diffs/{}", map_dir_name, diff)).join("beatmap.json");
        let beatmap_json = match fs::read_to_string(diff) {
            Ok(content) => content,
            Err(err) => {
                println!("Failed to read beatmap JSON: {}", err);
                return None;
            }
        };
        let beatmap =
            match serde_json::from_str::<crate::map_format::beatmap::Beatmap>(&beatmap_json) {
                Ok(b) => b,
                Err(err) => {
                    println!("Failed to parse beatmap JSON: {}", err);
                    return None;
                }
            };
        beatmaps.push(beatmap);
    }

    let assets_folder = format!("saves/{}/assets", map_dir_name);
    let assets_folder = Path::new(assets_folder.as_str());
    let assets_folder = scan_folder_recursive_files(assets_folder);
    let mut assets: HashMap<String, Vec<u8>> = HashMap::new();
    for asset in assets_folder {
        let asset_path = Path::new("saves")
            .join(map_dir_name)
            .join("assets")
            .join(&asset);
        let asset_bytes = match fs::read(&asset_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                println!("Failed to read asset {}: {}", asset, err);
                continue;
            }
        };
        assets.insert(asset, asset_bytes);
    }

    return Some(BeatmapsetFolder {
        map_dir_name: map_dir_name.clone(),
        beatmapset,
        beatmaps,
        assets: AssetsFolder { assets },
    });
}

pub fn create_zip(files: HashMap<String, Vec<u8>>) -> Option<Vec<u8>> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut buffer);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for (file_name, file_bytes) in files {
        if let Err(err) = zip.start_file(file_name, options) {
            println!("Failed to add file to zip: {}", err);
            return None;
        }
        if let Err(err) = zip.write_all(&file_bytes) {
            println!("Failed to write file to zip: {}", err);
            return None;
        }
    }

    if let Err(err) = zip.finish() {
        println!("Failed to finalize zip archive: {}", err);
        return None;
    }

    return Some(buffer.into_inner());
}

pub fn get_config() -> Option<Config> {
    let config_path = Path::new("config.json");

    let config_json = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(err) => {
            println!("Failed to read config.json: {}", err);
            return None;
        }
    };
    return match serde_json::from_str::<Config>(&config_json) {
        Ok(config) => Some(config),
        Err(err) => {
            println!("Failed to parse config.json: {}", err);
            None
        }
    };
}
