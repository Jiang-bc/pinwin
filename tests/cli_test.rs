use pinwin::cli::{CliAction, parse_cli_args, parse_hwnd};

#[test]
fn conflicting_actions_and_orphan_opacity_are_errors() {
    for args in [
        vec!["PinWin", "--list", "--pin-hwnd", "42"],
        vec!["PinWin", "--opacity", "120"],
        vec!["PinWin", "--pin-hwnd"],
        vec!["PinWin", "--unknown"],
        vec!["PinWin", "--pin-title", ""],
        vec!["PinWin", "--pin-hwnd", "0"],
        vec!["PinWin", "--pin-hwnd", "42", "--opacity", "256"],
    ] {
        assert!(parse_cli_args(args.clone()).is_err(), "accepted {args:?}");
    }
}

#[test]
fn aliases_handles_and_opacity_are_preserved() {
    assert_eq!(parse_cli_args(["PinWin", "/?"]).unwrap(), CliAction::Help);
    assert_eq!(
        parse_cli_args(["PinWin", "-v"]).unwrap(),
        CliAction::Version
    );
    for alias in ["--list", "--list-windows"] {
        assert_eq!(
            parse_cli_args(["PinWin", alias]).unwrap(),
            CliAction::ListWindows
        );
    }
    for alias in ["--tray", "--minimized"] {
        assert_eq!(
            parse_cli_args(["PinWin", alias]).unwrap(),
            CliAction::RunGui {
                start_minimized: true
            }
        );
    }
    assert_eq!(parse_hwnd("42").unwrap(), parse_hwnd("0X2A").unwrap());
    assert_eq!(
        parse_hwnd(&usize::MAX.to_string()).unwrap() as usize,
        usize::MAX
    );
    for args in [
        ["PinWin", "--pin-hwnd", "42", "--opacity", "0"],
        ["PinWin", "--opacity", "0", "--pin-hwnd", "42"],
    ] {
        assert_eq!(
            parse_cli_args(args).unwrap(),
            CliAction::PinHwnd {
                hwnd: 42usize as _,
                opacity: Some(30)
            }
        );
    }
}
