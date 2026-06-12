// Inline local video widget backed by gtk::Video (GStreamer).
//
// Inserted as a child anchor in the TextView — marker format: \x02video:<filename>\x02
//
// Resize: same ◢ drag grip used by ImageWidget.  Dragging changes the Video
// widget's size_request; GStreamer's pipeline scales the decoded frames to fit.

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

use gtk::{prelude::*, Align, Box, GestureDrag, Label, Orientation, Video};

const VIDEO_WIDTH:  i32 = 620;
const VIDEO_HEIGHT: i32 = 360;

pub struct VideoWidget {
    pub root: Box,
}

impl VideoWidget {
    pub fn new(path: &Path) -> Self {
        let video = Video::for_filename(Some(path));
        video.set_size_request(VIDEO_WIDTH, VIDEO_HEIGHT);
        video.set_autoplay(false);
        video.set_loop(false);

        let wrapper = Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(6)
            .margin_bottom(2)
            .build();
        wrapper.add_css_class("video-widget");
        wrapper.append(&video);

        // ── resize grip ───────────────────────────────────────────────────
        let grip_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .build();
        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        let grip = Label::builder()
            .label("◢")
            .halign(Align::End)
            .build();
        grip.add_css_class("resize-grip");
        grip.set_cursor_from_name(Some("se-resize"));
        grip_row.append(&spacer);
        grip_row.append(&grip);
        wrapper.append(&grip_row);

        attach_video_resize(&grip, &video);

        Self { root: wrapper }
    }

    pub fn widget(&self) -> &Box {
        &self.root
    }
}

fn attach_video_resize(handle: &Label, video: &Video) {
    let drag = GestureDrag::new();

    let start: Rc<Cell<(i32, i32)>> = Rc::new(Cell::new((VIDEO_WIDTH, VIDEO_HEIGHT)));
    let start_a = start.clone();
    let start_b = start;
    let vid_a   = video.clone();
    let vid_b   = video.clone();

    drag.connect_drag_begin(move |_, _, _| {
        start_a.set((vid_a.allocated_width(), vid_a.allocated_height()));
    });

    drag.connect_drag_update(move |_, dx, dy| {
        let (sw, sh) = start_b.get();
        vid_b.set_size_request(
            (sw + dx as i32).max(200),
            (sh + dy as i32).max(120),
        );
    });

    handle.add_controller(drag);
}

pub const VIDEO_CSS: &str = r#"
.video-widget {
    border: 1px solid rgba(255,255,255,0.09);
    border-radius: 6px;
}
"#;
