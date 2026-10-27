//! Application settings and JSON persistence (`%APPDATA%\PinWin\settings.json`).
//!
//! Ported and enhanced from:
//! - `PinWin/BusinessLayer/ApplicationSettings.cs`
//! - `PinWin/BusinessLayer/ApplicationSettingsJson.cs`
//! - `PinWin/BusinessLayer/JsonKeysConverter.cs`

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::keys::KeyCombo;

pub const SETTINGS_FILENAME: &str = "settings.json";
pub const SETTINGS_FOLDER: &str = "PinWin";

pub const DEFAULT_TRAY_ICON_PATH: &str = "Images/Tray.ico";
pub const DEFAULT_CAPTURE_ICON_PATH: &str = "Images/TargetWindowIcon.ico";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationSettings {
    pub shortcut_pin_window_prompt: KeyCombo,
    pub shortcut_pin_window_under_cursor: KeyCombo,
    pub tray_icon_path: String,
    pub capture_icon_path: String,
    /// Default opacity for newly pinned windows (30..=255, default 255 = 100% opaque).
    pub default_opacity: u8,
    /// Highlight target window bounding box in the selection overlay.
    pub highlight_target_on_hover: bool,
}

impl Default for ApplicationSettings {
    fn default() -> Self {
        Self {
            shortcut_pin_window_prompt: KeyCombo::DEFAULT_PIN_PROMPT,
            shortcut_pin_window_under_cursor: KeyCombo::DEFAULT_PIN_UNDER_CURSOR,
            tray_icon_path: String::new(),
            capture_icon_path: String::new(),
            default_opacity: 255,
            highlight_target_on_hover: true,
        }
    }
}

impl ApplicationSettings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialize to pretty-printed JSON fully compatible with original PinWin's `settings.json`.
    pub fn to_json_pretty(&self) -> String {
        let prompt_val = match self.shortcut_pin_window_prompt.to_shortcut_string() {
            Some(s) => format!("\"{}\"", escape_json_str(&s)),
            None => "null".to_string(),
        };
        let cursor_val = match self.shortcut_pin_window_under_cursor.to_shortcut_string() {
            Some(s) => format!("\"{}\"", escape_json_str(&s)),
            None => "null".to_string(),
        };

        format!(
            "{{\r\n  \"ShortcutPinWindowPrompt\": {},\r\n  \"ShortcutPinWindowUnderCursor\": {},\r\n  \"TrayIconPath\": \"{}\",\r\n  \"CaptureIconPath\": \"{}\",\r\n  \"DefaultOpacity\": {},\r\n  \"HighlightTargetOnHover\": {}\r\n}}",
            prompt_val,
            cursor_val,
            escape_json_str(&self.tray_icon_path),
            escape_json_str(&self.capture_icon_path),
            self.default_opacity.clamp(30, 255),
            if self.highlight_target_on_hover { "true" } else { "false" }
        )
    }

    /// Parse settings from JSON string. Gracefully handles legacy 4-field JSON, nulls, or extra fields.
    pub fn from_json_str(json: &str) -> Result<Self, String> {
        let trimmed = json.trim();
        if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            return Err("Invalid JSON object".to_string());
        }

        let mut settings = Self::default();

        if let Some(val) = extract_json_field(trimmed, "ShortcutPinWindowPrompt") {
            settings.shortcut_pin_window_prompt = match val {
                JsonValue::Str(s) => KeyCombo::from_shortcut_string(&s),
                JsonValue::Null => KeyCombo::NONE,
                _ => KeyCombo::DEFAULT_PIN_PROMPT,
            };
        }

        if let Some(val) = extract_json_field(trimmed, "ShortcutPinWindowUnderCursor") {
            settings.shortcut_pin_window_under_cursor = match val {
                JsonValue::Str(s) => KeyCombo::from_shortcut_string(&s),
                JsonValue::Null => KeyCombo::NONE,
                _ => KeyCombo::DEFAULT_PIN_UNDER_CURSOR,
            };
        }

        if let Some(JsonValue::Str(s)) = extract_json_field(trimmed, "TrayIconPath") {
            settings.tray_icon_path = s;
        }

        if let Some(JsonValue::Str(s)) = extract_json_field(trimmed, "CaptureIconPath") {
            settings.capture_icon_path = s;
        }

        if let Some(JsonValue::Number(n)) = extract_json_field(trimmed, "DefaultOpacity") {
            settings.default_opacity = (n as i64).clamp(30, 255) as u8;
        }

        if let Some(JsonValue::Bool(b)) = extract_json_field(trimmed, "HighlightTargetOnHover") {
            settings.highlight_target_on_hover = b;
        }

        Ok(settings)
    }

    /// Load settings from `%APPDATA%\PinWin\settings.json` (or local `file_name` fallback).
    pub fn load_from_file(file_name: Option<&str>) -> Self {
        let name = file_name.unwrap_or(SETTINGS_FILENAME);
        let path = Self::get_settings_file_path(name, false);
        match fs::read_to_string(&path) {
            Ok(contents) => Self::from_json_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save settings to `%APPDATA%\PinWin\settings.json`.
    pub fn save_to_file(&self, file_name: Option<&str>) -> bool {
        let name = file_name.unwrap_or(SETTINGS_FILENAME);
        let path = Self::get_settings_file_path(name, true);
        self.save_to_explicit_path(&path).is_ok()
    }

    pub fn save_to_explicit_path(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, self.to_json_pretty())
    }

    pub fn get_settings_file_path(file_name: &str, for_save: bool) -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let appdata_path = Path::new(&appdata).join(SETTINGS_FOLDER).join(file_name);
            if for_save || appdata_path.exists() {
                return appdata_path;
            }
        }
        PathBuf::from(file_name)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Str(String),
    Number(f64),
    Bool(bool),
    Null,
}

