# PinWin v2.0 (Rust Native Rewrite)

**PinWin** is a lightweight, high-performance Windows utility to pin any window **Always on Top** (`WS_EX_TOPMOST`) — originally created by [Victor Zakharov](https://github.com/VictorZakharov/pinwin) in C# (`.NET Framework 4.0` WinForms) as an open-source replacement for DeskPins, and **completely rewritten and modernized in native Rust (`rustc 1.99.1`)**.

---

## Key Optimizations & New Capabilities in v2.0 (Rust Edition)

| Dimension | Legacy v0.2.2 (C# / .NET 4.0) | Modernized v2.0.0 (Rust Native Win32) |
| :--- | :--- | :--- |
| **Language & Runtime** | C# `.NET Framework 4.0` + `Newtonsoft.Json` | Pure **Rust 2021 (`rustc 1.99.1`)** — **0 external crate dependencies** |
| **Executable Artifact** | Multi-file `.exe` + `.dll` + `.NET` runtime required | Single **449 KB** standalone native `PinWin.exe` (all icons & bitmaps embedded via `include_bytes!`) |
| **Position Synchronization** | 10 ms `WinForms.Timer` polling loop + `Thread.Sleep(1)` on UI thread | Event-driven `SetWinEventHook` (`EVENT_OBJECT_LOCATIONCHANGE`, `EVENT_SYSTEM_MOVESIZEEND`, `EVENT_OBJECT_DESTROY`, `EVENT_OBJECT_NAMECHANGE`) + lightweight 250 ms non-blocking health check |
| **Window Title Lookup** | Blocking `SendMessage(WM_GETTEXT)` — freezes PinWin if any target app hangs | Non-blocking `SendMessageTimeoutW(..., SMTO_ABORTIFHUNG, 200ms)` with `GetWindowTextW` fallback |
| **High-DPI & Multi-Monitor** | Legacy `SetProcessDPIAware()` (Issue #17: broken on mixed-DPI displays) | `SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)` + `DWMWA_EXTENDED_FRAME_BOUNDS` |
| **Pin Badge Placement** | Fixed `GetSystemMetrics` formula (Issue #26: overlaps custom title bars) | `DWMWA_CAPTION_BUTTON_BOUNDS` query with automatic bounds clamping |
| **Window Opacity Control** | Not supported | **20%–100% Alpha Opacity** via Pin badge mouse-wheel, right-click menu, main GUI buttons, or CLI `--opacity` |
| **Click-Through (Ghost Mode)** | Not supported | **`WS_EX_TRANSPARENT | WS_EX_LAYERED` Ghost Mode** to click right through reference overlays |
| **Interactive Overlay HUD** | Static screenshot with crosshair cursor | Double-buffered GDI screenshot + **live cyan target window bounding-box highlight** + **real-time HUD coordinate/title bar** |
| **Headless CLI Automation** | GUI-only | Full CLI support (`--list-windows`, `--pin-hwnd`, `--unpin-hwnd`, `--pin-title`, `--opacity`, `--unpin-all`, `--print-settings`) |

---

## Project Architecture (`src/`)

- `src/lib.rs` — Library crate root exposing all modules for unit and integration testing.
- `src/main.rs` — Application entry point (Per-Monitor V2 DPI setup, CLI dispatch, `SingleInstanceManager` mutex guard, GUI launch).
- `src/algorithms.rs` — Generic `HierarchyLookup<T>` trait & `FnHierarchyLookup` with `MAX_HIERARCHY_DEPTH = 256` cycle protection.
- `src/keys.rs` — `KeyCombo` bitmask representation and full bidirectional `KeysStringConverter` (`"CTRL + F11"`, `"SHIFT + ALT + P"`).
- `src/relative_path.rs` — Windows path normalization, `get_relative_path`, `choose_optimal_path`, and asset path resolution.
- `src/debounce.rs` — Deterministic `DebounceTracker` state machine.
- `src/settings.rs` — Zero-dependency JSON serializer/parser compatible with `%APPDATA%\PinWin\settings.json`.
- `src/winapi/ffi.rs` — Raw `extern "system"` Win32 ABI declarations (`user32`, `gdi32`, `shell32`, `comdlg32`, `kernel32`, plus dynamic `dwmapi` / DPI symbols).
- `src/winapi/mod.rs` — Safe Rust wrappers (`ApiTopMost`, `ApiWindowEffects`, `ApiWindowPos`, `WindowStyle`, `ApiWindowTitle`, `ApiOwnerFormLookup`, `ApiForm`, `ApiHotKey`, `ApiResources`, `SingleInstanceManager`).
- `src/pin_manager.rs` — `PinManager`, floating `PinBadge` window (`PinWin_BadgeWindow_Class`), `SetWinEventHook` callbacks, badge context menu, and mouse-wheel opacity adjustment.
- `src/overlay.rs` — Full-virtual-desktop `DesktopOverlay::select_point` with double-buffered GDI rendering, target window highlight frame, and HUD banner.
- `src/ui.rs` — Native Win32 `MainForm`, `Shell_NotifyIconW` system tray integration, and modal `SettingsDialog` with live shortcut key capture and custom icon preview.
- `src/cli.rs` — Command-line parser and headless window automation actions.
- `tests/win32_integration_test.rs` — Live Win32 integration test suite covering parent-child `HWND` hierarchy, `WS_EX_TOPMOST`, `SendMessageTimeoutW`, layered opacity/click-through, embedded assets, mutex locking, and `PinManager` lifecycle.

---

## Building & Testing

```bash
# Run unit tests + live Win32 integration tests
cargo test

# Run strict Clippy static analysis
cargo clippy --all-targets -- -D warnings

# Build optimized standalone release binary (LTO + strip)
cargo build --release
```

The compiled standalone binary is emitted at `target/release/PinWin.exe`.

---

## CLI Usage

```text
PinWin v2.0.0 (Rust Rewrite) — Native Win32 Always-on-Top Window Manager

USAGE:
  PinWin.exe [OPTIONS]

OPTIONS:
  -h, --help                Show this help message
  -v, --version             Show version information
  --minimized, --tray       Start PinWin GUI minimized in the system tray
  --list-windows            List all visible top-level windows with HWND & TopMost state
  --pin-hwnd <HWND>         Set a window TopMost by HWND (decimal or 0xHEX)
  --unpin-hwnd <HWND>       Clear TopMost & restore opacity on a window by HWND
  --pin-title <SUBSTRING>   Set matching window(s) TopMost by title substring
  --opacity <30..255>       Optional alpha opacity when pinning via CLI
  --unpin-all               Clear TopMost on all user top-level windows
  --print-settings          Print active settings JSON
```

## License

MIT License
