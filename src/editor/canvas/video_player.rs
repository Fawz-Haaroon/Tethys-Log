// VideoPlayer — one playback implementation shared by local video
// (video_widget.rs) and a downloaded embed's inline playback
// (embed_widget.rs). Both are "play a local mp4 through GStreamer"; the
// only difference is where the file came from, which is the caller's
// problem, not this one's.
//
// Deliberately does NOT use gtk::Video. Video::for_filename brings its own
// built-in GtkMediaControls along for free -- a bottom transport bar that
// overlays the last slice of the frame, plus a large centered play button
// while paused, neither of which this file wants and neither of which
// gtk::Video exposes a way to turn off. Going one layer down -- a bare
// Picture with the MediaStream bound to it as a Paintable -- gets the same
// GStreamer-backed playback with a blank surface and full control over
// what sits on top of it.
//
// There is exactly one MediaFile per VideoPlayer, constructed once here
// and never replaced or duplicated for the lifetime of this struct.
// Entering the fullscreen viewer does not construct a second VideoPlayer
// -- viewer.rs reparents this same widget (and therefore this same
// MediaFile) into the viewer's slot and back again on close. That's
// deliberate: two independent MediaFile objects for what the user
// perceives as one video is exactly how a second, invisible audio stream
// ends up playing on its own, which an earlier version of this file did
// produce by giving the viewer its own fresh VideoPlayer.
//
// Looping is driven off the position poll below rather than off
// GtkMediaStream's `ended` notification. An earlier version trusted
// `connect_ended_notify` + `is_ended()`; tested against a real build, it
// did not reliably fire. Rather than guess at a second version of the
// same uncertain signal, looping reuses the poll timer this file already
// runs for the seek bar and timestamp, checking the same
// duration/position/is_playing values that display already depends on --
// every one of which was already proven working elsewhere in this file,
// unlike `ended`.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use gtk::{
    glib, prelude::*, Align, Box as GtkBox, Button, EventControllerMotion, Label, MediaFile,
    Orientation, Overlay, Picture, Scale,
};

const POLL_INTERVAL: Duration = Duration::from_millis(200);
const HIDE_AFTER: Duration = Duration::from_secs(2);
// How close to the end counts as "reached it" for loop-restart purposes.
// Wide enough to reliably catch the end despite the 200ms poll gap and
// GStreamer's own last-frame timestamp imprecision; narrow enough that
// pausing deliberately a couple of seconds before the end doesn't get
// mistaken for having finished.
const EOS_TOLERANCE_US: i64 = 300_000;

type ExpandCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

pub struct VideoPlayer {
    pub root: Overlay,
    media: MediaFile,
    loop_enabled: Rc<Cell<bool>>,
    on_expand: ExpandCallback,
}

impl VideoPlayer {
    pub fn new(path: &Path) -> Self {
        let media = MediaFile::for_filename(path);
        media.set_loop(false);

        let loop_enabled: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let on_expand: ExpandCallback = Rc::new(RefCell::new(None));

        let picture = Picture::new();
        picture.set_paintable(Some(&media));
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_can_shrink(true);

        let root = Overlay::new();
        root.add_css_class("video-player");
        root.set_child(Some(&picture));

        let bar = build_control_bar(&media, &loop_enabled, &on_expand);
        bar.set_halign(Align::Fill);
        bar.set_valign(Align::End);
        bar.set_visible(false);
        root.add_overlay(&bar);

        wire_auto_hide(&root, &bar);
        wire_click_to_toggle(&picture, &media);

        Self { root, media, loop_enabled, on_expand }
    }

    pub fn widget(&self) -> &Overlay { &self.root }

    pub fn play(&self) { self.media.play(); }

    pub fn pause(&self) { self.media.pause(); }

    pub fn is_playing(&self) -> bool { self.media.is_playing() }

    pub fn set_loop(&self, on: bool) {
        self.loop_enabled.set(on);
        self.media.set_loop(on);
    }

    pub fn is_loop(&self) -> bool { self.loop_enabled.get() }

    /// A cheap GObject clone of the underlying stream, for a caller that
    /// needs its own handle without holding onto the whole VideoPlayer.
    pub fn media(&self) -> MediaFile { self.media.clone() }

    /// Registers what happens when the user asks to expand this player --
    /// double-click (handled by MediaFrame, an ancestor once
    /// video_widget.rs/embed_widget.rs wrap this) and the fullscreen
    /// button in this file's own control bar both end up here, so there
    /// is exactly one implementation of "what expanding means" regardless
    /// of which affordance triggered it.
    pub fn connect_expand(&self, f: impl Fn() + 'static) {
        *self.on_expand.borrow_mut() = Some(Box::new(f));
    }
}

fn wire_click_to_toggle(picture: &Picture, media: &MediaFile) {
    let click = gtk::GestureClick::new();
    let media = media.clone();
    click.connect_pressed(move |_, n_press, _, _| {
        // Only a single click toggles playback -- the second click of a
        // double-click opens the fullscreen viewer via MediaFrame, and
        // toggling play twice on the way there would just cancel out.
        if n_press != 1 { return; }
        if media.is_playing() { media.pause(); } else { media.play(); }
    });
    picture.add_controller(click);
}

