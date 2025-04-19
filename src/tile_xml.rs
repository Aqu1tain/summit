use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
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

pub static TILESET_ID_PATH_MAP_FG: OnceCell<HashMap<char, String>> = OnceCell::new();
pub static TILESET_ID_PATH_MAP_BG: OnceCell<HashMap<char, String>> = OnceCell::new();

/// Ensures the tileset id/path maps are loaded for both foreground and background, using the Celeste install path.
pub fn ensure_tileset_id_path_map_loaded_from_celeste(editor: &CelesteMapEditor) {
    // Load foreground tileset map
    if TILESET_ID_PATH_MAP_FG.get().is_none() {
        if let Some(ref celeste_dir) = editor.celeste_assets.celeste_dir {
            let mut xml_path = celeste_dir.clone();
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
                eprintln!("[TILE XML] Loaded {} foreground entries:", map.len());
                for (id, path) in &map {
                    eprintln!("[TILE XML] id='{}' path='{}'", id, path);
                }
                let _ = TILESET_ID_PATH_MAP_FG.set(map);
            } else {
                eprintln!("[TILE XML] ForegroundTiles.xml not found at {}", xml_path.display());
            }
        } else {
            eprintln!("[TILE XML] celeste_dir is None!");
        }
    }

    // Load background tileset map
    if TILESET_ID_PATH_MAP_BG.get().is_none() {
        if let Some(ref celeste_dir) = editor.celeste_assets.celeste_dir {
            let mut xml_path = celeste_dir.clone();
            #[cfg(target_os = "macos")]
            {
                if !xml_path.ends_with("Celeste.app") {
                    xml_path = xml_path.join("Celeste.app");
                }
                xml_path = xml_path.join("Contents/Resources/Content/Graphics/BackgroundTiles.xml");
            }
            #[cfg(not(target_os = "macos"))]
            {
                xml_path = xml_path.join("Content/Graphics/BackgroundTiles.xml");
            }
            eprintln!("[TILE XML] Loading BackgroundTiles.xml from: {}", xml_path.display());
            if xml_path.exists() {
                let map = load_tileset_id_path_map(xml_path.to_str().unwrap());
                eprintln!("[TILE XML] Loaded {} background entries:", map.len());
                for (id, path) in &map {
                    eprintln!("[TILE XML] id='{}' path='{}'", id, path);
                }
                let _ = TILESET_ID_PATH_MAP_BG.set(map);
            } else {
                eprintln!("[TILE XML] BackgroundTiles.xml not found at {}", xml_path.display());
            }
        } else {
            eprintln!("[TILE XML] celeste_dir is None!");
        }
    }
}

// --- AUTOTILING DATA STRUCTURES ---
static TILESET_RULES: OnceCell<HashMap<char, Tileset>> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct Tileset {
    #[allow(dead_code)]
    pub id: char,
    #[allow(dead_code)]
    pub path: String,
    pub ignores: Option<String>,
    pub rules: Vec<SetRule>,
}

#[derive(Debug, Clone)]
pub struct SetRule {
    pub mask: String,
    pub tiles: Vec<(u32, u32)>,
}

/// Loads and caches all tileset definitions from ForegroundTiles.xml or BackgroundTiles.xml, including inherited rules via copy="z".
pub fn get_tilesets_with_rules(xml_path: &str) -> &HashMap<char, Tileset> {
    TILESET_RULES.get_or_init(|| load_tilesets_with_rules(xml_path))
}

