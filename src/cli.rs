//! Command-Line Interface (`PinWin.exe --help`, `--list-windows`, `--pin-hwnd`, etc.).
//!
//! Enables scripting, automation, and headless verification in addition to the system tray GUI.

use crate::settings::ApplicationSettings;
use crate::winapi::ffi::{HWND, RectExt};
use crate::winapi::{
    ApiTopMost, ApiWindowEffects, ApiWindowPos, ApiWindowTitle, enumerate_top_level_windows,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    RunGui {
        start_minimized: bool,
    },
    Help,
    Version,
    ListWindows,
    PrintSettings,
    PinHwnd {
        hwnd: HWND,
        opacity: Option<u8>,
    },
    UnpinHwnd {
        hwnd: HWND,
    },
    PinTitle {
        pattern: String,
        opacity: Option<u8>,
    },
    UnpinAllTopMost,
}

pub fn parse_cli_args<I, S>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let vec: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if vec.len() <= 1 {
        return Ok(CliAction::RunGui {
            start_minimized: false,
        });
    }

    let mut i = 1usize;
    let mut opacity: Option<u8> = None;
    let mut action: Option<CliAction> = None;

    while i < vec.len() {
        match vec[i].as_str() {
            "-h" | "--help" | "/?" => return Ok(CliAction::Help),
            "-v" | "--version" => return Ok(CliAction::Version),
            "--minimized" | "--tray" => {
                action = Some(CliAction::RunGui {
                    start_minimized: true,
                });
            }
            "--list" | "--list-windows" => {
                action = Some(CliAction::ListWindows);
            }
            "--print-settings" => {
                action = Some(CliAction::PrintSettings);
            }
            "--unpin-all" => {
                action = Some(CliAction::UnpinAllTopMost);
            }
            "--opacity" => {
                i += 1;
                if i >= vec.len() {
                    return Err("--opacity requires a value (30..255)".to_string());
                }
                let val: u8 = vec[i]
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid opacity value: {}", vec[i]))?;
                opacity = Some(val.clamp(30, 255));
            }
            "--pin-hwnd" => {
                i += 1;
                if i >= vec.len() {
                    return Err(
                        "--pin-hwnd requires a window handle (decimal or 0xHEX)".to_string()
                    );
                }
                let hwnd = parse_hwnd(&vec[i])?;
                action = Some(CliAction::PinHwnd { hwnd, opacity });
            }
            "--unpin-hwnd" => {
                i += 1;
                if i >= vec.len() {
                    return Err(
                        "--unpin-hwnd requires a window handle (decimal or 0xHEX)".to_string()
                    );
                }
                let hwnd = parse_hwnd(&vec[i])?;
                action = Some(CliAction::UnpinHwnd { hwnd });
            }
            "--pin-title" => {
                i += 1;
                if i >= vec.len() {
                    return Err("--pin-title requires a substring pattern".to_string());
                }
                action = Some(CliAction::PinTitle {
                    pattern: vec[i].clone(),
                    opacity,
                });
            }
            other => {
                return Err(format!(
                    "Unknown argument: '{}'. Use --help for usage.",
                    other
                ));
            }
        }
        i += 1;
    }

    // Propagate trailing --opacity if specified after --pin-hwnd or --pin-title
    match action {
        Some(CliAction::PinHwnd {
            hwnd,
            opacity: None,
        }) => Ok(CliAction::PinHwnd { hwnd, opacity }),
        Some(CliAction::PinTitle {
            pattern,
            opacity: None,
        }) => Ok(CliAction::PinTitle { pattern, opacity }),
        Some(act) => Ok(act),
        None => Ok(CliAction::RunGui {
            start_minimized: false,
        }),
    }
}

pub fn parse_hwnd(raw: &str) -> Result<HWND, String> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        usize::from_str_radix(hex, 16)
            .map(|value| value as HWND)
            .map_err(|_| format!("Invalid hex HWND: {}", raw))
    } else {
        trimmed
            .parse::<usize>()
            .map(|value| value as HWND)
            .map_err(|_| format!("Invalid HWND: {}", raw))
    }
}

