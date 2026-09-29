// MediaFrame — the selection and resize layer shared by every inline media
// object: images, local video, and an embed's video once it's playing.
// Wraps one child widget in an Overlay, adds a focus-driven selection
// boundary, and puts eight resize zones around it -- four corners plus
// four edges, so the whole boundary is a resize target rather than four
// small squares the pointer has to land on precisely. Corners change both
// dimensions; edges change one. Drives continuous 1:1 drag-resize through
// set_size_request on the frame itself -- the frame IS the sized widget,
// callers read its current_size() back for persistence rather than
// tracking a size separately, and codec.rs reads the same value at
// serialize time the same way.
//
// root is pinned to halign/valign = Start. GtkTextView offers a
// child-anchor widget the full width of its line to lay out within, and
// Overlay's default alignment is Fill -- without pinning this, root
// stretches to that full offered width regardless of size_request, which
// is what produced a media object with a tiny video floating in a huge
// empty frame. Start means root is allocated exactly its requested size
// and positioned within any extra space rather than expanding to
// consume it.
//
// Selection is GTK's own focus, not a hand-rolled flag. GTK already
// guarantees one focus widget per window, so "select this, deselect
// whatever else was selected", "Tab reaches it", and "arrow keys go to the
// focused object" all come for free instead of a shared
// Rc<Cell<Option<Id>>> coordinated across every media instance in the note.
//
// A compact width/height entry bar sits alongside the resize zones, shown
// and hidden by the same hover/focus state. Drag, the arrow-key nudge,
// the +/- buttons, and the entry fields all funnel through the one same
// place -- root.set_size_request -- so there's a single dimension state
// rather than drag state and numeric-entry state disagreeing with each
// other. The entries display whatever frame_size() reads back after
// every change, never a value tracked separately that could drift.
//
// Entry fields are free-form -- typing a width does not also move the
// height, unlike the +/- buttons and drag, which preserve aspect ratio by
// default. That split is deliberate: the entries are the precision tool
// for an exact, possibly non-native width and height together, and
// aspect-locking them would fight a user typing both fields in sequence.
//
// The boundary itself is native GTK CSS state (:hover / :focus) on the
// frame -- no Rust-side class toggling needed for that part. The eight
// zone widgets and the dimension bar are a different matter: CSS can't
// make a sibling widget appear, so their visibility is still driven from
// Rust, off the same hover/focus signals -- and while hidden the zones
// are also not hit-testable, which is what keeps them out of the way of
// ordinary text selection at the media's boundary when it isn't hovered
// or focused.
//
// All eight zones use GestureDrag at Capture phase, same technique the
// old bottom-right grip already used and for the same reason -- winning
// against the TextView's own drag-to-select before it gets a look at the
// event. A plain click to select doesn't need Capture: a child widget
// already gets first claim on anything landing in its own allocation.
//
// Expanding to the fullscreen viewer is a two-phase registration
// (connect_expand, called after construction) rather than a constructor
// argument, because the closure a caller registers here needs a weak
// reference back to this frame's own widget -- to detach and later
// reattach its content via that widget's own child()/set_child() -- which
// can't be captured by a closure built before the frame it refers to
// exists.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::{
    gdk, prelude::*, Align, Box as GtkBox, Button, DrawingArea, Entry, EventControllerFocus,
    EventControllerKey, EventControllerMotion, GestureClick, GestureDrag, Label, Orientation,
    Overlay,
};

const CORNER_HIT: i32 = 18;
const CORNER_VISIBLE: i32 = 8;
const EDGE_THICKNESS: i32 = 14;
const EDGE_INSET: i32 = 16; // keeps edge strips clear of the corner zones
const KEY_STEP: i32 = 20;
const KEY_STEP_BIG: i32 = 100;
const DIM_STEP: i32 = 20;

type ExpandCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

pub struct MediaFrame {
    pub root: Overlay,
    on_expand: ExpandCallback,
}

#[derive(Clone, Copy, PartialEq)]
enum Zone { TopLeft, TopRight, BottomLeft, BottomRight, Left, Right, Top, Bottom }

const ALL_ZONES: [Zone; 8] = [
    Zone::TopLeft, Zone::TopRight, Zone::BottomLeft, Zone::BottomRight,
    Zone::Left, Zone::Right, Zone::Top, Zone::Bottom,
];

impl Zone {
    fn is_corner(self) -> bool {
        matches!(self, Zone::TopLeft | Zone::TopRight | Zone::BottomLeft | Zone::BottomRight)
    }

