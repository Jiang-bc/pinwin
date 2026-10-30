//! Portable settings stored next to the executable.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Deserializer, Serialize};
use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};

use crate::keys::KeyCombo;
mod shortcut_serde;

pub const SETTINGS_FILENAME: &str = "settings.json";
pub const DEFAULT_TRAY_ICON_PATH: &str = "Images/Tray.ico";
pub const DEFAULT_CAPTURE_ICON_PATH: &str = "Images/TargetWindowIcon.ico";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApplicationSettings {
    #[serde(with = "shortcut_serde", alias = "ShortcutPinWindowPrompt")]
    pub shortcut_pin_window_prompt: KeyCombo,
    #[serde(with = "shortcut_serde", alias = "ShortcutPinWindowUnderCursor")]
    pub shortcut_pin_window_under_cursor: KeyCombo,
    #[serde(alias = "TrayIconPath")]
    pub tray_icon_path: String,
    #[serde(alias = "CaptureIconPath")]
    pub capture_icon_path: String,
    #[serde(deserialize_with = "opacity", alias = "DefaultOpacity")]
    pub default_opacity: u8,
    #[serde(alias = "HighlightTargetOnHover")]
    pub highlight_target_on_hover: bool,
}

fn opacity<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    let value = serde_json::Number::deserialize(deserializer)?;
    let number = value
        .as_f64()
        .ok_or_else(|| serde::de::Error::custom("invalid opacity"))?;
    Ok(number.clamp(30.0, 255.0) as u8)
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

    pub fn to_json_pretty(&self) -> String {
        let mut normalized = self.clone();
        normalized.default_opacity = normalized.default_opacity.clamp(30, 255);
        serde_json::to_string_pretty(&normalized)
            .expect("settings contain only JSON-compatible values")
    }

    pub fn from_json_str(json: &str) -> Result<Self, String> {
        if !json.trim_start().starts_with('{') {
            return Err("Settings must be a JSON object".into());
        }
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let contents = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_json_str(&contents)
    }

    pub fn load_from_file(file_name: Option<&str>) -> Self {
        Self::load_from_path(&Self::get_settings_file_path(
            file_name.unwrap_or(SETTINGS_FILENAME),
            false,
        ))
        .unwrap_or_default()
    }

    pub fn save_to_file(&self, file_name: Option<&str>) -> bool {
        self.save_to_explicit_path(&Self::get_settings_file_path(
            file_name.unwrap_or(SETTINGS_FILENAME),
            true,
        ))
        .is_ok()
    }

    pub fn save_to_explicit_path(&self, path: &Path) -> io::Result<()> {
        static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let temp = path.with_file_name(format!(
            ".pinwin-{}-{}.tmp",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(self.to_json_pretty().as_bytes())?;
            file.sync_all()?;
            drop(file);
            let source: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let target: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe {
                MoveFileExW(
                    source.as_ptr(),
                    target.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }

    pub fn get_settings_file_path(file_name: &str, _for_save: bool) -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
            .join(file_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_json_roundtrip() {
        let s = ApplicationSettings {
            tray_icon_path: r"C:\Custom\Tray.ico".into(),
            default_opacity: 220,
            ..Default::default()
        };
        assert_eq!(
            ApplicationSettings::from_json_str(&s.to_json_pretty()).unwrap(),
            s
        );
    }

    #[test]
    fn legacy_input_is_accepted_but_output_is_modern() {
        let s = ApplicationSettings::from_json_str(
            r#"{"ShortcutPinWindowPrompt":null,"TrayIconPath":"Images\\Tray.ico"}"#,
        )
        .unwrap();
        assert!(s.shortcut_pin_window_prompt.is_none());
        assert_eq!(s.tray_icon_path, r"Images\Tray.ico");
        assert!(!s.to_json_pretty().contains("ShortcutPinWindowPrompt"));
    }
}
