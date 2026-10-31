# PinWin

PinWin is a portable Windows utility that keeps selected windows on top. The application is implemented in Rust 2024 with native Win32 controls and official `windows-sys` bindings. It continues the MIT-licensed [original PinWin](https://github.com/VictorZakharov/pinwin) by Victor Zakharov.

## Run

Extract the portable ZIP into a writable folder and open `PinWin.exe`. No installer or .NET runtime is required. Settings are saved as `settings.json` beside the executable; moving that folder moves the settings too. Built-in icons and the selection cursor are embedded.

- Select a window with **Ctrl + F11**, or pin the window under the cursor with **Ctrl + F12**.
- Click a floating pin badge to unpin. Right-click it for opacity and click-through options; its mouse wheel adjusts opacity.
- Manage selected windows in the main list or use the tray menu. Minimizing hides the main window; exiting restores the original state of windows managed by that process.
- Settings configure shortcuts, custom icons and target highlighting. An empty shortcut disables it. Custom asset paths are absolute or relative to the executable's folder.
- Click-through windows can be recovered through **Restore ghost windows** in the tray menu.

Normal use does not request administrator privileges. Windows may reject changes to elevated or protected applications. Windows 10/11 x64 with the MSVC toolchain is the supported build target. Multi-monitor coordinates, per-monitor DPI updates and Explorer tray recreation are handled by the native UI; see the validation notes for coverage limits.

## Command line

```powershell
.\PinWin.exe --help
.\PinWin.exe --tray
.\PinWin.exe --list-windows
.\PinWin.exe --pin-hwnd 0x1234 --opacity 200
.\PinWin.exe --pin-title "Reference"
.\PinWin.exe --unpin-hwnd 0x1234
.\PinWin.exe --unpin-all
.\PinWin.exe --print-settings
```

`--unpin-all` queues a request for the running PinWin GUI to restore **its managed windows only**. Success acknowledges the queued request; restoration runs on the GUI message loop. It fails if that instance is unavailable. Explicit HWND/title commands operate directly on the specified external windows; they do not create GUI-managed records or floating badges. Consequently they do not provide automatic restoration when the command exits. `--opacity` accepts a byte value and clamps it to 30–255. Conflicting actions, invalid handles and opacity without a pin action are rejected.

The release executable uses the Windows GUI subsystem and attaches to an existing parent console for CLI output. For reliable capture and exit codes in scripts, invoke it through a process API or `Start-Process -Wait` with redirected output.

## Configuration

New files use snake_case keys and empty strings for disabled shortcuts:

```json
{
  "shortcut_pin_window_prompt": "CTRL + F11",
  "shortcut_pin_window_under_cursor": "CTRL + F12",
  "tray_icon_path": "Images/Tray.ico",
  "capture_icon_path": "Images/TargetWindowIcon.ico",
  "default_opacity": 255,
  "highlight_target_on_hover": true
}
```

Missing custom files fall back to embedded assets. Legacy PascalCase keys and `null` shortcuts are accepted on input. Old AppData settings are not automatically imported: copy the desired file beside the EXE. Saves use an atomic file replacement; failed saves keep the previous file and show an error in the settings dialog.

## Build and validate

Install Rust stable and Visual Studio C++ Build Tools with the Windows SDK. This branch declares and was built with Rust 1.99 (locally `rustc 1.99.1`); earlier toolchains have not been validated.

```powershell
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
pwsh -NoProfile -File tools/package.ps1
python tools/smoke.py
```

`target/release/PinWin.exe` is the application. Packaging writes `dist/PinWin-2.0.0-windows-x64.zip` containing the EXE, README and MIT license. There is no installer or automatic startup registration. The build embeds the application icon, Cargo package version, `asInvoker` execution level, Common Controls v6 and PerMonitorV2 manifest. `python tools/smoke.py` checks the built PE resources and exercises the release GUI using windows it creates itself; it requires an interactive Windows desktop and Python 3.

Tests include real Win32 window lifecycle, original opacity/topmost restoration, rollback, identity checks, resource ownership, cursor hotspots and portable configuration. Live desktop tests need a Windows desktop session. CI runs those tests on a Windows runner, but CI results alone do not establish visual or physical multi-monitor correctness.

The artwork generator is a separate development tool, with no image decoding dependency in the application:

```powershell
cargo run --locked --manifest-path tools/assetgen/Cargo.toml -- .
```

See [assets/README.md](assets/README.md) for AI provenance and deterministic cursor/icon generation, and [docs/validation.md](docs/validation.md) for the local validation record and remaining manual checks.

## Source

- `src/ui/`: main window, tray, settings and DPI layout.
- `src/winapi/`: official API exports, resource owners and original window-state snapshots.
- `src/pin_manager.rs`, `src/overlay.rs`: managed window lifecycle, floating badges and selection.
- `src/settings.rs`, `src/cli.rs`: Serde configuration and Clap argument parsing.
- `resources/`, `build.rs`: executable icon, version and manifest.
- `tools/assetgen/`: reproducible artwork conversion.

MIT License. See [LICENSE](LICENSE).
