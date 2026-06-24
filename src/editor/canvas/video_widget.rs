// Inline local video widget backed by gtk::Video (GStreamer).
//
// Inserted as a child anchor in the TextView — marker format: \x02video:<filename>\x02
//
// Controls row below the player:
//   [Loop: OFF]  [«10s] [«5s] [5s»] [10s»]   <spacer>  [W−] [W+] [H−] [H+]  [◢ grip]
//
// Loop toggles video.set_loop().
// Skip buttons seek the underlying MediaStream ±5 s or ±10 s.
// Size buttons step width/height by fixed increments.
// The ◢ grip is a 14×14 DrawingArea — drag-resize via GestureDrag at Capture
// phase so it wins over text selection.

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

use gtk::{prelude::*, Box, Button, GestureDrag, Label, Orientation, Video};

use crate::editor::canvas::image_widget::make_resize_grip;

const VIDEO_WIDTH:  i32 = 800;
const VIDEO_HEIGHT: i32 = 450;
const SIZE_STEP_W:  i32 = 80;
const SIZE_STEP_H:  i32 = 60;
const MIN_W:        i32 = 200;
const MIN_H:        i32 = 120;

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

        // ── control row ───────────────────────────────────────────────────────
        let ctrl_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(3)
            .margin_top(2)
            .build();

        // ── Loop toggle ───────────────────────────────────────────────────────
        let loop_btn = Button::builder().label("Loop: OFF").build();
        loop_btn.add_css_class("video-ctrl-btn");
        loop_btn.set_cursor_from_name(Some("pointer"));
        {
            let vid      = video.clone();
            let is_loop: Rc<Cell<bool>> = Rc::new(Cell::new(false));
            loop_btn.connect_clicked(move |b| {
                let next = !is_loop.get();
                is_loop.set(next);
                vid.set_loop(next);
                // Also propagate to the MediaStream directly.  gtk::Video::set_loop
                // may not reach a stream that was not yet ready when the Video was
                // first constructed; calling it on the stream guarantees effect.
                if let Some(stream) = vid.media_stream() {
                    stream.set_loop(next);
                }
                b.set_label(if next { "Loop: ON" } else { "Loop: OFF" });
            });
        }

        // ── Skip buttons (seek ±5 s / ±10 s via MediaStream) ─────────────────
        // Helper: create one skip button.
        // delta_us: microseconds to add to current position (negative = backward).
        fn make_skip(label: &str, delta_us: i64, video: &Video) -> Button {
            let b = Button::builder().label(label).build();
            b.add_css_class("video-ctrl-btn");
            b.set_cursor_from_name(Some("pointer"));
            let v = video.clone();
            b.connect_clicked(move |_| {
                if let Some(stream) = v.media_stream() {
                    let pos     = stream.timestamp();
                    let new_pos = (pos + delta_us).max(0);
                    stream.seek(new_pos);
                }
            });
            b
        }

        let skip_back10 = make_skip("«10s", -10_000_000, &video);
        let skip_back5  = make_skip("«5s",   -5_000_000, &video);
        let skip_fwd5   = make_skip("5s»",    5_000_000, &video);
        let skip_fwd10  = make_skip("10s»",  10_000_000, &video);

        // ── Size step buttons ─────────────────────────────────────────────────
        let make_btn = |label: &str| -> Button {
            let b = Button::builder().label(label).build();
            b.add_css_class("image-size-btn");
            b.set_cursor_from_name(Some("pointer"));
            b
        };

        let w_minus = make_btn("W−");
        let w_plus  = make_btn("W+");
        let h_minus = make_btn("H−");
        let h_plus  = make_btn("H+");

        {
            let vid = video.clone();
            w_minus.connect_clicked(move |_| {
                let w = vid.allocated_width().max(vid.width_request());
                let h = vid.allocated_height().max(vid.height_request());
                vid.set_size_request((w - SIZE_STEP_W).max(MIN_W), h);
            });
        }
        {
            let vid = video.clone();
            w_plus.connect_clicked(move |_| {
                let w = vid.allocated_width().max(vid.width_request());
                let h = vid.allocated_height().max(vid.height_request());
                vid.set_size_request(w + SIZE_STEP_W, h);
            });
        }
        {
            let vid = video.clone();
            h_minus.connect_clicked(move |_| {
                let w = vid.allocated_width().max(vid.width_request());
                let h = vid.allocated_height().max(vid.height_request());
                vid.set_size_request(w, (h - SIZE_STEP_H).max(MIN_H));
            });
        }
        {
            let vid = video.clone();
            h_plus.connect_clicked(move |_| {
                let w = vid.allocated_width().max(vid.width_request());
                let h = vid.allocated_height().max(vid.height_request());
                vid.set_size_request(w, h + SIZE_STEP_H);
            });
        }

        let grip = make_resize_grip();
        attach_drag_resize_video(&grip, &video);

        // ── Assemble control row ──────────────────────────────────────────────
        // [Loop]  [«10s] [«5s] [5s»] [10s»]  <spacer>  [W-] [W+]  [H-] [H+]  [grip]
        ctrl_row.append(&loop_btn);

        let sep1 = Label::builder().label("  ").build();
        ctrl_row.append(&sep1);
        ctrl_row.append(&skip_back10);
        ctrl_row.append(&skip_back5);
        ctrl_row.append(&skip_fwd5);
        ctrl_row.append(&skip_fwd10);

        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        ctrl_row.append(&spacer);

        ctrl_row.append(&w_minus);
        ctrl_row.append(&w_plus);

        let sep2 = Label::builder().label(" ").build();
        ctrl_row.append(&sep2);

        ctrl_row.append(&h_minus);
        ctrl_row.append(&h_plus);
        ctrl_row.append(&grip);

        wrapper.append(&ctrl_row);

        Self { root: wrapper }
    }

    pub fn widget(&self) -> &Box { &self.root }
}


fn attach_drag_resize_video(handle: &gtk::DrawingArea, video: &Video) {
    let drag = GestureDrag::new();
    drag.set_propagation_phase(gtk::PropagationPhase::Capture);

    let start: Rc<Cell<(i32, i32)>> = Rc::new(Cell::new((VIDEO_WIDTH, VIDEO_HEIGHT)));
    let sa = start.clone();
    let sb = start;
    let va = video.clone();
    let vb = video.clone();

    drag.connect_drag_begin(move |_, _, _| {
        sa.set((va.allocated_width(), va.allocated_height()));
    });
    drag.connect_drag_update(move |_, dx, dy| {
        let (sw, sh) = sb.get();
        vb.set_size_request((sw + dx as i32).max(MIN_W), (sh + dy as i32).max(MIN_H));
    });
    handle.add_controller(drag);
}


pub const VIDEO_CSS: &str = r#"
.video-widget {
    border: 1px solid rgba(255,255,255,0.09);
    border-radius: 6px;
}
.video-ctrl-btn {
    background: rgba(255,255,255,0.07);
    border: 1px solid rgba(255,255,255,0.14);
    border-radius: 4px;
    box-shadow: none;
    color: #8a9baa;
    font-size: 7.5pt;
    font-weight: 600;
    min-height: 0;
    min-width: 0;
    padding: 1px 8px;
}
.video-ctrl-btn:hover {
    background: rgba(255,255,255,0.14);
    color: #c4d0da;
}
"#;
