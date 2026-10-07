// Embed card widget — inline playback for YouTube, Instagram, TikTok, Vimeo, etc.
//
// Header bar:
//   [▶] <video title>                          [▶ Play] [↗ Open]
//
// Play ▶ — first click: yt-dlp downloads to a temp MP4, shows "Loading…"
//           on the button.  When done: reveals the inline player (the same
//           video_player::VideoPlayer local video uses -- see that file
//           for why this no longer goes through gtk::Video).
//           Stop ■: collapses the panel AND pauses playback.
//           Third click: re-reveals (no re-download).
//           Failure: falls back to xdg-open.
//
// Open ↗ — always opens the watch URL in the default browser.
//
// Title: fetched asynchronously via yt-dlp --get-title.
//
// Platform detection: driven by watch_url — sets the left accent stripe colour
// and the default title text before async title arrives.
//
// Thread model: std::sync::mpsc + glib::timeout_add_local polling (try_recv).
//
// Every successful download registers its temp path in DOWNLOADED_TEMP_FILES;
// app::boot sweeps that list on window close. Best-effort by nature -- a
// force-kill or crash skips it the same way it would skip any other exit
// handler -- but it means a normal quit doesn't leave downloaded video
// sitting in the OS temp directory forever, which is what happened before.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gtk::{glib, prelude::*, Align, Box, Button, Label, MediaFile, Orientation, Revealer};

use crate::editor::canvas::{
    embed::{platform_for_url, PlatformInfo},
    media_frame::MediaFrame,
    video_player::VideoPlayer,
    viewer,
};

const CARD_WIDTH: i32 = 860;
const POLL_INTERVAL: Duration = Duration::from_millis(150);
// Same numbers video_widget.rs uses for local video -- the whole point is
// that the two aren't allowed to drift into different defaults.
const VIDEO_MIN_W: i32 = 200;
const VIDEO_MIN_H: i32 = 120;
const VIDEO_DEFAULT_W: i32 = 800;
const VIDEO_DEFAULT_H: i32 = 450;

thread_local! {
    static DOWNLOADED_TEMP_FILES: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
}

/// Deletes every temp file a yt-dlp download wrote this session. Called
/// once from app::boot on window close -- see the module comment above
/// for what this does and doesn't cover.
pub fn cleanup_temp_downloads() {
    DOWNLOADED_TEMP_FILES.with(|cell| {
        for path in cell.borrow_mut().drain(..) {
            let _ = std::fs::remove_file(path);
        }
    });
}

pub struct EmbedCard {
    pub root: Box,
}

impl EmbedCard {
    /// Create an embed card for any URL yt-dlp can handle.
    /// Platform branding (name, accent colour) is derived from the URL automatically.
    /// `initial_size` is whatever dimensions were recovered from the note
    /// for this embed's *played* state -- the card itself doesn't use it,
    /// since it's a fixed-width preview until the user actually presses
    /// play, but wire_play_button below needs it to size the player
    /// correctly the moment it's built rather than starting over at a
    /// default every time the note is reopened.
    pub fn new(watch_url: &str, initial_size: Option<(i32, i32)>) -> Self {
        let PlatformInfo {
            name: platform_name,
            accent,
        } = platform_for_url(watch_url);

        let wrapper = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(0)
            .hexpand(false)
            .margin_top(6)
            .margin_bottom(6)
            .build();
        wrapper.set_size_request(CARD_WIDTH, -1);

        // ── header card row ──────────────────────────────────────────────────
        let card = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(10)
            .hexpand(false)
            .build();
        card.set_size_request(CARD_WIDTH, -1);
        card.add_css_class("embed-card");

        // apply per-instance accent stripe via an inline CssProvider
        {
            let provider = gtk::CssProvider::new();
            provider.load_from_data(&format!(".embed-card {{ border-left-color: {}; }}", accent));
            card.style_context()
                .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }

        let icon = Label::builder().label("▶").valign(Align::Center).build();
        icon.add_css_class("embed-icon");
        // tint the icon to match the platform accent
        {
            let p2 = gtk::CssProvider::new();
            p2.load_from_data(&format!(".embed-icon {{ color: {}; }}", accent));
            icon.style_context()
                .add_provider(&p2, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }

        let title_label = Label::builder()
            .label(platform_name)
            .halign(Align::Start)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        title_label.add_css_class("embed-title");

        let play_btn = Button::builder()
            .label("▶  Play")
            .valign(Align::Center)
            .build();
        play_btn.add_css_class("embed-play-btn");
        play_btn.set_cursor_from_name(Some("pointer"));

        let open_btn = Button::builder()
            .label("↗  Open")
            .valign(Align::Center)
            .build();
        open_btn.add_css_class("embed-open-btn");
        open_btn.set_cursor_from_name(Some("pointer"));

        {
            let url = watch_url.to_string();
            open_btn.connect_clicked(move |_| {
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            });
        }

        card.append(&icon);
        card.append(&title_label);
        card.append(&play_btn);
        card.append(&open_btn);

        // ── inline video panel (revealed after download) ─────────────────────
        let revealer = Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideDown)
            .transition_duration(220)
            .reveal_child(false)
            .build();

        let video_box = Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(4)
            .build();
        video_box.add_css_class("embed-video-box");
        revealer.set_child(Some(&video_box));

        wire_play_button(&play_btn, watch_url, &video_box, &revealer, initial_size);
        spawn_title_fetch(watch_url, &title_label);

        wrapper.append(&card);
        wrapper.append(&revealer);

        Self { root: wrapper }
    }

