// ImageWidget — inline image displayed as a child anchor in the TextView.
//
// Resize grip: a 22×22 DrawingArea at the bottom-right acts as the drag handle.
// Using DrawingArea (not a text Label) gives a proper hit area and lets us use
// PropagationPhase::Capture so the gesture takes priority over text selection.
// The drawn grip shows a corner triangle + dot pattern; it brightens on hover.

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

use gtk::{
    gdk, gdk_pixbuf,
    prelude::*,
    Align, Box, DrawingArea, GestureDrag, Orientation,
};

const MAX_INIT_WIDTH: i32 = 600;

pub struct ImageWidget {
    pub root: Box,
}

impl ImageWidget {
    pub fn new(path: &Path) -> Self {
        let pixbuf = gdk_pixbuf::Pixbuf::from_file(path).ok();

        let (init_w, init_h) = pixbuf.as_ref()
            .map(|p| {
                let w = p.width();
                let h = p.height();
                if w <= MAX_INIT_WIDTH {
                    (w, h)
                } else {
                    let s = MAX_INIT_WIDTH as f64 / w as f64;
                    (MAX_INIT_WIDTH, (h as f64 * s).round() as i32)
                }
            })
            .unwrap_or((400, 300));

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

        let grip_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .build();
        let spacer = gtk::Label::new(None);
        spacer.set_hexpand(true);

        let grip = make_resize_grip();
        grip_row.append(&spacer);
        grip_row.append(&grip);
        wrapper.append(&grip_row);

        attach_drag_resize(&grip, &wrapper);

        Self { root: wrapper }
    }

    pub fn widget(&self) -> &Box { &self.root }
}


// ── resize grip DrawingArea ───────────────────────────────────────────────────
// 22×22 px corner triangle with a dot-grid pattern.
// Brightens on hover for clear visual feedback.
pub fn make_resize_grip() -> DrawingArea {
    let da = DrawingArea::builder()
        .width_request(22)
        .height_request(22)
        .halign(Align::End)
        .build();
    da.set_cursor_from_name(Some("se-resize"));

    let hovered: Rc<Cell<bool>> = Rc::new(Cell::new(false));

    let h_enter = hovered.clone();
    let h_leave = hovered.clone();
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
        let alpha = if h_draw.get() { 0.68 } else { 0.30 };

        // Corner triangle fill
        cr.set_source_rgba(0.72, 0.84, 0.96, alpha);
        cr.move_to(wf, 0.0);
        cr.line_to(wf, hf);
        cr.line_to(0.0, hf);
        cr.close_path();
        let _ = cr.fill();

        // Dot-grid pattern (lower-right corner, 3 rows)
        cr.set_source_rgba(1.0, 1.0, 1.0, alpha + 0.08);
        let dots: &[(f64, f64)] = &[
            (wf - 3.5, hf - 3.5),
            (wf - 8.5, hf - 3.5),
            (wf - 3.5, hf - 8.5),
            (wf - 13.5, hf - 3.5),
            (wf - 8.5, hf - 8.5),
            (wf - 3.5, hf - 13.5),
        ];
        for &(x, y) in dots {
            if x > 0.0 && y > 0.0 {
                cr.arc(x, y, 1.3, 0.0, std::f64::consts::TAU);
                let _ = cr.fill();
            }
        }
    });

    da
}


// ── drag-resize controller ────────────────────────────────────────────────────
// PropagationPhase::Capture ensures this gesture wins over the TextView's
// built-in text-selection gesture, so fast drags no longer highlight lines.
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
            (sw + dx as i32).max(120),
            (sh + dy as i32).max(80),
        );
    });

    handle.add_controller(drag);
}
