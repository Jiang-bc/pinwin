# Local validation — 2026-10-31

Validated on Windows 11 x64, PowerShell 7.6.9, Rust/Cargo 1.99.1/1.99.1 with the stable x86_64-pc-windows-msvc toolchain. This is a local result, not a claim about remote CI.

## Automated checks

- 42 application tests: 16 library, one asset-container, two CLI, one list-selection, four configuration and 18 live Win32 integration tests.
- `cargo fmt --check` and strict `cargo clippy --all-targets --locked -- -D warnings`.
- `cargo check --locked` and optimized `cargo build --release --locked` with embedded Windows resources.
- Independent artwork tool format and strict Clippy checks; regeneration leaves tracked assets byte-identical.
- `tools/smoke.py` against the release EXE copied into a temporary writable portable folder. It creates and destroys its own desktop windows and closes its own PinWin process.

The release smoke inspects the actual PE GUI subsystem, embedded version resource, extracted large/small application icons, and embedded `asInvoker`, Common Controls and PerMonitorV2 manifest. It tests release CLI output/exit codes, managed pinning, following a moved target, two-item selection, opacity, click-through, tray-command ghost recovery, CLI managed bulk restoration, overlay selection/cancellation, repeated Settings opening/cancellation without accumulating GDI objects, portable JSON saving and orderly-exit original-state restoration. It leaves no process from the smoke run.

Independent review found and prompted fixes for recursive settings edit notifications, picker paths relative to the wrong base, retained DPI fonts and insufficient scaled badge spacing. Release smoke also exposed reentrant synchronous CLI bulk restoration during cross-thread synchronization. The sent request now queues the actual mutation onto the GUI message loop, and CLI output explicitly acknowledges queueing. Six repeated pin/opacity/CLI-bulk/restore cycles were added to the smoke test; they pass along with Settings behavior and resource stability.

## Test-environment corrections

Two original-state tests used hidden windows whose topmost style behaved differently after earlier desktop tests; they now create visible test windows matching normal use. The no-instance bulk test no longer sends a request to an unrelated running GUI. During review a temporary debug instance briefly locked the default build output; the full test suite was also verified under `CARGO_TARGET_DIR=target/verify`. The smoke harness now uses dedicated native top-level windows and pumps their message queue while PinWin performs cross-thread window operations. The terminal's topmost window was confirmed to obscure earlier selection points; disposable targets start topmost to avoid that interference, and the harness verifies the hit target before sending selection and waits for a managed list entry. Restoration of initially non-topmost targets remains covered by live Rust integration tests. This also removes Tk's independent window-style management from restoration assertions.

## Artifacts

`target/release/PinWin.exe` and `dist/PinWin-2.0.0-windows-x64.zip` are generated locally by `tools/package.ps1`. The EXE is approximately 1.45 MB (decimal) on this MSVC build; the ZIP is approximately 0.40 MB. Exact lengths and SHA256 are printed by the packaging command. The ZIP contains the EXE, README and MIT license; no installer is generated.

## Manual coverage limits

Physical mixed-DPI monitor transitions, negative-coordinate monitor arrangements, visual appearance on every Windows theme, Explorer restart and interactions with elevated/protected third-party applications have not all been exercised manually. To check: move main/settings windows and pinned targets across monitors at 100/150/200%, verify controls and badge spacing; select targets on left/up monitors; restart Explorer and check the tray icon returns; compare normal and elevated target behavior. Window APIs, DPI calculations, cursor hotspots and virtual desktop coordinate paths have automated coverage, but that does not replace these physical display checks.