pub fn execute_cli_action(action: CliAction) -> i32 {
    match action {
        CliAction::Help => {
            println!(
                "PinWin v{} (Rust Rewrite) — Native Win32 Always-on-Top Window Manager",
                VERSION
            );
            println!();
            println!("USAGE:");
            println!("  PinWin.exe [OPTIONS]");
            println!();
            println!("OPTIONS:");
            println!("  -h, --help                Show this help message");
            println!("  -v, --version             Show version information");
            println!("  --minimized, --tray       Start PinWin GUI minimized in the system tray");
            println!(
                "  --list-windows            List all visible top-level windows with HWND & TopMost state"
            );
            println!("  --pin-hwnd <HWND>         Set a window TopMost by HWND (decimal or 0xHEX)");
            println!(
                "  --unpin-hwnd <HWND>       Clear TopMost & restore opacity on a window by HWND"
            );
            println!(
                "  --pin-title <SUBSTRING>   Set matching window(s) TopMost by title substring"
            );
            println!("  --opacity <30..255>       Optional alpha opacity when pinning via CLI");
            println!("  --unpin-all               Clear TopMost on all user top-level windows");
            println!("  --print-settings          Print active settings JSON");
            0
        }
        CliAction::Version => {
            println!("PinWin v{} (Rust native x86_64-pc-windows-gnu)", VERSION);
            0
        }
        CliAction::PrintSettings => {
            let s = ApplicationSettings::load_from_file(None);
            println!("{}", s.to_json_pretty());
            0
        }
        CliAction::ListWindows => {
            let windows = enumerate_top_level_windows();
            println!("{:<12}  {:<7}  {:<18}  TITLE", "HWND", "TOPMOST", "BOUNDS");
            for w in windows {
                let bounds_str = format!(
                    "{},{} {}x{}",
                    w.bounds.left,
                    w.bounds.top,
                    w.bounds.width(),
                    w.bounds.height()
                );
                println!(
                    "0x{:08X}    {:<7}  {:<18}  {}",
                    w.hwnd as usize,
                    if w.is_topmost { "YES" } else { "NO" },
                    bounds_str,
                    w.title
                );
            }
            0
        }
        CliAction::PinHwnd { hwnd, opacity } => {
            if !ApiWindowPos::is_window(hwnd) {
                eprintln!("Error: 0x{:X} is not a valid window handle.", hwnd as usize);
                return 1;
            }
            let ok = ApiTopMost::set(hwnd);
            if let Some(alpha) = opacity {
                let _ = ApiWindowEffects::set_opacity(hwnd, alpha);
            }
            let title = ApiWindowTitle::from_handle(hwnd);
            if ok {
                println!(
                    "Pinned 0x{:X} (\"{}\") -> TopMost=true",
                    hwnd as usize, title
                );
                0
            } else {
                eprintln!("Failed to pin 0x{:X} (\"{}\").", hwnd as usize, title);
                1
            }
        }
        CliAction::UnpinHwnd { hwnd } => {
            if !ApiWindowPos::is_window(hwnd) {
                eprintln!("Error: 0x{:X} is not a valid window handle.", hwnd as usize);
                return 1;
            }
            let _ = ApiWindowEffects::set_click_through(hwnd, false, 255);
            let _ = ApiWindowEffects::set_opacity(hwnd, 255);
            let ok = ApiTopMost::clear(hwnd);
            let title = ApiWindowTitle::from_handle(hwnd);
            if ok {
                println!(
                    "Unpinned 0x{:X} (\"{}\") -> TopMost=false",
                    hwnd as usize, title
                );
                0
            } else {
                eprintln!("Failed to unpin 0x{:X} (\"{}\").", hwnd as usize, title);
                1
            }
        }
        CliAction::PinTitle { pattern, opacity } => {
            let needle = pattern.to_lowercase();
            let windows = enumerate_top_level_windows();
            let mut matched = 0usize;
            for w in windows {
                if w.title.to_lowercase().contains(&needle) && ApiTopMost::set(w.hwnd) {
                    if let Some(alpha) = opacity {
                        let _ = ApiWindowEffects::set_opacity(w.hwnd, alpha);
                    }
                    println!("Pinned 0x{:X} (\"{}\")", w.hwnd as usize, w.title);
                    matched += 1;
                }
            }
            if matched == 0 {
                eprintln!("No visible window matched title pattern '{}'.", pattern);
                1
            } else {
                0
            }
        }
        CliAction::UnpinAllTopMost => {
            let windows = enumerate_top_level_windows();
            let mut count = 0usize;
            for w in windows {
                if w.is_topmost && ApiTopMost::clear(w.hwnd) {
                    count += 1;
                }
            }
            println!("Cleared TopMost on {} window(s).", count);
            0
        }
        CliAction::RunGui { .. } => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_variants() {
        assert_eq!(
            parse_cli_args(["PinWin.exe"]).unwrap(),
            CliAction::RunGui {
                start_minimized: false
            }
        );
        assert_eq!(
            parse_cli_args(["PinWin.exe", "--tray"]).unwrap(),
            CliAction::RunGui {
                start_minimized: true
            }
        );
        assert_eq!(
            parse_cli_args(["PinWin.exe", "--pin-hwnd", "0x1234", "--opacity", "200"]).unwrap(),
            CliAction::PinHwnd {
                hwnd: 0x1234 as HWND,
                opacity: Some(200)
            }
        );
    }
}