    fn cursor_name(self) -> &'static str {
        match self {
            Zone::TopLeft     => "nw-resize",
            Zone::TopRight    => "ne-resize",
            Zone::BottomLeft  => "sw-resize",
            Zone::BottomRight => "se-resize",
            Zone::Left | Zone::Right => "ew-resize",
            Zone::Top  | Zone::Bottom => "ns-resize",
        }
    }

    fn align(self) -> (Align, Align) {
        match self {
            Zone::TopLeft     => (Align::Start, Align::Start),
            Zone::TopRight    => (Align::End,   Align::Start),
            Zone::BottomLeft  => (Align::Start, Align::End),
            Zone::BottomRight => (Align::End,   Align::End),
            Zone::Left        => (Align::Start, Align::Fill),
            Zone::Right       => (Align::End,   Align::Fill),
            Zone::Top         => (Align::Fill,  Align::Start),
            Zone::Bottom      => (Align::Fill,  Align::End),
        }
    }

    fn signs(self) -> (f64, f64) {
        match self {
            Zone::TopLeft     => (-1.0, -1.0),
            Zone::TopRight    => ( 1.0, -1.0),
            Zone::BottomLeft  => (-1.0,  1.0),
            Zone::BottomRight => ( 1.0,  1.0),
            Zone::Left        => (-1.0,  0.0),
            Zone::Right       => ( 1.0,  0.0),
            Zone::Top         => ( 0.0, -1.0),
            Zone::Bottom      => ( 0.0,  1.0),
        }
    }

    fn drives(self) -> (bool, bool) {
        match self {
            Zone::TopLeft | Zone::TopRight | Zone::BottomLeft | Zone::BottomRight => (true, true),
            Zone::Left | Zone::Right => (true, false),
            Zone::Top  | Zone::Bottom => (false, true),
        }
    }
}

impl MediaFrame {
    /// `child` is the actual visual content (a Picture, a video player's
    /// root, anything). `init_w`/`init_h` seed both the starting size and
    /// the aspect ratio drags preserve by default. `min_w`/`min_h` are a
    /// floor a drag, keyboard nudge, or numeric entry won't shrink past --
    /// callers pass their own, since a photo and a video have different
    /// sensible minimums.
    pub fn new(child: &impl IsA<gtk::Widget>, init_w: i32, init_h: i32, min_w: i32, min_h: i32) -> Self {
        let min_w = min_w.max(1);
        let min_h = min_h.max(1);
        let aspect = if init_h > 0 { init_w as f64 / init_h as f64 } else { 1.0 };

        child.set_hexpand(true);
        child.set_vexpand(true);

        let root = Overlay::new();
        root.add_css_class("media-frame");
        root.set_focusable(true);
        root.set_halign(Align::Start);
        root.set_valign(Align::Start);
        root.set_hexpand(false);
        root.set_vexpand(false);
        root.set_size_request(init_w.max(min_w), init_h.max(min_h));
        root.set_child(Some(child));

        let hovered: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        let focused: Rc<Cell<bool>> = Rc::new(Cell::new(false));

        let zone_widgets: Vec<DrawingArea> =
            ALL_ZONES.iter().map(|&z| make_zone_widget(z)).collect();
        let (dim_bar, refresh_dims) = build_dimension_bar(&root, aspect, min_w, min_h);
        for (widget, &zone) in zone_widgets.iter().zip(ALL_ZONES.iter()) {
            root.add_overlay(widget);
            attach_zone_drag(widget, &root, zone, aspect, min_w, min_h, refresh_dims.clone());
        }
        dim_bar.set_visible(false);
        root.add_overlay(&dim_bar);

        let sync_visible = {
            let hovered = hovered.clone();
            let focused = focused.clone();
            let widgets = zone_widgets.clone();
            let dim_bar = dim_bar.clone();
            let refresh_dims = refresh_dims.clone();
            move || {
                let show = hovered.get() || focused.get();
                for w in &widgets {
                    if w.is_visible() != show { w.set_visible(show); }
                }
                if dim_bar.is_visible() != show { dim_bar.set_visible(show); }
                if show { refresh_dims(); }
            }
        };

        let motion = EventControllerMotion::new();
        {
            let hovered = hovered.clone();
            let sync = sync_visible.clone();
            motion.connect_enter(move |_, _, _| { hovered.set(true); sync(); });
        }
        {
            let hovered = hovered.clone();
            let sync = sync_visible.clone();
            motion.connect_leave(move |_| { hovered.set(false); sync(); });
        }
        root.add_controller(motion);

        let focus_ctrl = EventControllerFocus::new();
        {
            let focused = focused.clone();
            let sync = sync_visible.clone();
            focus_ctrl.connect_enter(move |_| { focused.set(true); sync(); });
        }
        {
            let focused = focused.clone();
            let sync = sync_visible;
            focus_ctrl.connect_leave(move |_| { focused.set(false); sync(); });
        }
        root.add_controller(focus_ctrl);

        let on_expand: ExpandCallback = Rc::new(RefCell::new(None));

        let click = GestureClick::new();
        {
            let root_weak = root.downgrade();
            let on_expand = on_expand.clone();
            click.connect_pressed(move |_, n_press, _, _| {
                if let Some(r) = root_weak.upgrade() {
                    r.grab_focus();
                    if n_press >= 2 {
                        if let Some(f) = on_expand.borrow().as_ref() { f(); }
                    }
                }
            });
        }
        root.add_controller(click);

        let keys = EventControllerKey::new();
        {
            let root_weak = root.downgrade();
            let on_expand = on_expand.clone();
            let refresh_dims = refresh_dims;
            keys.connect_key_pressed(move |_, key, _, mods| {
                let Some(r) = root_weak.upgrade() else {
                    return gtk::glib::Propagation::Proceed;
                };
                match key {
                    gdk::Key::Return | gdk::Key::KP_Enter => {
                        if let Some(f) = on_expand.borrow().as_ref() { f(); }
                        gtk::glib::Propagation::Stop
                    }
                    gdk::Key::Left | gdk::Key::Right | gdk::Key::Up | gdk::Key::Down => {
                        let big = mods.contains(gdk::ModifierType::SHIFT_MASK);
                        let step = if big { KEY_STEP_BIG } else { KEY_STEP };
                        let (w, _h) = frame_size(&r);
                        let grow = matches!(key, gdk::Key::Right | gdk::Key::Down);
                        let delta = if grow { step } else { -step };
                        let new_w = (w + delta).max(min_w);
                        let new_h = ((new_w as f64 / aspect).round() as i32).max(min_h);
                        r.set_size_request(new_w, new_h);
                        refresh_dims();
                        gtk::glib::Propagation::Stop
                    }
                    _ => gtk::glib::Propagation::Proceed,
                }
            });
        }
        root.add_controller(keys);

        Self { root, on_expand }
    }

