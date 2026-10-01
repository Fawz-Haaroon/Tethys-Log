// MediaViewer — the expanded/fullscreen presentation for an image or
// video, reached by double-clicking the media, pressing Enter while it's
// focused, or the explicit expand button in a video's control bar.
//
// This is a window-level Overlay, not a second gtk::Window. Tethys Log's
// window has exactly one child (workspace_view.widget()) going into
// boot.rs's ApplicationWindow::builder -- install() wraps that one child
// in an Overlay and keeps the viewer surface as a hidden-by-default
// overlay sibling, so opening it never spawns anything, never changes
// which window the compositor thinks is active, and never gives the
// underlying TextView a reason to lose its cursor or selection: the
// editor is still sitting right there under the backdrop, untouched.
//
// Video does NOT get a second VideoPlayer. An earlier version constructed
// a fresh one for the viewer and paused the inline copy first -- which
// meant, for a brief window, two independent MediaFile objects existed
// for what the user experienced as one video, and depending on exactly
// how disposal played out once the viewer closed, that could leave the
// old one alive and still producing audio nobody could see or stop.
// Instead, entering the viewer detaches the *same* VideoPlayer's widget
// from its inline MediaFrame and reparents it into the viewer's slot;
// leaving does the reverse. There is only ever one MediaFile for a given
// video, for as long as that video's widget exists, full stop -- and
// since nothing about the widget itself changes across the move, whatever
// it was doing (playing, paused, mid-seek) keeps doing it, uninterrupted,
// on both sides of the trip.
//
// Images have no such lifecycle to protect, so they stay simple: a fresh
// Picture is built for the viewer each time, pointed at the same file.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use gtk::{gdk, prelude::*, Align, Box as GtkBox, Button, EventControllerKey, Orientation, Overlay, Picture};

thread_local! {
    static ACTIVE_VIEWER: RefCell<Option<Rc<MediaViewer>>> = const { RefCell::new(None) };
}

struct MediaViewer {
    backdrop: GtkBox,
    slot: GtkBox,
    close_btn: Button,
    // The widget currently borrowed into the viewer (video case), and how
    // to give it back to wherever it came from. There is deliberately no
    // second player field here to pause/stop on close -- there is nothing
    // to stop, because there was never a second player.
    borrowed: RefCell<Option<(gtk::Widget, Box<dyn Fn(&gtk::Widget)>)>>,
}