    pub fn widget(&self) -> &Box {
        &self.root
    }
}

// ── play button state machine ─────────────────────────────────────────────────
fn wire_play_button(
    play_btn: &Button,
    watch_url: &str,
    video_box: &Box,
    revealer: &Revealer,
    initial_size: Option<(i32, i32)>,
) {
    let showing = Rc::new(Cell::new(false));
    // Set once the first download finishes, so toggling off can pause the
    // actual stream instead of needing to downcast whatever's sitting in
    // video_box back into something playable.
    let current_stream: Rc<RefCell<Option<MediaFile>>> = Rc::new(RefCell::new(None));

    let video_box_ref = video_box.clone();
    let revealer_ref = revealer.clone();
    let watch_url_str = watch_url.to_string();
    let play_btn_clone = play_btn.clone();

    play_btn.connect_clicked(move |b| {
        // toggle off: collapse + pause
        if showing.get() {
            revealer_ref.set_reveal_child(false);
            b.set_label("▶  Play");
            showing.set(false);
            if let Some(m) = current_stream.borrow().as_ref() {
                m.pause();
            }
            return;
        }

        // re-reveal existing widget (no re-download)
        if video_box_ref.first_child().is_some() {
            revealer_ref.set_reveal_child(true);
            b.set_label("■  Stop");
            showing.set(true);
            return;
        }

        // first play: download, then show
        b.set_sensitive(false);
        b.set_label("Loading…");

        let vb = video_box_ref.clone();
        let rv = revealer_ref.clone();
        let pbtn = play_btn_clone.clone();
        let url_for_dl = watch_url_str.clone();
        let showing_clone = showing.clone();
        let stream_slot = current_stream.clone();

        let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();

        std::thread::spawn(move || {
            let _ = tx.send(download_via_yt_dlp(&url_for_dl));
        });

        glib::timeout_add_local(POLL_INTERVAL, move || {
            match rx.try_recv() {
                Ok(Some(path)) => {
                    while let Some(child) = vb.first_child() {
                        vb.remove(&child);
                    }

                    DOWNLOADED_TEMP_FILES.with(|cell| cell.borrow_mut().push(path.clone()));

                    let player = VideoPlayer::new(&path);
                    *stream_slot.borrow_mut() = Some(player.media());
                    player.play();

                    // Wrapped in the exact same MediaFrame local video
                    // uses, at the exact same default/minimum sizes --
                    // this is the whole point: an embedded video that's
                    // actually playing is a video object like any other,
                    // not a second, differently-behaved implementation.
                    // Expanding to the viewer reparents this same
                    // player's widget rather than building a second one
                    // -- see video_widget.rs's build_expand_handler and
                    // viewer.rs for why.
                    let (init_w, init_h) =
                        initial_size.unwrap_or((VIDEO_DEFAULT_W, VIDEO_DEFAULT_H));
                    let frame = MediaFrame::new(player.widget(), init_w, init_h, VIDEO_MIN_W, VIDEO_MIN_H);
                    frame.widget().add_css_class("video-widget");

                    let frame_weak = frame.widget().downgrade();
                    let expand: Rc<dyn Fn()> = Rc::new(move || {
                        let Some(root) = frame_weak.upgrade() else { return };
                        let Some(content) = root.child() else { return };
                        root.set_child(Option::<&gtk::Widget>::None);
                        let root_weak = frame_weak.clone();
                        viewer::show_reparented(&content, move |returned| {
                            if let Some(r) = root_weak.upgrade() {
                                returned.set_hexpand(true);
                                returned.set_vexpand(true);
                                r.set_child(Some(returned));
                            }
                        });
                    });
                    frame.connect_expand({
                        let expand = expand.clone();
                        move || expand()
                    });
                    player.connect_expand(move || expand());

                    vb.append(frame.widget());
                    // The Rust-level VideoPlayer's job past this point is
                    // done -- it drops naturally at the end of this match
                    // arm, same as ImageWidget::new's local `widget`
                    // already does at every call site in codec.rs. The
                    // widget tree (frame, and through it vb, now owns
                    // player.widget()) and the stream (stream_slot above,
                    // and the Picture's own paintable reference inside
                    // player.widget()) keep the actual playback alive
                    // independently of the Rust wrapper.

                    rv.set_reveal_child(true);
                    pbtn.set_label("■  Stop");
                    pbtn.set_sensitive(true);
                    showing_clone.set(true);
                    glib::ControlFlow::Break
                }
                Ok(None) => {
                    // Download failed — don't auto-open the browser.
                    // Show a clear "unavailable" state; the Open button still works.
                    pbtn.set_label("Unavailable");
                    pbtn.set_sensitive(false);
                    pbtn.set_tooltip_text(Some(
                        "Could not download for inline playback. Use Open to view in your browser.",
                    ));
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    pbtn.set_label("▶  Play");
                    pbtn.set_sensitive(true);
                    glib::ControlFlow::Break
                }
            }
        });
    });
}

