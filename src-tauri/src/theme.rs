// User-authored themes from disk (issue #6).
//
// Each theme is a JSON file in the app config dir's `themes/` folder, mirroring
// an entry in the frontend's themes.js: a label, a swatch, and the token map.
// Invalid or unparseable files are skipped rather than failing the whole load,
// so a stray file can never break the theme picker.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct DiskTheme {
    /// Stable id; defaults to the file stem when omitted.
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub swatch: String,
    /// CSS custom properties, e.g. "--accent" -> "#5b8cff".
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
}

/// Parse every `*.json` theme in `dir`, filling in sensible fallbacks. A missing
/// directory yields an empty list (the common first-run case).
pub fn parse_dir(dir: &Path) -> Vec<DiskTheme> {
    let mut themes = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return themes,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mut theme) = serde_json::from_str::<DiskTheme>(&text) else {
            continue;
        };

        if theme.vars.is_empty() {
            continue; // nothing to apply
        }
        if theme.key.trim().is_empty() {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                theme.key = stem.to_string();
            }
        }
        if theme.key.trim().is_empty() {
            continue;
        }
        if theme.label.trim().is_empty() {
            theme.label = theme.key.clone();
        }
        if theme.swatch.trim().is_empty() {
            theme.swatch = theme
                .vars
                .get("--accent")
                .cloned()
                .unwrap_or_else(|| "#888888".to_string());
        }
        themes.push(theme);
    }

    themes.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    themes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_and_skips_the_rest() {
        let dir = std::env::temp_dir().join(format!("glint-themes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Valid, no explicit key -> key from file stem; no label -> label = key.
        std::fs::write(
            dir.join("nord.json"),
            r##"{ "swatch": "#88c0d0", "vars": { "--accent": "#88c0d0", "--ink": "#2e3440" } }"##,
        )
        .unwrap();
        // Valid with explicit fields.
        std::fs::write(
            dir.join("rose.json"),
            r##"{ "key": "rose", "label": "Rosé", "vars": { "--accent": "#e0567a" } }"##,
        )
        .unwrap();
        // Skipped: malformed JSON, wrong extension, and empty vars.
        std::fs::write(dir.join("broken.json"), "{ not json").unwrap();
        std::fs::write(dir.join("note.txt"), "ignore me").unwrap();
        std::fs::write(dir.join("empty.json"), r##"{ "label": "Empty", "vars": {} }"##).unwrap();

        let themes = parse_dir(&dir);
        assert_eq!(themes.len(), 2, "two valid themes");
        // Sorted by label: "Nord" (stem) then "Rosé".
        assert_eq!(themes[0].key, "nord");
        assert_eq!(themes[0].label, "nord"); // fell back to key
        assert_eq!(themes[0].vars.get("--accent").unwrap(), "#88c0d0");
        assert_eq!(themes[1].key, "rose");
        assert_eq!(themes[1].label, "Rosé");
        // swatch defaulted to --accent.
        assert_eq!(themes[1].swatch, "#e0567a");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_dir_is_empty() {
        let themes = parse_dir(Path::new("/no/such/glint/themes/dir"));
        assert!(themes.is_empty());
    }
}