/// Loads all tileset definitions from ForegroundTiles.xml or BackgroundTiles.xml, including inherited rules via copy="z".
pub fn load_tilesets_with_rules(xml_path: &str) -> HashMap<char, Tileset> {
    let mut tilesets: HashMap<char, Tileset> = HashMap::new();
    let mut rules_by_id: HashMap<char, Vec<SetRule>> = HashMap::new();
    let mut ignores_by_id: HashMap<char, Option<String>> = HashMap::new();
    let mut path_by_id: HashMap<char, String> = HashMap::new();
    let mut copy_map: HashMap<char, char> = HashMap::new();

    let file = match File::open(xml_path) {
        Ok(f) => f,
        Err(_) => return tilesets,
    };
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut current_id: Option<char> = None;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if e.name().as_ref() == b"Tileset" => {
                let mut id: Option<char> = None;
                let mut path: Option<String> = None;
                let mut copy: Option<char> = None;
                let mut ignores: Option<String> = None;
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"id" => {
                            if let Ok(val) = attr.unescape_value() {
                                id = val.chars().next();
                            }
                        }
                        b"path" => {
                            if let Ok(val) = attr.unescape_value() {
                                path = Some(val.to_string());
                            }
                        }
                        b"copy" => {
                            if let Ok(val) = attr.unescape_value() {
                                copy = val.chars().next();
                            }
                        }
                        b"ignores" => {
                            if let Ok(val) = attr.unescape_value() {
                                ignores = Some(val.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(id_val) = id {
                    current_id = Some(id_val);
                    if let Some(path) = path.clone() {
                        path_by_id.insert(id_val, path);
                    }
                    if let Some(copy_id) = copy {
                        copy_map.insert(id_val, copy_id);
                    }
                    ignores_by_id.insert(id_val, ignores.clone());
                    rules_by_id.entry(id_val).or_default();
                }
            }
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if e.name().as_ref() == b"set" => {
                if let Some(_id) = current_id {
                    let mut mask: Option<String> = None;
                    let mut tiles: Vec<(u32, u32)> = vec![];
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"mask" => {
                                if let Ok(val) = attr.unescape_value() {
                                    mask = Some(val.to_string());
                                }
                            }
                            b"tiles" => {
                                if let Ok(val) = attr.unescape_value() {
                                    for pair in val.split(';') {
                                        let coords: Vec<&str> = pair.split(',').collect();
                                        if coords.len() == 2 {
                                            if let (Ok(x), Ok(y)) = (coords[0].trim().parse(), coords[1].trim().parse()) {
                                                tiles.push((x, y));
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(mask) = mask {
                        rules_by_id.entry(current_id.unwrap()).or_default().push(SetRule { mask, tiles });
                    }
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"Tileset" => {
                current_id = None;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    // Inherit rules from copy=... (e.g. copy="z")
    for (id, copy_id) in &copy_map {
        let base_rules = rules_by_id.get(copy_id).cloned().unwrap_or_default();
        rules_by_id.entry(*id).or_default().extend(base_rules);
    }
    // Build Tileset structs
    for (id, path) in path_by_id {
        let rules = rules_by_id.remove(&id).unwrap_or_default();
        let ignores = ignores_by_id.remove(&id).flatten();
        tilesets.insert(id, Tileset { id, path, ignores, rules });
    }
    tilesets
}

/// Given a tile id, returns the Tileset struct (with inherited rules) from a preloaded map.
pub fn get_tileset_for_id(tilesets: &HashMap<char, Tileset>, id: char) -> Option<&Tileset> {
    tilesets.get(&id)
}

/// Given a 3x3 grid of chars, and a mask, returns true if the mask matches the neighborhood.
pub fn mask_matches(neighborhood: &[[char; 3]; 3], mask: &str, is_solid: &dyn Fn(char) -> bool, ignores: Option<&str>) -> bool {
    if mask == "center" {
        // All tiles (including center) must be solid (including OOB)
        for row in 0..3 {
            for col in 0..3 {
                let tile = neighborhood[row][col];
                if tile == '\0' {
                    continue; // OOB is solid
                }
                if !is_solid(tile) {
                    return false;
                }
            }
        }
        return true;
    }
    if mask == "padding" {
        // Center solid, all 8 neighbors solid (including OOB)
        let center = neighborhood[1][1];
        if center == '\0' {
            // OOB center should not happen, but treat as solid
        } else if !is_solid(center) {
            return false;
        }
        for row in 0..3 {
            for col in 0..3 {
                if row == 1 && col == 1 { continue; }
                let tile = neighborhood[row][col];
                if tile == '\0' {
                    continue; // OOB is solid
                }
                if !is_solid(tile) {
                    return false;
                }
            }
        }
        // 2-away orthogonal check is enforced in autotile_tile_coord.
        return true;
    }
    // Default: explicit mask parsing
    let mask_rows: Vec<&str> = mask.split('-').collect();
    if mask_rows.len() != 3 { return false; }
    for (y, mask_row) in mask_rows.iter().enumerate() {
        let mask_chars: Vec<char> = mask_row.chars().collect();
        if mask_chars.len() != 3 { return false; }
        for (x, m) in mask_chars.iter().enumerate() {
            let tile = neighborhood[y][x];
            let oob = tile == '\0';
            match m {
                '0' => {
                    // Must be empty
                    if (is_solid(tile) && ignores.map_or(true, |ign| !ign.contains(tile))) || oob {
                        return false;
                    }
                }
                '1' => {
                    // Must be solid
                    if !(is_solid(tile) && ignores.map_or(true, |ign| !ign.contains(tile))) && !oob {
                        return false;
                    }
                }
                'x' | 'X' => {
                    // Wildcard, matches anything
                }
                _ => {}
            }
        }
    }
    true
}

/// Given the tile map and coordinates, extracts the 3x3 neighborhood for autotiling.
pub fn get_neighborhood(solids: &Vec<Vec<char>>, x: usize, y: usize) -> [[char; 3]; 3] {
    let mut n = [['\0'; 3]; 3];
    let h = solids.len() as isize;
    let w = if h > 0 { solids[0].len() as isize } else { 0 };
    for dy in -1..=1 {
        for dx in -1..=1 {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx >= 0 && ny >= 0 && ny < h {
                let nyu = ny as usize;
                if nyu < solids.len() {
                    let row = &solids[nyu];
                    if nx < row.len() as isize && nx >= 0 {
                        n[(dy + 1) as usize][(dx + 1) as usize] = row[nx as usize];
                        continue;
                    }
                }
            }
            n[(dy + 1) as usize][(dx + 1) as usize] = '\0';
        }
    }
    n
}

/// Helper for padding: check 2-away orthogonal neighbors for air
fn has_orthogonal_air(solids: &Vec<Vec<char>>, x: usize, y: usize, is_solid: &dyn Fn(char) -> bool) -> bool {
    let offsets = [(-2, 0), (2, 0), (0, -2), (0, 2)];
    let h = solids.len() as isize;
    let w = if h > 0 { solids[0].len() as isize } else { 0 };
    for (dx, dy) in offsets.iter() {
        let nx = x as isize + dx;
        let ny = y as isize + dy;
        // Out of bounds is counted as solid
        if nx < 0 || ny < 0 || ny >= h || nx >= w {
            continue;
        }
        if (ny as usize) >= solids.len() {
            continue;
        }
        let row = &solids[ny as usize];
        if (nx as usize) >= row.len() {
            continue;
        }
        if !is_solid(row[nx as usize]) {
            return true;
        }
    }
    false
}

/// Main autotiling entry: given tile id, solids, x, y, and tilesets, returns the tile coordinate to use.
pub fn autotile_tile_coord(tile_id: char, solids: &Vec<Vec<char>>, x: usize, y: usize, tilesets: &HashMap<char, Tileset>, is_solid: &dyn Fn(char) -> bool) -> Option<(u32, u32)> {
    let tileset = get_tileset_for_id(tilesets, tile_id)?;
    let n = get_neighborhood(solids, x, y);
    // 1. Explicit masks (not "padding" or "center") in order
    for rule in &tileset.rules {
        if rule.mask != "padding" && rule.mask != "center" {
            if mask_matches(&n, &rule.mask, is_solid, tileset.ignores.as_deref()) {
                if !rule.tiles.is_empty() {
                    let idx = ((x as u64 * 31 + y as u64 * 17) % rule.tiles.len() as u64) as usize;
                    return Some(rule.tiles[idx]);
                }
            }
        }
    }
    // 2. Fallback: "padding" (before center)
    let mut padding_rule: Option<&SetRule> = None;
    for rule in &tileset.rules {
        if rule.mask == "padding" {
            if mask_matches(&n, &rule.mask, is_solid, tileset.ignores.as_deref()) && has_orthogonal_air(solids, x, y, is_solid) {
                padding_rule = Some(rule);
                break;
            }
        }
    }
    if let Some(rule) = padding_rule {
        if !rule.tiles.is_empty() {
            let idx = ((x as u64 * 31 + y as u64 * 17) % rule.tiles.len() as u64) as usize;
            return Some(rule.tiles[idx]);
        }
    }
    // 3. Fallback: "center"
    let mut center_rule: Option<&SetRule> = None;
    for rule in &tileset.rules {
        if rule.mask == "center" {
            if mask_matches(&n, &rule.mask, is_solid, tileset.ignores.as_deref()) {
                center_rule = Some(rule);
                break;
            }
        }
    }
    if let Some(rule) = center_rule {
        if !rule.tiles.is_empty() {
            let idx = ((x as u64 * 31 + y as u64 * 17) % rule.tiles.len() as u64) as usize;
            return Some(rule.tiles[idx]);
        }
    }
    // 4. Fallback: top-left
    Some((0, 0))
}