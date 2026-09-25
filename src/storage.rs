// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 MTRE Consulting

//! Local flat-file storage for recently opened and favorited tools.
//!
//! Stores data in the app's internal data directory without third-party dependencies,
//! ensuring persistence across app restarts on Android and desktop.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static STORAGE_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Initialize the storage directory (typically from Android internal_data_path).
pub fn init_storage_dir(dir: PathBuf) {
    if let Ok(mut lock) = STORAGE_DIR.lock() {
        *lock = Some(dir);
    }
}

fn get_storage_dir() -> PathBuf {
    if let Ok(lock) = STORAGE_DIR.lock() {
        if let Some(ref path) = *lock {
            return path.clone();
        }
    }

    #[cfg(target_os = "android")]
    {
        PathBuf::from("/data/data/it.mtreconsulting.devkit/files")
    }
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home).join(".local/share/devkit");
            let _ = fs::create_dir_all(&p);
            p
        } else {
            PathBuf::from(".")
        }
    }
}

fn recents_file() -> PathBuf {
    get_storage_dir().join("recents.txt")
}

fn favorites_file() -> PathBuf {
    get_storage_dir().join("favorites.txt")
}

/// Load list of recent tool IDs (up to 5, most recent first).
pub fn load_recents() -> Vec<i32> {
    let path = recents_file();
    if let Ok(content) = fs::read_to_string(path) {
        content
            .lines()
            .flat_map(|line| line.split(','))
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .filter(|&id| (1..=6).contains(&id))
            .take(5)
            .collect()
    } else {
        Vec::new()
    }
}

/// Save list of recent tool IDs.
pub fn save_recents(recents: &[i32]) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let str_vals: Vec<String> = recents.iter().map(|id| id.to_string()).collect();
    let content = str_vals.join(",");
    let _ = fs::write(recents_file(), content);
}

/// Record a tool being opened. Pushes to front, deduplicates, caps at 5.
pub fn record_tool_opened(tool_id: i32) -> Vec<i32> {
    let mut recents = load_recents();
    recents.retain(|&id| id != tool_id);
    recents.insert(0, tool_id);
    recents.truncate(5);
    save_recents(&recents);
    recents
}

/// Load set of favorite tool IDs.
pub fn load_favorites() -> HashSet<i32> {
    let path = favorites_file();
    if let Ok(content) = fs::read_to_string(path) {
        content
            .lines()
            .flat_map(|line| line.split(','))
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .filter(|&id| (1..=6).contains(&id))
            .collect()
    } else {
        HashSet::new()
    }
}

/// Save set of favorite tool IDs.
pub fn save_favorites(favorites: &HashSet<i32>) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let str_vals: Vec<String> = favorites.iter().map(|id| id.to_string()).collect();
    let content = str_vals.join(",");
    let _ = fs::write(favorites_file(), content);
}

/// Toggle favorite status of a tool ID. Returns (is_favorite_now, all_favorites).
pub fn toggle_favorite(tool_id: i32) -> (bool, HashSet<i32>) {
    let mut favorites = load_favorites();
    let is_fav = if favorites.contains(&tool_id) {
        favorites.remove(&tool_id);
        false
    } else {
        favorites.insert(tool_id);
        true
    };
    save_favorites(&favorites);
    (is_fav, favorites)
}

/// Clear all saved recent tools.
pub fn clear_recents() {
    let _ = fs::remove_file(recents_file());
}

/// Clear all saved favorite tools.
pub fn clear_favorites() {
    let _ = fs::remove_file(favorites_file());
}
