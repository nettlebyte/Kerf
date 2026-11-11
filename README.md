<h1 align="center">Kerf</h1>
<p align="center">A fast, minimalist screenshot tool for Windows 10/11.<br>Written in Rust. Precise cuts, nothing in the way.</p>

<p align="center">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-FF5A1F">
  <img alt="Platform: Windows 10/11" src="https://img.shields.io/badge/platform-Windows%2010%2F11-151515">
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-151515">
</p>

<p align="center">
  <img alt="Kerf demo" src="docs/demo.gif" width="720">
</p>

## Why Kerf

A kerf is the thin cut a saw leaves. The tool is built around the same idea: one clean, precise cut of your screen, as fast as possible.

- **Instant.** Press a hotkey and the screen freezes immediately. Low latency, near-zero idle CPU, small memory footprint.
- **Above everything.** The overlay covers every monitor and sits over any window, the taskbar and fullscreen apps.
- **Precise.** Pixel magnifier with coordinates and color value, a 1px selection edge, keyboard nudging.
- **Minimal.** Flat Windows 11-style interface with a single accent color. No clutter, no accounts, no telemetry.
- **Configurable.** Hotkeys, formats, filename templates, overlay look and theme all live in one readable config file.

## Features

- Region, window, monitor and all-monitors capture
- Window detection (hover and click)
- Multi-monitor and mixed-DPI support (Per-Monitor DPI Aware v2)
- Copy to clipboard and/or save as PNG / JPG / WebP
- Pixel magnifier with HEX color picker
- Tray-only app, single instance, autostart option
- Light, dark and system themes, custom accent color
- Hot-reloaded `config.toml`
- Annotations (arrow, rectangle, text, pen, blur): *planned*

## Install

Download the latest `kerf.exe` from [Releases](https://github.com/fypcode/kerf/releases) and run it. Kerf lives in the system tray.

> The build is not code-signed yet, so Windows SmartScreen may warn on first launch. Click **More info → Run anyway**.

## Usage

| Action | Default |
| --- | --- |
| Start region capture | `PrintScreen` |
| Confirm selection | `Enter` |
| Cancel | `Esc` / right-click |
| Nudge selection | arrow keys (`Shift` = 10px) |
| Move selection while dragging | `Space` |
| Copy color under cursor | `C` |

All hotkeys can be changed in Settings.

## Configuration

Settings live in `%APPDATA%\Kerf\config.toml` and reload automatically when the file changes. You can also change everything from the Settings window in the tray menu.

Main options:

- hotkeys per capture mode
- save folder, filename template (default `Kerf_{date}_{time}`), format and quality
- action after capture: clipboard, save or both
- overlay: dim opacity, accent color (default `#FF5A1F`), border thickness, magnifier, crosshair, crop-mark corners
- theme: light, dark or system
- autostart, toasts, sound

## Build from source

Requires Rust (stable) and Windows 10/11.

```powershell
git clone https://github.com/<your-username>/kerf.git
cd kerf
cargo build --release
```

The binary will be at `target\release\kerf.exe`. The Cargo package is named `kerf-app`; the executable is `kerf`.

## Known limitations

- Windows with capture protection (DRM video, windows using `SetWindowDisplayAffinity`) appear black. This is an OS restriction.
- To use hotkeys while an elevated (admin) window is focused, run Kerf as administrator.

## Roadmap

- [ ] Annotation toolbar
- [ ] Screen recording (GIF / MP4)
- [ ] OCR text capture
- [ ] winget package

## Contributing

Issues and pull requests are welcome. Please keep changes focused and in line with the project's goals: performance, minimal design, convenience.

## License

[MIT](LICENSE) © 2026 fypcode