/// Wraps `window_content` in a window-level Overlay carrying the (hidden)
/// viewer surface, and remembers it for show_image / show_reparented to
/// reach later. Call once, from boot.rs, before the window's child is set.
pub fn install(window_content: &impl IsA<gtk::Widget>) -> Overlay {
    let close_btn = Button::with_label("✕");
    close_btn.add_css_class("viewer-close-btn");
    close_btn.set_halign(Align::End);
    close_btn.set_valign(Align::Start);

    let slot = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .halign(Align::Fill).valign(Align::Fill)
        .hexpand(true).vexpand(true)
        .build();
    slot.add_css_class("viewer-slot");

    let backdrop = GtkBox::builder().orientation(Orientation::Vertical).visible(false).build();
    backdrop.add_css_class("viewer-backdrop");

    let inner = Overlay::new();
    inner.set_child(Some(&slot));
    inner.add_overlay(&close_btn);
    backdrop.append(&inner);

    let viewer = Rc::new(MediaViewer {
        backdrop: backdrop.clone(),
        slot,
        close_btn: close_btn.clone(),
        borrowed: RefCell::new(None),
    });

    {
        let viewer_weak = Rc::downgrade(&viewer);
        close_btn.connect_clicked(move |_| {
            if let Some(v) = viewer_weak.upgrade() { close(&v); }
        });
    }

    let root = Overlay::new();
    root.set_child(Some(window_content));
    root.add_overlay(&backdrop);

    // Escape is handled at the root, not on backdrop -- backdrop is a
    // sibling of the editor content, not an ancestor of whatever has GTK
    // focus while the viewer is open, so a controller on backdrop alone
    // would never see the key press. The root is always an ancestor of
    // everything in the window, the same reason app::keybindings attaches
    // its own controller to the window rather than to any one widget.
    let keys = EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let viewer_weak = Rc::downgrade(&viewer);
        keys.connect_key_pressed(move |_, key, _, _| {
            let Some(v) = viewer_weak.upgrade() else { return gtk::glib::Propagation::Proceed; };
            if !v.backdrop.is_visible() { return gtk::glib::Propagation::Proceed; }
            if key == gdk::Key::Escape {
                close(&v);
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
    }
    root.add_controller(keys);

    ACTIVE_VIEWER.with(|cell| *cell.borrow_mut() = Some(viewer));

    root
}

fn return_borrowed_content(viewer: &Rc<MediaViewer>) {
    if let Some((widget, return_to)) = viewer.borrowed.borrow_mut().take() {
        widget.unparent();
        return_to(&widget);
    }
}

fn close(viewer: &Rc<MediaViewer>) {
    viewer.backdrop.set_visible(false);
    return_borrowed_content(viewer);
    while let Some(child) = viewer.slot.first_child() {
        viewer.slot.remove(&child);
    }
}

fn with_viewer(f: impl FnOnce(&Rc<MediaViewer>)) {
    ACTIVE_VIEWER.with(|cell| {
        if let Some(v) = cell.borrow().as_ref() { f(v); }
    });
}

/// Shows `path` as a large, comfortably-scaled image. Images have no
/// playback state to protect, so this just builds a fresh Picture each
/// time rather than reparenting anything.
pub fn show_image(path: &Path) {
    let path = path.to_path_buf();
    with_viewer(move |viewer| {
        return_borrowed_content(viewer);
        while let Some(child) = viewer.slot.first_child() { viewer.slot.remove(&child); }

        let picture = Picture::for_filename(&path);
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_can_shrink(true);
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        picture.set_margin_top(48);
        picture.set_margin_bottom(48);
        picture.set_margin_start(48);
        picture.set_margin_end(48);
        viewer.slot.append(&picture);

        viewer.backdrop.set_visible(true);
        viewer.close_btn.grab_focus();
    });
}

/// Detaches `content` from wherever it currently lives and shows it in
/// the viewer at a larger scale, calling `return_to` with the same widget
/// when the viewer later closes so the caller can put it back. This is
/// the entire mechanism behind fullscreen video: the widget -- and the
/// one MediaFile it owns -- physically relocates for a while rather than
/// a second copy being built to stand in for it.
pub fn show_reparented(content: &impl IsA<gtk::Widget>, return_to: impl Fn(&gtk::Widget) + 'static) {
    let content: gtk::Widget = content.clone().upcast();
    with_viewer(move |viewer| {
        return_borrowed_content(viewer);
        while let Some(child) = viewer.slot.first_child() { viewer.slot.remove(&child); }

        content.set_margin_top(32);
        content.set_margin_bottom(32);
        content.set_margin_start(32);
        content.set_margin_end(32);
        viewer.slot.append(&content);
        *viewer.borrowed.borrow_mut() = Some((content, Box::new(return_to)));

        viewer.backdrop.set_visible(true);
        viewer.close_btn.grab_focus();
    });
}

pub const VIEWER_CSS: &str = r#"
.viewer-backdrop {
    background: rgba(6,7,9,0.94);
}
.viewer-close-btn {
    background: rgba(255,255,255,0.08);
    border: none;
    border-radius: 999px;
    box-shadow: none;
    color: #e4ebf0;
    font-size: 11pt;
    margin: 16px;
    min-height: 32px;
    min-width: 32px;
    padding: 0;
}
.viewer-close-btn:hover {
    background: rgba(255,255,255,0.18);
}
"#;
