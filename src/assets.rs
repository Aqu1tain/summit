#![allow(dead_code, unused_imports, unused_variables)]

use std::env;
use std::path::{Path, PathBuf};

pub struct CelesteAssets {
    pub celeste_dir: Option<PathBuf>,
}

impl CelesteAssets {
    pub fn detect_celeste_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir()?;
            let path = home.join("Library").join("Application Support").join("Steam").join("steamapps").join("common").join("Celeste");
            if path.exists() {
                return Some(path);
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = env::var("APPDATA") {
                let path = PathBuf::from(appdata).join("Celeste");
                if path.exists() {
                    return Some(path);
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            let home = dirs::home_dir()?;
            let path = home.join(".local").join("share").join("Celeste");
            if path.exists() {
                return Some(path);
            }
        }
        None
    }
    pub fn new() -> Self {
        let detected = Self::detect_celeste_dir();
        Self {
            celeste_dir: detected,
        }
    }
    pub fn set_celeste_dir(&mut self, path: &Path) -> bool {
        self.celeste_dir = Some(path.to_path_buf());
        true
    }
    pub fn clear_celeste_dir(&mut self) {
        self.celeste_dir = None;
    }
}