use pinwin::keys::KeyCombo;
use pinwin::settings::ApplicationSettings;
use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;

#[test]
fn snake_case_unicode_and_disabled_shortcuts() {
    let settings = ApplicationSettings::from_json_str(
        r#"{"shortcut_pin_window_prompt":"","tray_icon_path":"中📌\\tray.ico","default_opacity":999}"#,
    ).unwrap();
    assert_eq!(settings.shortcut_pin_window_prompt, KeyCombo::NONE);
    assert_eq!(settings.tray_icon_path, "中📌\\tray.ico");
    assert_eq!(settings.default_opacity, 255);
    assert!(settings.highlight_target_on_hover);
    let json = settings.to_json_pretty();
    assert!(json.contains("\"shortcut_pin_window_prompt\": \"\""));
    assert_eq!(ApplicationSettings::from_json_str(&json).unwrap(), settings);
    assert_eq!(
        ApplicationSettings::from_json_str(r#"{"default_opacity":-1}"#)
            .unwrap()
            .default_opacity,
        30
    );
}

#[test]
fn malformed_and_duplicate_settings_are_rejected() {
    for json in [
        "{broken}",
        "[]",
        r#"{"default_opacity":70,"default_opacity":80}"#,
        r#"{"tray_icon_path":"\uD800"}"#,
    ] {
        assert!(
            ApplicationSettings::from_json_str(json).is_err(),
            "accepted {json}"
        );
    }
}

#[test]
fn default_settings_path_is_beside_executable() {
    let expected = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("settings.json");
    assert_eq!(
        ApplicationSettings::get_settings_file_path("settings.json", true),
        expected
    );
    assert_eq!(
        ApplicationSettings::get_settings_file_path("settings.json", false),
        expected
    );
}

#[test]
fn atomic_save_preserves_previous_file_on_failure() {
    let dir = std::env::temp_dir().join(format!("pinwin-settings-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("settings.json");
    let original = b"original settings must survive";
    fs::write(&path, original).unwrap();
    let locked = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    assert!(
        ApplicationSettings::default()
            .save_to_explicit_path(&path)
            .is_err()
    );
    drop(locked);
    assert_eq!(fs::read(&path).unwrap(), original);
    ApplicationSettings::default()
        .save_to_explicit_path(&path)
        .unwrap();
    assert!(ApplicationSettings::from_json_str(&fs::read_to_string(&path).unwrap()).is_ok());
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
    fs::remove_dir_all(dir).unwrap();
}
