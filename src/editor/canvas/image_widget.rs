// ImageWidget — inline image displayed as a child anchor in the TextView.
//
// Resize: a 14×14 DrawingArea grip at the bottom-right for drag-resize.
// Plus/minus buttons for precise width and height control.
// Default display: natural image size (no artificial cap — images load at full
// resolution so they are immediately useful without manual resizing).

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

use gtk::{
    gdk, gdk_pixbuf,
    prelude::*,
    Align, Box, Button, DrawingArea, GestureDrag, Label, Orientation,
};

const SIZE_STEP_W: i32 = 80;
const SIZE_STEP_H: i32 = 60;
const MIN_W:       i32 = 120;
const MIN_H:       i32 = 80;

pub struct ImageWidget {
    pub root: Box,
}

impl ImageWidget {
    pub fn new(path: &Path) -> Self {
        let pixbuf = gdk_pixbuf::Pixbuf::from_file(path).ok();

        // Show at natural size — no artificial scale-down.
        // Very large images (> 1400 px wide) are gently capped so the canvas
        // stays usable; the user can still drag-resize beyond that.
        let (init_w, init_h) = pixbuf.as_ref()
            .map(|p| {
                let w = p.width();
                let h = p.height();
                if w <= 1400 {
                    (w, h)
                } else {
                    let s = 1400.0 / w as f64;
                    (1400, (h as f64 * s).round() as i32)
                }
            })
            .unwrap_or((800, 600));

        let picture = gtk::Picture::new();
        if let Some(pb) = &pixbuf {
            let texture = gdk::Texture::for_pixbuf(pb);
            picture.set_paintable(Some(&texture));
        }
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_can_shrink(true);
        picture.set_hexpand(true);
        picture.set_vexpand(true);

        let wrapper = Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(6)
            .margin_bottom(2)
            .build();
        wrapper.add_css_class("image-widget");
        wrapper.set_size_request(init_w, init_h);
        wrapper.append(&picture);

        // ── size control row: W− / W+ / H− / H+ / grip ───────────────────────
        let ctrl_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(3)
            .margin_top(2)
            .build();

        let spacer = Label::new(None);
        spacer.set_hexpand(true);
        ctrl_row.append(&spacer);

        // helper: small styled button
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
            let wr = wrapper.clone();
            w_minus.connect_clicked(move |_| {
                let w = wr.allocated_width().max(wr.width_request());
                let h = wr.allocated_height().max(wr.height_request());
                wr.set_size_request((w - SIZE_STEP_W).max(MIN_W), h);
            });
        }
        {
            let wr = wrapper.clone();
            w_plus.connect_clicked(move |_| {
                let w = wr.allocated_width().max(wr.width_request());
                let h = wr.allocated_height().max(wr.height_request());
                wr.set_size_request(w + SIZE_STEP_W, h);
            });
        }
        {
            let wr = wrapper.clone();
            h_minus.connect_clicked(move |_| {
                let w = wr.allocated_width().max(wr.width_request());
                let h = wr.allocated_height().max(wr.height_request());
                wr.set_size_request(w, (h - SIZE_STEP_H).max(MIN_H));
            });
        }
        {
            let wr = wrapper.clone();
            h_plus.connect_clicked(move |_| {
                let w = wr.allocated_width().max(wr.width_request());
                let h = wr.allocated_height().max(wr.height_request());
                wr.set_size_request(w, h + SIZE_STEP_H);
            });
        }

        ctrl_row.append(&w_minus);
        ctrl_row.append(&w_plus);

        let sep = Label::builder().label(" ").build();
        ctrl_row.append(&sep);

        ctrl_row.append(&h_minus);
        ctrl_row.append(&h_plus);

        let grip = make_resize_grip();
        ctrl_row.append(&grip);

        attach_drag_resize(&grip, &wrapper);
        wrapper.append(&ctrl_row);

        Self { root: wrapper }
    }

    pub fn widget(&self) -> &Box { &self.root }
}


// ── resize grip DrawingArea ───────────────────────────────────────────────────
// 14×14 px corner triangle — small enough to not dominate the layout.
pub fn make_resize_grip() -> DrawingArea {
    let da = DrawingArea::builder()
        .width_request(14)
        .height_request(14)
        .halign(Align::End)
        .margin_start(4)
        .build();
    da.set_cursor_from_name(Some("se-resize"));

    let hovered: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let h_enter  = hovered.clone();
    let h_leave  = hovered.clone();
    let da_enter = da.downgrade();
    let da_leave = da.downgrade();

    let motion = gtk::EventControllerMotion::new();
    motion.connect_enter(move |_, _, _| {
        h_enter.set(true);
        if let Some(d) = da_enter.upgrade() { d.queue_draw(); }
    });
    motion.connect_leave(move |_| {
        h_leave.set(false);
        if let Some(d) = da_leave.upgrade() { d.queue_draw(); }
    });
    da.add_controller(motion);

    let h_draw = hovered.clone();
    da.set_draw_func(move |_, cr, w, h| {
        let wf = w as f64;
        let hf = h as f64;
        let alpha = if h_draw.get() { 0.75 } else { 0.38 };

        cr.set_source_rgba(0.72, 0.84, 0.96, alpha);
        cr.move_to(wf, 0.0);
        cr.line_to(wf, hf);
        cr.line_to(0.0, hf);
        cr.close_path();
        let _ = cr.fill();

        cr.set_source_rgba(1.0, 1.0, 1.0, alpha + 0.10);
        let dots: &[(f64, f64)] = &[
            (wf - 2.5, hf - 2.5),
            (wf - 6.0, hf - 2.5),
            (wf - 2.5, hf - 6.0),
        ];
        for &(x, y) in dots {
            if x > 0.0 && y > 0.0 {
                cr.arc(x, y, 1.1, 0.0, std::f64::consts::TAU);
                let _ = cr.fill();
            }
        }
    });

    da
}


// ── drag-resize controller ────────────────────────────────────────────────────
pub fn attach_drag_resize(handle: &DrawingArea, target: &Box) {
    let drag = GestureDrag::new();
    drag.set_propagation_phase(gtk::PropagationPhase::Capture);

    let start: Rc<Cell<(i32, i32)>> = Rc::new(Cell::new((0, 0)));
    let start_a = start.clone();
    let start_b = start;
    let target_a = target.clone();
    let target_b = target.clone();

    drag.connect_drag_begin(move |_, _, _| {
        start_a.set((target_a.allocated_width(), target_a.allocated_height()));
    });

    drag.connect_drag_update(move |_, dx, dy| {
        let (sw, sh) = start_b.get();
        target_b.set_size_request(
            (sw + dx as i32).max(MIN_W),
            (sh + dy as i32).max(MIN_H),
        );
    });

    handle.add_controller(drag);
}


// ── CSS ───────────────────────────────────────────────────────────────────────
pub const IMAGE_CSS: &str = r#"
.image-size-btn {
    background: rgba(255,255,255,0.07);
    border: 1px solid rgba(255,255,255,0.14);
    border-radius: 4px;
    box-shadow: none;
    color: #8a9baa;
    font-size: 7.5pt;
    font-weight: 600;
    min-height: 0;
    min-width: 0;
    padding: 1px 5px;
}
.image-size-btn:hover {
    background: rgba(255,255,255,0.14);
    color: #c4d0da;
}
"#;
