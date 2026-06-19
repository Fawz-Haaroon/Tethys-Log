# Changelog

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- Markdown heading styles (h1/h2/h3) with distinct font scale and weight
- Markdown inline code, bold, italic, horizontal rule, blockquote, and list markup
- Rich code syntax: macro calls (`macro!`), ALL_CAPS constants, decorators/attributes (`@`, `#[…]`)
- Code fence header line styled separately from the block body
- Resize grip rebuilt as 22×22 DrawingArea with hover brightness feedback

### Changed
- Enhanced Tokyo Night palette — higher-contrast, more vivid colours for all token types
- Resize grip uses `PropagationPhase::Capture` — fast drags no longer accidentally select text
- Path construction centralised in `storage::paths::note_path`; callers no longer duplicate the formula

---

## [0.1.0] - 2026-05-30

Initial public release.

### Added
- Multi-tab note editor — Cantarell chrome, Adwaita Mono editor font
- Vim motions: Normal / Insert / Visual modes with block / I-beam cursor switching
- `/` search with `n` / `N` next/previous match navigation
- Inline image support — file picker, clipboard paste, drag-and-drop
- Inline video support — local file picker and drag-and-drop
- Web embed cards for YouTube, Instagram, Pinterest, Rumble, Dailymotion,
  Twitch, Reddit, Bilibili, Niconico, SoundCloud, Streamable, and more
- `yt-dlp`-powered inline YouTube playback via GStreamer; degrades gracefully
  when `yt-dlp` is absent
- Tokyo Night syntax highlighting inside fenced code blocks (Rust, Python,
  JavaScript / TypeScript, Go, and generic fallback)
- Active tab title shown in the status-bar path label
- Image and Video attach buttons in the workspace status bar
- Drag-to-resize grip on inline image and video widgets
- `install.sh` — system-wide and user-local install / uninstall
- `.desktop` entry and SVG icon for the application grid
- Note files stored as `~/.local/share/tethys-log/notes/<id>.tlog`
