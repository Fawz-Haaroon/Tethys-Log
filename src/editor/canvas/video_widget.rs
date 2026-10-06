// VideoWidget — inline local video displayed as a child anchor in the
// TextView. Playback and controls live in video_player.rs, shared with
// embed_widget.rs's downloaded-video case; this file is just that player
// dropped into a MediaFrame with video-appropriate minimums and a default
// size for the (rare, pre-persistence) case where a note has no recovered
// size to restore.
//
// Expanding to the fullscreen viewer -- from MediaFrame's double-click or
// VideoPlayer's own expand button, both wired to the same closure below
// -- detaches the player's widget from this frame and hands it to the
// viewer; the viewer hands it back on close. There is one VideoPlayer for
// the lifetime of this widget, full stop, whether or not the viewer has
// ever been opened.

use std::path::Path;

use gtk::{prelude::*, Overlay};

use crate::editor::canvas::{media_frame::MediaFrame, video_player::VideoPlayer, viewer};

const MIN_W: i32 = 200;
const MIN_H: i32 = 120;
const DEFAULT_W: i32 = 800;
const DEFAULT_H: i32 = 450;

pub struct VideoWidget {
    frame: MediaFrame,
    // Kept only so the player -- and the GStreamer pipeline underneath it
    // -- stays alive for exactly as long as this widget does. Nothing
    // reads it after construction.
    _player: VideoPlayer,
}

impl VideoWidget {
    pub fn new(path: &Path, initial_size: Option<(i32, i32)>) -> Self {
        let player = VideoPlayer::new(path);
        let (init_w, init_h) = initial_size.unwrap_or((DEFAULT_W, DEFAULT_H));

        let frame = MediaFrame::new(player.widget(), init_w, init_h, MIN_W, MIN_H);
        frame.widget().add_css_class("video-widget");

        let expand = build_expand_handler(&frame);
        frame.connect_expand({
            let expand = expand.clone();
            move || expand()
        });
        player.connect_expand(move || expand());

        Self { frame, _player: player }
    }

    pub fn widget(&self) -> &Overlay { self.frame.widget() }
}

/// Builds the "expand this video" closure once, shared by both trigger
/// paths (MediaFrame's double-click/Enter and VideoPlayer's own expand
/// button). Detaches the frame's current content, hands it to the viewer,
/// and puts it back exactly where it came from on close -- see viewer.rs
/// for why this is a reparent rather than a second player.
fn build_expand_handler(frame: &MediaFrame) -> std::rc::Rc<dyn Fn()> {
    let frame_weak = frame.widget().downgrade();
    std::rc::Rc::new(move || {
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
    })
}

pub const VIDEO_CSS: &str = r#"
.video-widget {
    margin-top: 6px;
    margin-bottom: 2px;
}
"#;