fn escape_json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '\r' => out.push_str(r"\r"),
            '\n' => out.push_str(r"\n"),
            '\t' => out.push_str(r"\t"),
            c => out.push(c),
        }
    }
    out
}

fn extract_json_field(json: &str, key: &str) -> Option<JsonValue> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let after_key = &json[idx + pattern.len()..];
    let colon_idx = after_key.find(':')?;
    let rest = after_key[colon_idx + 1..].trim_start();

    if let Some(after_quote) = rest.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = after_quote.chars();
        while let Some(c) = chars.next() {
            if c == '"' {
                return Some(JsonValue::Str(out));
            }
            if c == '\\' {
                if let Some(esc) = chars.next() {
                    match esc {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'r' => out.push('\r'),
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        other => out.push(other),
                    }
                }
            } else {
                out.push(c);
            }
        }
        None
    } else if rest.starts_with("null") {
        Some(JsonValue::Null)
    } else if rest.starts_with("true") {
        Some(JsonValue::Bool(true))
    } else if rest.starts_with("false") {
        Some(JsonValue::Bool(false))
    } else {
        let end = rest
            .find(|c: char| [',', '}', '\r', '\n', ' ', '\t'].contains(&c))
            .unwrap_or(rest.len());
        let num_str = &rest[..end];
        num_str.parse::<f64>().ok().map(JsonValue::Number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_json_roundtrip() {
        let s = ApplicationSettings {
            tray_icon_path: r"C:\Custom\Tray.ico".to_string(),
            capture_icon_path: r"Images\TargetWindowIcon.ico".to_string(),
            default_opacity: 220,
            ..Default::default()
        };

        let json = s.to_json_pretty();
        let parsed = ApplicationSettings::from_json_str(&json).unwrap();
        assert_eq!(parsed, s);
    }

    #[test]
    fn test_legacy_csharp_settings_json_compatibility() {
        let legacy_json = r#"{
  "ShortcutPinWindowPrompt": "CTRL + F11",
  "ShortcutPinWindowUnderCursor": "SHIFT + ALT + P",
  "TrayIconPath": "Images\\Tray.ico",
  "CaptureIconPath": ""
}"#;
        let parsed = ApplicationSettings::from_json_str(legacy_json).unwrap();
        assert_eq!(
            parsed.shortcut_pin_window_prompt,
            KeyCombo::DEFAULT_PIN_PROMPT
        );
        assert_eq!(
            parsed
                .shortcut_pin_window_under_cursor
                .to_shortcut_string()
                .as_deref(),
            Some("SHIFT + ALT + P")
        );
        assert_eq!(parsed.tray_icon_path, r"Images\Tray.ico");
        assert_eq!(parsed.capture_icon_path, "");
        assert_eq!(parsed.default_opacity, 255);
    }
}
