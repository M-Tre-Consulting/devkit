// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 M-Tre Consulting

//! Local flat-file storage for recently opened and favorited tools.
//!
//! Stores data in the app's internal data directory without third-party dependencies,
//! ensuring persistence across app restarts on Android and desktop.

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

fn high_refresh_rate_file() -> PathBuf {
    get_storage_dir().join("high_refresh_rate.txt")
}

fn dark_theme_file() -> PathBuf {
    get_storage_dir().join("dark_theme.txt")
}

fn dynamic_color_file() -> PathBuf {
    get_storage_dir().join("dynamic_color.txt")
}

fn graphics_backend_file() -> PathBuf {
    get_storage_dir().join("graphics_backend.txt")
}

fn default_hash_algorithm_file() -> PathBuf {
    get_storage_dir().join("default_hash_algorithm.txt")
}

fn default_uuid_version_file() -> PathBuf {
    get_storage_dir().join("default_uuid_version.txt")
}

fn default_timestamp_format_file() -> PathBuf {
    get_storage_dir().join("default_timestamp_format.txt")
}

fn welcome_file() -> PathBuf {
    get_storage_dir().join("welcome_seen.txt")
}

/// Load the first-launch welcome completion flag (defaults to false).
pub fn load_has_seen_welcome() -> bool {
    let path = welcome_file();
    if let Ok(content) = fs::read_to_string(path) {
        content.trim().eq_ignore_ascii_case("true") || content.trim() == "1"
    } else {
        false
    }
}

/// Save the first-launch welcome completion flag.
pub fn save_has_seen_welcome(seen: bool) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(welcome_file(), if seen { "true" } else { "false" });
}

/// Reset the welcome flag so the welcome screen shows on next launch.
pub fn reset_welcome() {
    save_has_seen_welcome(false);
}

/// Highest valid tool ID registered in the application.
pub const MAX_TOOL_ID: i32 = crate::catalog::MAX_TOOL_ID;

/// Load list of recent tool IDs (up to 5, most recent first).
pub fn load_recents() -> Vec<i32> {
    let path = recents_file();
    if let Ok(content) = fs::read_to_string(path) {
        content
            .lines()
            .flat_map(|line| line.split(','))
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .filter(|&id| (1..=MAX_TOOL_ID).contains(&id))
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
    write_atomic(&recents_file(), &content);
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

/// Load favorite tool IDs in the order they were pinned (deduplicated).
pub fn load_favorites() -> Vec<i32> {
    let path = favorites_file();
    let mut out: Vec<i32> = Vec::new();
    if let Ok(content) = fs::read_to_string(path) {
        for id in content
            .lines()
            .flat_map(|line| line.split(','))
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .filter(|&id| (1..=MAX_TOOL_ID).contains(&id))
        {
            if !out.contains(&id) {
                out.push(id);
            }
        }
    }
    out
}

/// Save favorite tool IDs (order preserved).
pub fn save_favorites(favorites: &[i32]) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let content = favorites.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
    write_atomic(&favorites_file(), &content);
}

/// Toggle favorite status of a tool ID. Returns (is_favorite_now, all_favorites).
pub fn toggle_favorite(tool_id: i32) -> (bool, Vec<i32>) {
    let mut favorites = load_favorites();
    let is_fav = if let Some(pos) = favorites.iter().position(|&id| id == tool_id) {
        favorites.remove(pos);
        false
    } else {
        if (1..=MAX_TOOL_ID).contains(&tool_id) {
            favorites.push(tool_id);
        }
        true
    };
    save_favorites(&favorites);
    (is_fav, favorites)
}

/// Write via a temp file + rename so a crash mid-write never leaves a truncated file.
fn write_atomic(path: &std::path::Path, content: &str) {
    let tmp = path.with_extension("tmp");
    if fs::write(&tmp, content).is_ok() && fs::rename(&tmp, path).is_ok() {
        return;
    }
    let _ = fs::write(path, content);
}

/// Clear all saved recent tools.
pub fn clear_recents() {
    let _ = fs::remove_file(recents_file());
}

/// Clear all saved favorite tools.
pub fn clear_favorites() {
    let _ = fs::remove_file(favorites_file());
}

/// Load the persisted "force peak refresh rate" preference.
///
/// Defaults to true: without an explicit vote, LTPO panels (Pixel 9/10 on Tensor G4/G5)
/// often keep a NativeActivity at 60 Hz during animations, which reads as stutter.
pub fn load_high_refresh_rate() -> bool {
    let path = high_refresh_rate_file();
    if let Ok(content) = fs::read_to_string(path) {
        content.trim().eq_ignore_ascii_case("true") || content.trim() == "1"
    } else {
        true
    }
}

/// Save the high refresh rate preference.
pub fn save_high_refresh_rate(enabled: bool) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(high_refresh_rate_file(), if enabled { "true" } else { "false" });
}

