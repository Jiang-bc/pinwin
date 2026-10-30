//! PinWin executable entry point.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pinwin::cli::{CliAction, execute_cli_action, parse_cli_args};
use pinwin::ui::run_gui_app;
use pinwin::winapi::{SingleInstanceManager, enable_high_dpi_awareness};

fn attach_parent_console() {
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }
}

fn main() {
    enable_high_dpi_awareness();

    let action = match parse_cli_args(std::env::args()) {
        Ok(act) => act,
        Err(err) => {
            attach_parent_console();
            eprintln!("Error: {}", err);
            std::process::exit(2);
        }
    };

    match action {
        CliAction::RunGui { start_minimized } => {
            let mut single_instance =
                SingleInstanceManager::new("Local\\PinWin_SingleInstance_Mutex");
            if !single_instance.is_first_instance(1000) {
                eprintln!("PinWin is already running.");
                return;
            }
            let code = run_gui_app(start_minimized);
            if code != 0 {
                std::process::exit(code);
            }
        }
        other => {
            attach_parent_console();
            let code = execute_cli_action(other);
            if code != 0 {
                std::process::exit(code);
            }
        }
    }
}