    pub fn widget(&self) -> &Overlay { &self.root }

    pub fn current_size(&self) -> (i32, i32) { frame_size(&self.root) }

    /// Registers what happens on double-click or Enter-while-focused.
    /// video_widget.rs and embed_widget.rs call this once, after both
    /// this frame and the player inside it exist, with a closure that
    /// detaches this frame's content (via widget().child()/set_child(),
    /// since a weak reference can be taken on the Overlay but not on
    /// MediaFrame itself) and hands it to the viewer -- see viewer.rs for
    /// why that's a reparent rather than a second player.
    pub fn connect_expand(&self, f: impl Fn() + 'static) {
        *self.on_expand.borrow_mut() = Some(Box::new(f));
    }
}

fn frame_size(w: &impl IsA<gtk::Widget>) -> (i32, i32) {
    let (wr, hr) = (w.width_request(), w.height_request());
    let aw = w.allocated_width();
    let ah = w.allocated_height();
    (if wr > 0 { wr } else { aw }, if hr > 0 { hr } else { ah })
}

fn make_zone_widget(zone: Zone) -> DrawingArea {
    let (ha, va) = zone.align();
    let mut builder = DrawingArea::builder().halign(ha).valign(va).visible(false);

    builder = if zone.is_corner() {
        builder.width_request(CORNER_HIT).height_request(CORNER_HIT)
    } else if matches!(zone, Zone::Left | Zone::Right) {
        builder.width_request(EDGE_THICKNESS).vexpand(true)
            .margin_top(EDGE_INSET).margin_bottom(EDGE_INSET)
    } else {
        builder.height_request(EDGE_THICKNESS).hexpand(true)
            .margin_start(EDGE_INSET).margin_end(EDGE_INSET)
    };

    let da = builder.build();
    da.add_css_class("media-frame-zone");
    da.set_cursor_from_name(Some(zone.cursor_name()));

    if zone.is_corner() {
        da.set_draw_func(move |_, cr, w, h| {
            let ox = ((w - CORNER_VISIBLE) / 2).max(0) as f64;
            let oy = ((h - CORNER_VISIBLE) / 2).max(0) as f64;
            cr.set_source_rgba(0.55, 0.78, 1.0, 0.95);
            cr.rectangle(ox, oy, CORNER_VISIBLE as f64, CORNER_VISIBLE as f64);
            let _ = cr.fill();
            cr.set_source_rgba(0.06, 0.07, 0.09, 0.9);
            let inset = 1.0;
            cr.rectangle(ox + inset, oy + inset,
                (CORNER_VISIBLE as f64 - 2.0 * inset).max(0.0),
                (CORNER_VISIBLE as f64 - 2.0 * inset).max(0.0));
            let _ = cr.fill();
        });
    }

    da
}

