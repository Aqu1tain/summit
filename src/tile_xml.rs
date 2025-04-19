use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use once_cell::sync::OnceCell;
use quick_xml::events::Event;
use quick_xml::Reader;
use crate::app::CelesteMapEditor;

/// Loads a mapping from tile id (char) to tileset path from a ForegroundTiles.xml or BackgroundTiles.xml file.
pub fn load_tileset_id_path_map(xml_path: &str) -> HashMap<char, String> {
    let mut copy_map: HashMap<char, char> = HashMap::new();
    let mut path_map: HashMap<char, String> = HashMap::new();
    let file = match File::open(xml_path) {
        Ok(f) => f,
        Err(_) => return path_map,
    };
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if e.name().as_ref() == b"Tileset" => {
                let mut id: Option<char> = None;
                let mut path: Option<String> = None;
                let mut copy: Option<char> = None;
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"id" => {
                            if let Ok(val) = attr.unescape_value() {
                                let ch = val.chars().next();
                                id = ch;
                                eprintln!("[TILE XML DEBUG] Found Tileset id attribute: '{}' (full: '{}')", ch.unwrap_or('?'), val);
                            }
                        }
                        b"path" => {
                            if let Ok(val) = attr.unescape_value() {
                                path = Some(val.to_string());
                            }
                        }
                        b"copy" => {
                            if let Ok(val) = attr.unescape_value() {
                                let ch = val.chars().next();
                                copy = ch;
                            }
                        }
                        _ => {}
                    }
                }
                if let (Some(id), Some(path)) = (id, path.clone()) {
                    path_map.insert(id, path);
                }
                if let (Some(id), Some(copy_id)) = (id, copy) {
                    copy_map.insert(id, copy_id);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    // Resolve copy chains
    for (id, copy_id) in &copy_map {
        if !path_map.contains_key(id) {
            if let Some(base_path) = path_map.get(copy_id) {
                path_map.insert(*id, base_path.clone());
            }
        }
    }
    path_map
}

/// Helper to get the tileset path for a tile id from a preloaded map.
pub fn get_tileset_path_for_id(map: &HashMap<char, String>, id: char) -> Option<&str> {
    map.get(&id).map(|s| s.as_str())
}

/// For a given tileset id, return the default tile coordinate (0,0) for the top-left 8x8 tile.
/// If ForegroundTiles.xml does not specify, always return (0,0).
pub fn get_first_tile_coords_for_id_or_default(_xml_path: &str, _id: char) -> (u32, u32) {
    // Always return top-left tile
    (0, 0)
}

pub static TILESET_ID_PATH_MAP: OnceCell<HashMap<char, String>> = OnceCell::new();

/// Ensures the tileset id/path map is loaded, using the Celeste install path.
pub fn ensure_tileset_id_path_map_loaded_from_celeste(editor: &CelesteMapEditor) {
    if TILESET_ID_PATH_MAP.get().is_some() { return; }
    if let Some(ref celeste_dir) = editor.celeste_assets.celeste_dir {
        let mut xml_path = celeste_dir.clone();
        // On Mac, assets are inside Celeste.app/Contents/Resources/Content/Graphics/ForegroundTiles.xml
        #[cfg(target_os = "macos")]
        {
            if !xml_path.ends_with("Celeste.app") {
                xml_path = xml_path.join("Celeste.app");
            }
            xml_path = xml_path.join("Contents/Resources/Content/Graphics/ForegroundTiles.xml");
        }
        #[cfg(not(target_os = "macos"))]
        {
            xml_path = xml_path.join("Content/Graphics/ForegroundTiles.xml");
        }
        eprintln!("[TILE XML] Loading ForegroundTiles.xml from: {}", xml_path.display());
        if xml_path.exists() {
            let map = load_tileset_id_path_map(xml_path.to_str().unwrap());
            eprintln!("[TILE XML] Loaded {} entries:", map.len());
            for (id, path) in &map {
                eprintln!("[TILE XML] id='{}' path='{}'", id, path);
            }
            let _ = TILESET_ID_PATH_MAP.set(map);
        } else {
            eprintln!("[TILE XML] ForegroundTiles.xml not found at {}", xml_path.display());
        }
    } else {
        eprintln!("[TILE XML] celeste_dir is None!");
    }
}
