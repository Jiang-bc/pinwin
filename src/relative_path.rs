//! Relative path calculation and optimal path selection.
//!
//! Ported and enhanced from `PinWin/BusinessLayer/RelativePath.cs` and `FileFromDisk.cs`.

use std::path::{Component, Path, PathBuf};

/// Calculate a relative path from `from_dir` to `to_path` using Windows-style `\` separators.
pub fn get_relative_path(from_dir: &str, to_path: &str) -> Result<String, &'static str> {
    if from_dir.trim().is_empty() {
        return Err("fromPath cannot be empty");
    }
    if to_path.trim().is_empty() {
        return Err("toPath cannot be empty");
    }

    let from_norm = normalize_windows_path(from_dir);
    let to_norm = normalize_windows_path(to_path);

    let from_comps = split_components(&from_norm);
    let to_comps = split_components(&to_norm);

    if from_comps.is_empty() || to_comps.is_empty() {
        return Ok(to_norm);
    }

    if !from_comps[0].eq_ignore_ascii_case(&to_comps[0]) {
        return Ok(to_norm);
    }

    let mut common = 1usize;
    while common < from_comps.len()
        && common < to_comps.len()
        && from_comps[common].eq_ignore_ascii_case(&to_comps[common])
    {
        common += 1;
    }

    let mut result_parts: Vec<String> = Vec::new();
    for _ in common..from_comps.len() {
        result_parts.push("..".to_string());
    }
    for part in &to_comps[common..] {
        result_parts.push(part.clone());
    }

    if result_parts.is_empty() {
        Ok(".".to_string())
    } else {
        Ok(result_parts.join(r"\"))
    }
}

/// Choose the shorter representation between relative path and absolute path,
/// matching `FileFromDisk.Select` in the original C# implementation.
pub fn choose_optimal_path(base_dir: &str, selected_absolute_path: &str) -> String {
    if let Ok(rel) = get_relative_path(base_dir, selected_absolute_path) {
        if rel.len() < selected_absolute_path.len() {
            return rel;
        }
    }
    selected_absolute_path.to_string()
}

fn normalize_windows_path(raw: &str) -> String {
    raw.trim().replace('/', r"\")
}

fn split_components(path_str: &str) -> Vec<String> {
    let path = Path::new(path_str);
    let mut parts = Vec::new();
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => parts.push(p.as_os_str().to_string_lossy().into_owned()),
            Component::RootDir => {}
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.len() > 1 {
                    parts.pop();
                }
            }
            Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
        }
    }
    parts
}

/// Resolve an icon path against candidate directories (current dir, exe dir, `PinWin/Images`).
pub fn resolve_asset_path(raw_path: &str) -> Option<PathBuf> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let direct = PathBuf::from(trimmed);
    if direct.is_file() {
        return Some(direct);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join(trimmed);
            if candidate.is_file() {
                return Some(candidate);
            }
            let parent_candidate = exe_dir.join("..").join(trimmed);
            if parent_candidate.is_file() {
                return Some(parent_candidate);
            }
        }
    }
    let fallback = PathBuf::from("PinWin").join(trimmed);
    if fallback.is_file() {
        return Some(fallback);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_path_same_directory_tree() {
        let rel = get_relative_path(
            r"C:\Users\opp\projects\pinwin",
            r"C:\Users\opp\projects\pinwin\Images\Tray.ico",
        )
        .unwrap();
        assert_eq!(rel, r"Images\Tray.ico");
    }

    #[test]
    fn test_relative_path_parent_navigation() {
        let rel = get_relative_path(
            r"C:\Users\opp\projects\pinwin\bin\Release",
            r"C:\Users\opp\projects\pinwin\Images\Tray.ico",
        )
        .unwrap();
        assert_eq!(rel, r"..\..\Images\Tray.ico");
    }

    #[test]
    fn test_relative_path_different_drives() {
        let rel = get_relative_path(r"C:\Users\opp", r"D:\Icons\Tray.ico").unwrap();
        assert_eq!(rel, r"D:\Icons\Tray.ico");
    }

    #[test]
    fn test_choose_optimal_path_picks_shorter() {
        let chosen = choose_optimal_path(
            r"C:\Program Files\PinWin",
            r"C:\Program Files\PinWin\Images\Tray.ico",
        );
        assert_eq!(chosen, r"Images\Tray.ico");
    }
}
