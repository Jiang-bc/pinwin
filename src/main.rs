//! PinWin executable entry point (`PinWin/Program.cs` equivalent).

use pinwin::cli::{CliAction, execute_cli_action, parse_cli_args};
use pinwin::ui::run_gui_app;
use pinwin::winapi::{SingleInstanceManager, enable_high_dpi_awareness};

fn main() {
    enable_high_dpi_awareness();

    let action = match parse_cli_args(std::env::args()) {
        Ok(act) => act,
        Err(err) => {
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
            let code = execute_cli_action(other);
            if code != 0 {
                std::process::exit(code);
            }
        }
    }
}