#[allow(clippy::too_many_arguments)]
fn attach_zone_drag(
    handle: &DrawingArea, target: &Overlay, zone: Zone, aspect: f64,
    min_w: i32, min_h: i32, refresh: Rc<dyn Fn()>,
) {
    let drag = GestureDrag::new();
    drag.set_propagation_phase(gtk::PropagationPhase::Capture);

    let start: Rc<Cell<(i32, i32)>> = Rc::new(Cell::new((0, 0)));
    let start_begin = start.clone();
    let target_begin = target.downgrade();
    drag.connect_drag_begin(move |_, _, _| {
        if let Some(t) = target_begin.upgrade() { start_begin.set(frame_size(&t)); }
    });

    let target_update = target.downgrade();
    drag.connect_drag_update(move |gesture, dx, dy| {
        let Some(t) = target_update.upgrade() else { return };
        let (start_w, start_h) = start.get();
        let free = gesture.current_event_state().contains(gdk::ModifierType::SHIFT_MASK);
        let (new_w, new_h) = resolve_zone_drag(zone, start_w, start_h, dx, dy, aspect, min_w, min_h, free);
        t.set_size_request(new_w, new_h);
        refresh();
    });

    handle.add_controller(drag);
}

/// The actual resize math, factored out of the gesture wiring so it can be
/// reasoned about on its own. Every path through here ends in max(min_*)
/// before rounding to i32 -- a resize frame should never be able to hand
/// GTK a negative or zero request regardless of how the drag math above
/// it worked out.
#[allow(clippy::too_many_arguments)]
fn resolve_zone_drag(
    zone: Zone, start_w: i32, start_h: i32, dx: f64, dy: f64,
    aspect: f64, min_w: i32, min_h: i32, free: bool,
) -> (i32, i32) {
    let (sx, sy) = zone.signs();
    let (drives_w, drives_h) = zone.drives();
    let dw = dx * sx;
    let dh = dy * sy;
    let (min_w, min_h) = (min_w as f64, min_h as f64);

    let (w, h): (f64, f64) = if free {
        (
            if drives_w { (start_w as f64 + dw).max(min_w) } else { start_w as f64 },
            if drives_h { (start_h as f64 + dh).max(min_h) } else { start_h as f64 },
        )
    } else {
        match (drives_w, drives_h) {
            (true, true) => {
                if dw.abs() >= dh.abs() {
                    let w = (start_w as f64 + dw).max(min_w);
                    (w, (w / aspect).max(min_h))
                } else {
                    let h = (start_h as f64 + dh).max(min_h);
                    (h * aspect, h)
                }
            }
            (true, false) => {
                let w = (start_w as f64 + dw).max(min_w);
                (w, (w / aspect).max(min_h))
            }
            (false, true) => {
                let h = (start_h as f64 + dh).max(min_h);
                (h * aspect, h)
            }
            (false, false) => (start_w as f64, start_h as f64),
        }
    };

    (w.round().max(min_w) as i32, h.round().max(min_h) as i32)
}