// ── async title fetch ─────────────────────────────────────────────────────────
fn spawn_title_fetch(watch_url: &str, label: &Label) {
    let (tx, rx) = std::sync::mpsc::channel::<Option<String>>();
    let url = watch_url.to_string();

    std::thread::spawn(move || {
        let _ = tx.send(fetch_video_title(&url));
    });

    let lbl = label.clone();
    glib::timeout_add_local(Duration::from_millis(200), move || match rx.try_recv() {
        Ok(Some(title)) => {
            lbl.set_label(&title);
            glib::ControlFlow::Break
        }
        Ok(None) => glib::ControlFlow::Break,
        Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
    });
}

fn fetch_video_title(url: &str) -> Option<String> {
    let out = std::process::Command::new("yt-dlp")
        .args(["--get-title", "--no-playlist", url])
        .output()
        .ok()?;
    if out.status.success() {
        let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !t.is_empty() {
            Some(t)
        } else {
            None
        }
    } else {
        None
    }
}

// ── yt-dlp download ───────────────────────────────────────────────────────────
//
// Strategy: use -S (format sort) instead of -f (format filter).
//   -f "best[ext=mp4]"  →  rejects HLS/m3u8 streams entirely (Pinterest, etc.)
//   -S "res:480,ext:mp4:m4a"  →  prefers mp4/480p but accepts any available
//   --merge-output-format mp4  →  re-mux HLS segments into a single mp4
//
// Two attempts:
//   1. Resolution-capped + format-sorted  (fast, small file)
//   2. No constraints at all              (last resort for unusual streams)
fn download_via_yt_dlp(url: &str) -> Option<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    // Attempt 1: prefer ≤480p mp4, but accept any codec/container
    let dest1 = std::env::temp_dir().join(format!("tethys-log-embed-{stamp}.mp4"));
    let ok1 = std::process::Command::new("yt-dlp")
        .args([
            "--no-playlist",
            "--no-part",
            "-S",
            "res:480,ext:mp4:m4a",
            "--merge-output-format",
            "mp4",
            "-o",
            dest1.to_str()?,
            url,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok1 && dest1.exists() {
        return Some(dest1);
    }

    // Attempt 2: no format constraints — accept whatever yt-dlp can get
    let dest2 = std::env::temp_dir().join(format!("tethys-log-embed-{stamp}-b.mp4"));
    let ok2 = std::process::Command::new("yt-dlp")
        .args([
            "--no-playlist",
            "--no-part",
            "--merge-output-format",
            "mp4",
            "-o",
            dest2.to_str()?,
            url,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok2 && dest2.exists() {
        Some(dest2)
    } else {
        None
    }
}

// ── CSS ───────────────────────────────────────────────────────────────────────
pub const EMBED_CSS: &str = r#"
/* Embed card — compact header bar.
   Left accent stripe colour is set per-instance via an inline CssProvider
   so each platform gets its own brand colour (red for YouTube, pink for
   Instagram, teal for TikTok, etc.). */
.embed-card {
    background: rgba(18, 20, 26, 0.88);
    border: 1px solid rgba(255,255,255,0.09);
    border-left: 3px solid #5a7a9a;   /* default; overridden per-instance */
    border-radius: 6px;
    padding: 8px 12px;
}

.embed-icon {
    color: #c8312a;   /* overridden per-instance */
    font-size: 12pt;
    min-width: 20px;
}

.embed-title {
    color: #c4cdd4;
    font-family: "Cantarell", sans-serif;
    font-size: 9.5pt;
    font-weight: 600;
}

.embed-play-btn {
    background: rgba(255,255,255,0.10);
    border: 1px solid rgba(255,255,255,0.22);
    border-radius: 5px;
    box-shadow: none;
    color: #d8e4ee;
    font-family: "Cantarell", sans-serif;
    font-size: 9pt;
    font-weight: 700;
    padding: 4px 14px;
    min-height: 0;
}

.embed-play-btn:hover {
    background: rgba(255,255,255,0.18);
    color: #ffffff;
}

.embed-play-btn:disabled { opacity: 0.40; }

.embed-open-btn {
    background: transparent;
    border: 1px solid rgba(255,255,255,0.14);
    border-radius: 5px;
    box-shadow: none;
    color: #7a8896;
    font-family: "Cantarell", sans-serif;
    font-size: 9pt;
    font-weight: 500;
    padding: 4px 14px;
    min-height: 0;
}

.embed-open-btn:hover {
    background: rgba(255,255,255,0.08);
    color: #b0bcc8;
}

.embed-video-box {
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 0 0 6px 6px;
}
"#;
