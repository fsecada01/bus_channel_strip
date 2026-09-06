//! User preset save/load against `~/Documents/Bus Channel Strip/Presets/`
//! (issue #21). The directory-parameterized `*_in_dir` functions are the
//! actual implementation and are exercised directly by tests against a
//! scratch directory; the public `presets_dir()`-based functions are thin
//! wrappers so production code never has to think about the two paths.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::schema::{Preset, PRESET_EXTENSION};

/// `~/Documents/Bus Channel Strip/Presets/`. Not guaranteed to exist yet —
/// [`save_user_preset`] creates it on first save.
///
/// Only called from `editor.rs` (gui-gated) in production; tests exercise
/// the directory-parameterized `*_in_dir` functions directly instead so
/// they stay hermetic (no real Documents-folder access).
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn presets_dir() -> io::Result<PathBuf> {
    let documents = dirs::document_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "could not resolve the platform Documents directory",
        )
    })?;
    Ok(documents.join("Bus Channel Strip").join("Presets"))
}

/// Replaces characters that are illegal (or awkward) in file names on
/// Windows/macOS/Linux with `_`, so any preset name can round-trip through a
/// file name without silently colliding with an unrelated preset or failing
/// to save outright.
fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        "Untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

fn save_preset_in_dir(preset: &Preset, dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let filename = format!("{}.{}", sanitize_filename(&preset.name), PRESET_EXTENSION);
    let path = dir.join(filename);
    let json = serde_json::to_string_pretty(preset)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, json)?;
    Ok(path)
}

pub fn load_preset_file(path: &Path) -> io::Result<Preset> {
    let json = fs::read_to_string(path)?;
    serde_json::from_str(&json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn list_presets_in_dir(dir: &Path) -> io::Result<Vec<Preset>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some(PRESET_EXTENSION))
        .collect();
    paths.sort();

    paths
        .into_iter()
        .map(|path| load_preset_file(&path))
        .collect()
}

/// Saves `preset` as a `.bcs` file under `presets_dir()`, creating the
/// directory if this is the first user preset saved. Returns the path
/// written to.
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn save_user_preset(preset: &Preset) -> io::Result<PathBuf> {
    save_preset_in_dir(preset, &presets_dir()?)
}

/// Every saved user preset under `presets_dir()`, sorted alphabetically by
/// file name. Returns an empty list (not an error) if the directory doesn't
/// exist yet — e.g. before the user has saved anything.
#[cfg_attr(not(feature = "gui"), allow(dead_code))]
pub fn list_user_presets() -> io::Result<Vec<Preset>> {
    list_presets_in_dir(&presets_dir()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU32, Ordering};

    use nice_plug::plugin::ParamValue;

    static SCRATCH_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// A fresh, empty scratch directory under the OS temp dir, unique per
    /// call so concurrent tests never collide.
    fn scratch_dir() -> PathBuf {
        let n = SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("bcs_preset_io_test_{n}"));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn sample_preset(name: &str) -> Preset {
        let mut params = BTreeMap::new();
        params.insert("gain".to_string(), ParamValue::F32(0.5));
        Preset::new(name, "Test", params)
    }

    #[test]
    fn sanitize_filename_replaces_illegal_characters() {
        assert_eq!(sanitize_filename("Rock/Drum: Bus?"), "Rock_Drum_ Bus_");
    }

    #[test]
    fn sanitize_filename_falls_back_to_untitled_when_empty() {
        assert_eq!(sanitize_filename("   "), "Untitled");
        assert_eq!(sanitize_filename(""), "Untitled");
    }

    #[test]
    fn save_and_load_round_trips_in_a_scratch_dir() {
        let dir = scratch_dir();
        let preset = sample_preset("My Vocal Chain");

        let path = save_preset_in_dir(&preset, &dir).expect("save");
        assert!(path.exists());

        let loaded = load_preset_file(&path).expect("load");
        assert_eq!(loaded.name, preset.name);
        assert_eq!(loaded.params.len(), preset.params.len());
        for (id, value) in &preset.params {
            let loaded_value = loaded.params.get(id).expect("param present after reload");
            assert!(crate::presets::value_ext::param_values_equal(
                value,
                loaded_value
            ));
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn list_presets_in_dir_is_empty_for_a_missing_directory() {
        let dir = scratch_dir();
        assert!(list_presets_in_dir(&dir).expect("list").is_empty());
    }

    #[test]
    fn list_presets_in_dir_finds_every_saved_preset() {
        let dir = scratch_dir();
        save_preset_in_dir(&sample_preset("Alpha"), &dir).expect("save alpha");
        save_preset_in_dir(&sample_preset("Beta"), &dir).expect("save beta");

        let names: Vec<String> = list_presets_in_dir(&dir)
            .expect("list")
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, vec!["Alpha".to_string(), "Beta".to_string()]);

        fs::remove_dir_all(&dir).ok();
    }
}