/// Load the persisted dark theme preference (defaults to true).
pub fn load_dark_theme() -> bool {
    let path = dark_theme_file();
    if let Ok(content) = fs::read_to_string(path) {
        content.trim().eq_ignore_ascii_case("true") || content.trim() == "1"
    } else {
        true
    }
}

/// Save the dark theme preference.
pub fn save_dark_theme(enabled: bool) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(dark_theme_file(), if enabled { "true" } else { "false" });
}

/// Load the persisted dynamic color (Monet) preference (defaults to false).
pub fn load_dynamic_color() -> bool {
    let path = dynamic_color_file();
    if let Ok(content) = fs::read_to_string(path) {
        content.trim().eq_ignore_ascii_case("true") || content.trim() == "1"
    } else {
        false
    }
}

/// Save the dynamic color (Monet) preference.
pub fn save_dynamic_color(enabled: bool) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(dynamic_color_file(), if enabled { "true" } else { "false" });
}

/// Load the rendering engine preference: 0 = Auto, 1 = Vulkan, 2 = OpenGL ES (defaults to Auto).
pub fn load_graphics_backend() -> i32 {
    if let Ok(content) = fs::read_to_string(graphics_backend_file()) {
        if let Ok(val) = content.trim().parse::<i32>() {
            if (0..=2).contains(&val) {
                return val;
            }
        }
    }
    0
}

/// Save the rendering engine preference (applied on next launch).
pub fn save_graphics_backend(value: i32) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(graphics_backend_file(), value.clamp(0, 2).to_string());
}

/// Load the persisted default hash algorithm index (defaults to 2 for SHA-256).
pub fn load_default_hash_algorithm() -> i32 {
    let path = default_hash_algorithm_file();
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(val) = content.trim().parse::<i32>() {
            if (0..=5).contains(&val) {
                return val;
            }
        }
    }
    2
}

/// Save the default hash algorithm index.
pub fn save_default_hash_algorithm(algo: i32) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(default_hash_algorithm_file(), algo.to_string());
}

/// Load the persisted default UUID version index (defaults to 1 for v4).
pub fn load_default_uuid_version() -> i32 {
    let path = default_uuid_version_file();
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(val) = content.trim().parse::<i32>() {
            if (0..=2).contains(&val) {
                return val;
            }
        }
    }
    1
}

/// Save the default UUID version index.
pub fn save_default_uuid_version(version: i32) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(default_uuid_version_file(), version.to_string());
}

/// Load the persisted default timestamp format index (defaults to 0 for ISO 8601).
pub fn load_default_timestamp_format() -> i32 {
    let path = default_timestamp_format_file();
    if let Ok(content) = fs::read_to_string(path) {
        if let Ok(val) = content.trim().parse::<i32>() {
            if (0..=4).contains(&val) {
                return val;
            }
        }
    }
    0
}