/// Builds the compact width/height entry row and returns it along with a
/// closure that re-reads the frame's actual current size and writes it
/// into both entries -- the one function every size-changing path (drag,
/// keyboard, the +/- buttons, entry commit) calls afterward, so the
/// displayed numbers can't drift from root's real size_request.
fn build_dimension_bar(root: &Overlay, aspect: f64, min_w: i32, min_h: i32) -> (GtkBox, Rc<dyn Fn()>) {
    let bar = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .halign(Align::Start)
        .valign(Align::Start)
        .margin_top(4)
        .margin_start(4)
        .build();
    bar.add_css_class("media-frame-dim-bar");

    let w_label = Label::new(Some("W"));
    let w_minus = Button::with_label("−");
    let w_entry = Entry::new();
    let w_plus = Button::with_label("+");
    let h_label = Label::new(Some("H"));
    let h_minus = Button::with_label("−");
    let h_entry = Entry::new();
    let h_plus = Button::with_label("+");

    for l in [&w_label, &h_label] { l.add_css_class("media-frame-dim-label"); }
    for b in [&w_minus, &w_plus, &h_minus, &h_plus] { b.add_css_class("media-frame-dim-step"); }
    for e in [&w_entry, &h_entry] {
        e.set_width_chars(5);
        e.set_max_width_chars(5);
        e.add_css_class("media-frame-dim-entry");
    }

    bar.append(&w_label); bar.append(&w_minus); bar.append(&w_entry); bar.append(&w_plus);
    bar.append(&h_label); bar.append(&h_minus); bar.append(&h_entry); bar.append(&h_plus);

    let refresh: Rc<dyn Fn()> = {
        let root = root.downgrade();
        let w_entry = w_entry.downgrade();
        let h_entry = h_entry.downgrade();
        Rc::new(move || {
            let (Some(r), Some(we), Some(he)) = (root.upgrade(), w_entry.upgrade(), h_entry.upgrade()) else { return };
            let (w, h) = frame_size(&r);
            we.set_text(&w.to_string());
            he.set_text(&h.to_string());
        })
    };

    wire_dim_entry(&w_entry, root, min_w, true, refresh.clone());
    wire_dim_entry(&h_entry, root, min_h, false, refresh.clone());
    wire_dim_step(&w_minus, root, -DIM_STEP, true, aspect, min_w, min_h, refresh.clone());
    wire_dim_step(&w_plus, root, DIM_STEP, true, aspect, min_w, min_h, refresh.clone());
    wire_dim_step(&h_minus, root, -DIM_STEP, false, aspect, min_w, min_h, refresh.clone());
    wire_dim_step(&h_plus, root, DIM_STEP, false, aspect, min_w, min_h, refresh.clone());

    (bar, refresh)
}

/// Entries are free-form -- see module doc for why -- and commit only
/// their own dimension, on Enter or on losing focus. Anything that
/// doesn't parse as a positive integer is dropped silently; refresh()
/// afterward puts the real current value back on screen either way, so
/// there's no path where the field is left showing something that
/// doesn't match the document.
fn wire_dim_entry(entry: &Entry, root: &Overlay, min: i32, is_width: bool, refresh: Rc<dyn Fn()>) {
    let commit = {
        let root = root.downgrade();
        let entry_weak = entry.downgrade();
        move || {
            let (Some(r), Some(e)) = (root.upgrade(), entry_weak.upgrade()) else { return };
            let (w, h) = frame_size(&r);
            let parsed = e.text().trim().parse::<i32>().ok().filter(|v| *v > 0).map(|v| v.max(min));
            match parsed {
                Some(v) if is_width => r.set_size_request(v, h),
                Some(v) => r.set_size_request(w, v),
                None => {}
            }
        }
    };

    {
        let commit = commit.clone();
        let refresh = refresh.clone();
        entry.connect_activate(move |_| { commit(); refresh(); });
    }

    let focus_ctrl = EventControllerFocus::new();
    focus_ctrl.connect_leave(move |_| { commit(); refresh(); });
    entry.add_controller(focus_ctrl);
}

#[allow(clippy::too_many_arguments)]
fn wire_dim_step(
    button: &Button, root: &Overlay, delta: i32, is_width: bool,
    aspect: f64, min_w: i32, min_h: i32, refresh: Rc<dyn Fn()>,
) {
    let root = root.downgrade();
    button.connect_clicked(move |_| {
        let Some(r) = root.upgrade() else { return };
        let (w, h) = frame_size(&r);
        let (new_w, new_h) = if is_width {
            let nw = (w + delta).max(min_w);
            (nw, (nw as f64 / aspect).round().max(min_h as f64) as i32)
        } else {
            let nh = (h + delta).max(min_h);
            ((nh as f64 * aspect).round().max(min_w as f64) as i32, nh)
        };
        r.set_size_request(new_w, new_h);
        refresh();
    });
}

pub const MEDIA_FRAME_CSS: &str = r#"
.media-frame {
    border-radius: 4px;
}
.media-frame:hover, .media-frame:focus {
    box-shadow: 0 0 0 2px rgba(97,175,239,0.65);
}
.media-frame-zone {
    opacity: 0.95;
}
.media-frame-dim-bar {
    background: rgba(10,11,14,0.82);
    border-radius: 4px;
    padding: 3px 5px;
}
.media-frame-dim-label {
    color: #8a97a5;
    font-size: 8pt;
}
.media-frame-dim-entry {
    background: rgba(255,255,255,0.06);
    border: none;
    box-shadow: none;
    color: #e4ebf0;
    font-family: monospace;
    font-size: 8pt;
    min-height: 0;
    padding: 1px 3px;
}
.media-frame-dim-step {
    background: transparent;
    border: none;
    box-shadow: none;
    color: #c4cdd4;
    font-size: 9pt;
    min-height: 0;
    min-width: 0;
    padding: 0px 4px;
}
.media-frame-dim-step:hover {
    color: #ffffff;
}
"#;
