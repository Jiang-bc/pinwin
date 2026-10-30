# PinWin artwork

The application/tray pushpin was drawn with `remote-magpie/antigravity/gemini-3.1-flash-image`. The original response is preserved in `source/pinwin-ai.jpg`. The model returned a grayscale checkerboard JPEG despite the transparency request. The asset generator removes bright neutral pixels (channel spread <18 and maximum channel >100), preserving the red body and dark needle, and writes actual alpha to `source/pin.png`.

Prompt:

> Create a single application icon for PinWin, a lightweight Windows always-on-top utility. One bold red pushpin tilted diagonally, point directed bottom-left, rounded geometric red cap toward top-right. Modern clean vector-like flat design with a subtle dark red side plane, crisp charcoal needle, very simple silhouette clearly readable at 16 pixels. Center the pushpin within the square with 12 percent safe padding. Transparent background, no ground plane, no shadow outside the silhouette, no text, no letters, no border, no badge, no window, no other objects. Primary color warm vivid red #EF4444, deep red #B91C1C and dark charcoal #242936. Polished restrained Windows utility branding. Output actual transparent alpha.

The target crosshair is generated geometrically rather than using AI: exact alignment and cursor hotspots matter more than illustration. Each CUR frame has its hotspot at the center. `target.ico` is the matching settings preview. `pinned.png` is the 64px transparent badge source; `pinned.bmp` is a 24px magenta color-key rendering for the native badge window.

Rebuild from the repository root:

```powershell
cargo run --locked --manifest-path tools/assetgen/Cargo.toml -- .
```

`app.ico`: 16/20/24/32/48/64/128/256px. `tray.ico`: 16/20/24/32px. `target.cur`: 32/48/64px. ICO/CUR frames use 32-bit DIBs and AND masks. All outputs are deterministic. `preview.png` displays 16/20/24/32px on light and dark backgrounds. Image conversion dependencies are confined to the developer tool; they are not linked into PinWin.