fn wire_auto_hide(root: &Overlay, bar: &GtkBox) {
    let generation: Rc<Cell<u64>> = Rc::new(Cell::new(0));

    let show = {
        let bar_weak = bar.downgrade();
        let generation = generation.clone();
        move || {
            let Some(b) = bar_weak.upgrade() else { return };
            b.set_visible(true);
            let my_gen = generation.get().wrapping_add(1);
            generation.set(my_gen);

            let bar_weak2 = bar_weak.clone();
            let generation2 = generation.clone();
            glib::timeout_add_local(HIDE_AFTER, move || {
                if generation2.get() != my_gen { return glib::ControlFlow::Break; }
                if let Some(b) = bar_weak2.upgrade() { b.set_visible(false); }
                glib::ControlFlow::Break
            });
        }
    };

    let motion = EventControllerMotion::new();
    { let show = show.clone(); motion.connect_enter(move |_, _, _| show()); }
    { let show = show.clone(); motion.connect_motion(move |_, _, _| show()); }
    root.add_controller(motion);

    show();
}

fn build_control_bar(media: &MediaFile, loop_enabled: &Rc<Cell<bool>>, on_expand: &ExpandCallback) -> GtkBox {
    let bar = GtkBox::builder().orientation(Orientation::Horizontal).spacing(6).build();
    bar.add_css_class("video-player-bar");

    let play_btn = Button::with_label("⏵");
    play_btn.add_css_class("video-player-btn");
    {
        let btn_weak = play_btn.downgrade();
        media.connect_playing_notify(move |m| {
            if let Some(btn) = btn_weak.upgrade() {
                btn.set_label(if m.is_playing() { "⏸" } else { "⏵" });
            }
        });
        let media = media.clone();
        play_btn.connect_clicked(move |_| {
            if media.is_playing() { media.pause(); } else { media.play(); }
        });
    }
    bar.append(&play_btn);

    let time_label = Label::new(Some("0:00"));
    time_label.add_css_class("video-player-time");
    bar.append(&time_label);

    let seek = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.001);
    seek.set_hexpand(true);
    seek.set_draw_value(false);
    seek.add_css_class("video-player-seek");
    {
        let media = media.clone();
        seek.connect_change_value(move |_, _, value| {
            let dur = media.duration();
            if dur > 0 { media.seek((value.clamp(0.0, 1.0) * dur as f64) as i64); }
            glib::Propagation::Stop
        });
    }
    bar.append(&seek);

    let loop_btn = Button::with_label("⟲");
    loop_btn.add_css_class("video-player-btn");
    loop_btn.set_tooltip_text(Some("Loop"));
    if loop_enabled.get() { loop_btn.add_css_class("video-player-btn-on"); }
    {
        let media = media.clone();
        let loop_enabled = loop_enabled.clone();
        loop_btn.connect_clicked(move |b| {
            let on = !loop_enabled.get();
            loop_enabled.set(on);
            media.set_loop(on);
            if on { b.add_css_class("video-player-btn-on"); } else { b.remove_css_class("video-player-btn-on"); }
        });
    }
    bar.append(&loop_btn);

    let expand_btn = Button::with_label("⤢");
    expand_btn.add_css_class("video-player-btn");
    expand_btn.set_tooltip_text(Some("Expand"));
    {
        let on_expand = on_expand.clone();
        expand_btn.connect_clicked(move |_| {
            if let Some(f) = on_expand.borrow().as_ref() { f(); }
        });
    }
    bar.append(&expand_btn);

    wire_position_poll(media, &seek, &time_label, loop_enabled);

    bar
}

fn wire_position_poll(media: &MediaFile, seek: &Scale, time_label: &Label, loop_enabled: &Rc<Cell<bool>>) {
    let media_weak = media.downgrade();
    let seek_weak = seek.downgrade();
    let label_weak = time_label.downgrade();
    let loop_enabled = loop_enabled.clone();

    glib::timeout_add_local(POLL_INTERVAL, move || {
        let (Some(m), Some(s), Some(l)) = (media_weak.upgrade(), seek_weak.upgrade(), label_weak.upgrade())
        else { return glib::ControlFlow::Break; };

        let dur = m.duration();
        let pos = m.timestamp();

        // Looping lives here rather than on an `ended` signal -- see
        // module doc. Restarting moves position away from the end zone
        // immediately, so this naturally fires once per genuine end
        // rather than needing its own "already handled" flag.
        if dur > 0 && !m.is_playing() && pos >= dur - EOS_TOLERANCE_US && loop_enabled.get() {
            m.seek(0);
            m.play();
        }

        if dur > 0 { s.set_value((pos as f64 / dur as f64).clamp(0.0, 1.0)); }
        l.set_label(&format_timestamp(pos));

        glib::ControlFlow::Continue
    });
}

fn format_timestamp(micros: i64) -> String {
    let secs = (micros.max(0) / 1_000_000) as u64;
    format!("{}:{:02}", secs / 60, secs % 60)
}

pub const VIDEO_PLAYER_CSS: &str = r#"
.video-player {
    border: 1px solid rgba(255,255,255,0.09);
    border-radius: 6px;
}
.video-player-bar {
    background: linear-gradient(to top, rgba(0,0,0,0.72), rgba(0,0,0,0.0));
    padding: 14px 10px 8px 10px;
}
.video-player-btn {
    background: transparent;
    border: none;
    box-shadow: none;
    color: #e4ebf0;
    font-size: 11pt;
    min-height: 0;
    min-width: 0;
    padding: 2px 6px;
}
.video-player-btn:hover { color: #ffffff; }
.video-player-btn-on { color: #61afef; }
.video-player-time {
    color: #c4cdd4;
    font-family: monospace;
    font-size: 8pt;
}
.video-player-seek trough { background: rgba(255,255,255,0.22); }
.video-player-seek highlight { background: #61afef; }
.video-player-seek slider { background: #e4ebf0; border-radius: 50%; }
"#;
