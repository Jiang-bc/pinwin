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

#[derive(clap::Parser)]
#[command(
    name = "PinWin",
    version,
    about = "Portable native Windows window manager",
    disable_help_flag = true,
    disable_version_flag = true
)]
#[command(group(clap::ArgGroup::new("action").args(["help", "version", "minimized", "list_windows", "print_settings", "unpin_all", "pin_hwnd", "unpin_hwnd", "pin_title"])))]
struct Arguments {
    #[arg(short = 'h', long)]
    help: bool,
    #[arg(short = 'v', long)]
    version: bool,
    #[arg(long, alias = "tray")]
    minimized: bool,
    #[arg(long, alias = "list")]
    list_windows: bool,
    #[arg(long)]
    print_settings: bool,
    #[arg(long)]
    unpin_all: bool,
    #[arg(long, value_name = "HWND")]
    pin_hwnd: Option<String>,
    #[arg(long, value_name = "HWND")]
    unpin_hwnd: Option<String>,
    #[arg(long, value_name = "SUBSTRING")]
    pin_title: Option<String>,
    #[arg(long, value_name = "0..255 (clamped to 30..255)")]
    opacity: Option<u8>,
}

pub fn parse_cli_args<I, S>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    use clap::Parser;
    let args = args.into_iter().map(|s| {
        let s = s.as_ref();
        if s == "/?" {
            "--help".to_owned()
        } else {
            s.to_owned()
        }
    });
    let a = Arguments::try_parse_from(args).map_err(|e| e.to_string())?;
    if a.opacity.is_some() && a.pin_hwnd.is_none() && a.pin_title.is_none() {
        return Err("--opacity requires --pin-hwnd or --pin-title".into());
    }
    let opacity = a.opacity.map(|v| v.clamp(30, 255));
    Ok(if a.help {
        CliAction::Help
    } else if a.version {
        CliAction::Version
    } else if let Some(raw) = a.pin_hwnd {
        CliAction::PinHwnd {
            hwnd: parse_hwnd(&raw)?,
            opacity,
        }
    } else if let Some(raw) = a.unpin_hwnd {
        CliAction::UnpinHwnd {
            hwnd: parse_hwnd(&raw)?,
        }
    } else if let Some(pattern) = a.pin_title {
        if pattern.trim().is_empty() {
            return Err("Window title pattern cannot be empty".into());
        }
        CliAction::PinTitle { pattern, opacity }
    } else if a.list_windows {
        CliAction::ListWindows
    } else if a.print_settings {
        CliAction::PrintSettings
    } else if a.unpin_all {
        CliAction::UnpinAllTopMost
    } else {
        CliAction::RunGui {
            start_minimized: a.minimized,
        }
    })
}

pub fn parse_hwnd(raw: &str) -> Result<HWND, String> {
    let trimmed = raw.trim();
    let value = if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        usize::from_str_radix(hex, 16)
    } else {
        trimmed.parse::<usize>()
    }
    .map_err(|_| format!("Invalid HWND: {raw}"))?;
    if value == 0 {
        return Err("HWND cannot be zero".into());
    }
    Ok(value as HWND)
}

pub fn request_unpin_all() -> Result<usize, String> {
    use crate::winapi::{ffi::*, to_wide};
    unsafe {
        let class = to_wide("PinWin_MainForm_Class");
        let hwnd = FindWindowW(class.as_ptr(), std::ptr::null());
        if hwnd.is_null() {
            return Err("PinWin is not running; no managed windows were changed.".into());
        }
        let mut result = 0;
        if SendMessageTimeoutW(
            hwnd,
            crate::pin_manager::WM_PINWIN_UNPIN_ALL,
            0,
            0,
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            1500,
            &mut result,
        ) == 0
            || result == usize::MAX
        {
            return Err("The running PinWin instance did not respond.".into());
        }
        Ok(result)
    }
}

pub fn execute_cli_action(action: CliAction) -> i32 {
    match action {
        CliAction::Help => {
            use clap::CommandFactory;
            let mut command = Arguments::command();
            let _ = command.print_long_help();
            println!(
                "\n\nExplicit HWND/title commands act directly and do not create GUI pin records.\n--unpin-all restores only windows managed by the running PinWin instance."
            );
            0
        }
        CliAction::Version => {
            println!(
                "PinWin v{} (Rust native Windows {})",
                VERSION,
                std::env::consts::ARCH
            );
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
        CliAction::UnpinAllTopMost => match request_unpin_all() {
            Ok(count) => {
                println!("Queued restoration of {count} managed window(s).");
                0
            }
            Err(error) => {
                eprintln!("{error}");
                1
            }
        },

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
