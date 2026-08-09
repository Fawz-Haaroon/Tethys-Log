# Tethys-Log

A simple text editor built with GTK4 and Rust, with support for viewing images and videos inline, web embeds, syntax-highlighted code blocks, vim motions, and much more.

## Features

- Multi-tab editing
- Vim motions (Normal, Insert, and Visual modes), with `/` search and `n`/`N` to cycle through matches
- Inline images — attach via file picker, clipboard paste, or drag-and-drop, with a resizable grip on each
- Inline video playback for local files, with YouTube support via yt-dlp and a resizable grip
- Web embed cards for YouTube, Instagram, Pinterest, Twitch, Reddit, Rumble, Dailymotion, Bilibili, Niconico, SoundCloud, and Streamable
- Syntax-highlighted code blocks (Rust, Python, JavaScript/TypeScript, Go)
- Basic markdown markup (headings, bold, italic, inline code, blockquotes, lists)

## Install

### Via Cargo

Install the runtime libraries listed below first, then:

```sh
cargo install tethys-log
```

### From source

```sh
git clone https://github.com/Fawz-Haaroon/Tethys-Log
cd Tethys-Log
cargo build --release

# System-wide install
sudo ./install.sh

# User install (no sudo)
./install.sh --user

# Uninstall
sudo ./install.sh --uninstall
./install.sh --uninstall --user
```

## Runtime dependencies

| Library                | Debian / Ubuntu                 | Fedora                          | Arch                     |
|------------------------|---------------------------------|---------------------------------|--------------------------|
| GTK 4                  | `libgtk-4-1`                    | `gtk4`                          | `gtk4`                   |
| GStreamer good plugins | `gstreamer1.0-plugins-good`     | `gstreamer1-plugins-good`       | `gstreamer-plugins-good` |
| GStreamer bad plugins  | `gstreamer1.0-plugins-bad`      | `gstreamer1-plugins-bad-free`   | `gstreamer-plugins-bad`  |
| gdk-pixbuf             | `libgdk-pixbuf-2.0-0`           | `gdk-pixbuf2`                   | `gdk-pixbuf2`            |

Building from source also requires the `-dev` / `-devel` counterparts and a Rust toolchain 1.85 or newer.

### Optional: yt-dlp for YouTube playback

```sh
sudo pacman -S yt-dlp       # Arch
sudo apt install yt-dlp     # Debian / Ubuntu
sudo dnf install yt-dlp     # Fedora
```

Without yt-dlp, YouTube embed cards fall back to an Open-in-browser button instead of playing inline.

## Vim motions

Default mode is Insert. Press **Escape** to enter Normal mode.

```
Normal mode
  h j k l     move left / down / up / right
  w b         word forward / backward
  0 $         line start / end
  gg G        buffer start / end
  x           delete character under cursor
  d           delete current line
  y / p       yank / paste line
  u / Ctrl+r  undo / redo
  v           enter Visual mode
  /           open search bar, then n / N to cycle matches

Normal to Insert
  i  a  o  O  A  I
```

## Keyboard shortcuts

| Keys             | Action                  |
|------------------|-------------------------|
| Ctrl+T           | New tab                 |
| Ctrl+W           | Close tab               |
| Ctrl+Shift+T     | Reopen last closed tab  |
| Ctrl+O           | Open file                |
| Ctrl+R           | Rename active tab       |
| Ctrl+F           | Find in note            |
| Ctrl+Tab         | Next tab                |
| Ctrl+Shift+Tab   | Previous tab            |
| Ctrl+1 to 9      | Jump to tab by position |
| Ctrl+= / -       | Zoom in / out           |
| Ctrl+0           | Reset zoom              |

## Opening files

Besides Ctrl+O, Tethys Log opens files handed to it from outside the app:

```
tethys-log path/to/note.tlog
```

Double-clicking a `.tlog` file in your file manager, or "Open With → Tethys
Log" on any text file, does the same thing — a `.desktop` entry and MIME
type are registered by `install.sh` so the system recognizes `.tlog` files
and offers Tethys Log for them. If the app is already running, the file
opens as a new tab in the existing window instead of starting a second copy;
a file that's already open is focused rather than opened twice.

What happens depends on the file:

- **`.tlog` files** open and save in place at the exact path you gave —
  the same way a plain text editor treats a `.txt` file. A path that
  doesn't exist yet works too: it starts as an empty tab and the file is
  created on first save, the same way `nvim newfile.txt` does.
- **Everything else** (`.md`, `.py`, source files, plain `.txt`, ...) is
  imported: a managed copy is made in `~/Tethys-Log/imports/` and edited
  from there, exactly like using the Open-file dialog today. The original
  file is left untouched — attaching an image or video later writes this
  app's own markers into the buffer, which isn't safe to do to a file
  you expect to stay plain text.
- **Folders** aren't opened — there's no per-folder workspace concept here,
  everything lives under the single `~/Tethys-Log/` store described below.

`.tlog` files are plain UTF-8 text (see below), so tools outside Tethys Log
— `cat`, `less`, `git diff`, GitHub's file viewer — display them normally
instead of reporting a binary file.

## Images and videos

Attach an image or video using the buttons in the status bar, by pasting from the clipboard, or by dragging a file directly into the note. Files are copied into your notes folder when attached, so the note stays intact even if the original file is moved or deleted. Each embedded image or video has a grip in the corner you can drag to resize it.

For web videos, paste a supported URL and it converts into an embed card automatically. YouTube links play inline; everything else shows a card with an Open button.

Supported platforms: YouTube, Instagram, Pinterest, Twitch, Reddit, Rumble, Dailymotion, Bilibili, Niconico, SoundCloud, Streamable.

## Notes storage

Everything lives in `~/Tethys-Log/` — a plain folder in your home directory you can open, copy, and back up like any other files.

```
~/Tethys-Log/
├── notes/       notes you create inside the app
├── imports/     copies of external files you opened (txt, md, etc.)
├── drafts/      unsaved and in-progress notes
├── media/
│   ├── images/  images attached to notes
│   └── videos/  videos attached to notes
└── session.json open tabs and last active tab
```

Uninstalling the app never touches this folder.

### The `.tlog` format

Notes are plain UTF-8 text files. Attached images, videos, and embeds are
stored as inline markers — a filename bracketed by a pair of
Unicode Private-Use-Area characters, e.g. `photo.png` between a pair of
markers around it — rather than embedded binary data, so the note itself
stays small and diffable regardless of what's attached to it. Notes written
before this scheme was introduced used C0 control characters (NUL/SOH/STX)
for the same purpose, which is what made those files look like binary data
to `file`, `git diff`, and GitHub's viewer; Tethys Log still reads that
older form, and rewrites it to the current one the first time the note is
saved again.