/// Save the default timestamp format index.
pub fn save_default_timestamp_format(format: i32) {
    let dir = get_storage_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(default_timestamp_format_file(), format.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_high_refresh_rate_persistence() {
        let _guard = TEST_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("devkit_test_{}", std::process::id()));
        init_storage_dir(temp_dir.clone());

        // Default should be true (peak refresh) when the file does not exist
        let _ = fs::remove_file(high_refresh_rate_file());
        assert!(load_high_refresh_rate());

        // Save true and verify
        save_high_refresh_rate(true);
        assert!(load_high_refresh_rate());

        // Save false and verify
        save_high_refresh_rate(false);
        assert!(!load_high_refresh_rate());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_dark_theme_persistence() {
        let _guard = TEST_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("devkit_theme_test_{}", std::process::id()));
        init_storage_dir(temp_dir.clone());

        // Default should be true (dark) when file does not exist
        let _ = fs::remove_file(dark_theme_file());
        assert!(load_dark_theme());

        // Save false (light) and verify
        save_dark_theme(false);
        assert!(!load_dark_theme());

        // Save true and verify
        save_dark_theme(true);
        assert!(load_dark_theme());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_dynamic_color_persistence() {
        let _guard = TEST_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("devkit_monet_test_{}", std::process::id()));
        init_storage_dir(temp_dir.clone());

        // Default should be false when file does not exist
        let _ = fs::remove_file(dynamic_color_file());
        assert!(!load_dynamic_color());

        // Save true and verify
        save_dynamic_color(true);
        assert!(load_dynamic_color());

        // Save false and verify
        save_dynamic_color(false);
        assert!(!load_dynamic_color());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_welcome_seen_persistence() {
        let _guard = TEST_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("devkit_welcome_test_{}", std::process::id()));
        init_storage_dir(temp_dir.clone());

        // Default should be false when file does not exist
        let _ = fs::remove_file(welcome_file());
        assert!(!load_has_seen_welcome());

        // Save true and verify
        save_has_seen_welcome(true);
        assert!(load_has_seen_welcome());

        // Reset and verify
        reset_welcome();
        assert!(!load_has_seen_welcome());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_default_hash_algorithm_persistence() {
        let _guard = TEST_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("devkit_hash_algo_test_{}", std::process::id()));
        init_storage_dir(temp_dir.clone());

        let _ = fs::remove_file(default_hash_algorithm_file());
        assert_eq!(load_default_hash_algorithm(), 2); // Default is SHA-256 (2)

        save_default_hash_algorithm(0); // MD5
        assert_eq!(load_default_hash_algorithm(), 0);

        save_default_hash_algorithm(5); // BLAKE3
        assert_eq!(load_default_hash_algorithm(), 5);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_default_uuid_version_persistence() {
        let _guard = TEST_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("devkit_uuid_ver_test_{}", std::process::id()));
        init_storage_dir(temp_dir.clone());

        let _ = fs::remove_file(default_uuid_version_file());
        assert_eq!(load_default_uuid_version(), 1); // Default is v4 (1)

        save_default_uuid_version(2); // v7
        assert_eq!(load_default_uuid_version(), 2);

        save_default_uuid_version(0); // v1
        assert_eq!(load_default_uuid_version(), 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_default_timestamp_format_persistence() {
        let _guard = TEST_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("devkit_ts_fmt_test_{}", std::process::id()));
        init_storage_dir(temp_dir.clone());

        let _ = fs::remove_file(default_timestamp_format_file());
        assert_eq!(load_default_timestamp_format(), 0); // Default is ISO 8601 (0)

        save_default_timestamp_format(1); // RFC 2822
        assert_eq!(load_default_timestamp_format(), 1);

        save_default_timestamp_format(3); // Seconds
        assert_eq!(load_default_timestamp_format(), 3);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_graphics_backend_persistence() {
        let _guard = TEST_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("devkit_gfx_test_{}", std::process::id()));
        init_storage_dir(temp_dir.clone());

        let _ = fs::remove_file(graphics_backend_file());
        assert_eq!(load_graphics_backend(), 0); // Auto

        save_graphics_backend(2);
        assert_eq!(load_graphics_backend(), 2);
        save_graphics_backend(9);
        assert_eq!(load_graphics_backend(), 2); // clamped

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_favorites_keep_pin_order() {
        let _guard = TEST_LOCK.lock().unwrap();
        let temp_dir = std::env::temp_dir().join(format!("devkit_fav_test_{}", std::process::id()));
        init_storage_dir(temp_dir.clone());
        let _ = fs::create_dir_all(&temp_dir);
        clear_favorites();

        assert_eq!(toggle_favorite(5), (true, vec![5]));
        assert_eq!(toggle_favorite(2), (true, vec![5, 2]));
        assert_eq!(toggle_favorite(9), (true, vec![5, 2, 9]));
        assert_eq!(toggle_favorite(2), (false, vec![5, 9]));
        assert_eq!(load_favorites(), vec![5, 9]);
        // Out-of-range IDs are never persisted.
        toggle_favorite(999);
        assert_eq!(load_favorites(), vec![5, 9]);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
