// Text highlight and colour tagging — right-click context menu on the TextView.
//
// Pre-registers a fixed palette of foreground and background colour tags.
// The popover appears only when text is selected; clicking a swatch applies
// the tag to the selection.  "Remove" clears all highlight tags from the
// selected range.
//
// Why a static palette rather than an arbitrary colour picker:
//   GTK TextTag names must be stable across save/load round-trips.  Using
//   enum-style names (hl-fg-yellow, hl-bg-green …) lets the serialiser
//   persist them as tag names without encoding RGBA in the file format.

use std::{cell::RefCell, rc::Rc};

use gtk::{gdk, prelude::*, Box, Button, Label, Orientation, Popover, TextView};

// (tag-name, colour hex, swatch glyph)
const FG_SWATCHES: &[(&str, &str, &str)] = &[
    ("hl-fg-default", "#d8dee9", "●"), // almost-white
    ("hl-fg-black",   "#111111", "●"),
    ("hl-fg-yellow",  "#ffd060", "●"),
    ("hl-fg-green",   "#7ec8a0", "●"),
    ("hl-fg-red",     "#ff6b6b", "●"),
    ("hl-fg-blue",    "#61afef", "●"),
    ("hl-fg-purple",  "#c678dd", "●"),
    ("hl-fg-orange",  "#e5a050", "●"),
    ("hl-fg-teal",    "#56b6c2", "●"),
    ("hl-fg-pink",    "#e06c75", "●"),
];
// Catpuccin based custom color palette
const BG_SWATCHES: &[(&str, &str, &str)] = &[
    ("hl-bg-none",   "transparent", "□"),
    ("hl-bg-white",  "#ffffff", "▁"),
    ("hl-bg-yellow", "#f9d982", "▁"),
    ("hl-bg-green",  "#97d5a5", "▁"),
    ("hl-bg-pink",   "#e8a2c7", "▁"),
    ("hl-bg-orange", "#e8b07a", "▁"),
    ("hl-bg-blue",   "#7aaee6", "▁"),
    ("hl-bg-purple", "#b694d6", "▁"),
    ("hl-bg-teal",   "#84d4cc", "▁"),
    ("hl-bg-red",    "#e28d96", "▁"),
];


pub fn wire_text_highlight(view: &TextView) {
    let buffer = view.buffer();
    register_highlight_tags(&buffer);

    // The selection bounds are saved at right-click time (before the TextView's
    // built-in handler fires and moves the cursor, clearing the selection).
    // Swatch button clicks then use these saved bounds instead of asking the
    // buffer — which by that point has no selection.
    let saved_sel: Rc<RefCell<Option<(i32, i32)>>> = Rc::new(RefCell::new(None));

    let popover = build_colour_popover(&buffer, saved_sel.clone());
    popover.set_parent(view);
    popover.set_has_arrow(true);
    popover.set_position(gtk::PositionType::Bottom);

    let popover_ref  = popover.clone();
    let buf_for_gate = buffer.clone();
    let sel_writer   = saved_sel;

    let gesture = gtk::GestureClick::new();
    gesture.set_button(gdk::BUTTON_SECONDARY);
    // CAPTURE phase: our handler fires before the TextView's built-in
    // right-click handler, so the selection is still intact when we read it.
    // We do NOT claim the event — claiming creates a pointer grab that blocks
    // subsequent mouse clicks inside the popover.
    gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
    gesture.connect_pressed(move |_gest, _n, x, y| {
        let Some((start, end)) = buf_for_gate.selection_bounds() else { return };
        // Persist the selection as plain offsets — the buffer content won't
        // change between right-click and swatch click, so offsets stay valid.
        *sel_writer.borrow_mut() = Some((start.offset(), end.offset()));
        let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
        popover_ref.set_pointing_to(Some(&rect));
        popover_ref.popup();
    });
    view.add_controller(gesture);
}


// ── Tag registration ──────────────────────────────────────────────────────────

fn register_highlight_tags(buffer: &gtk::TextBuffer) {
    let tt = buffer.tag_table();

    for swatch in FG_SWATCHES {
        let (name, colour, _) = *swatch;
        if tt.lookup(name).is_none() {
            if let Some(t) = buffer.create_tag(Some(name), &[]) {
                t.set_property("foreground", colour);
            }
        }
    }

    for swatch in BG_SWATCHES {
        let (name, colour, _) = *swatch;
        if tt.lookup(name).is_none() {
            if let Some(t) = buffer.create_tag(Some(name), &[]) {
                t.set_property("background", colour);
            }
        }
    }
}


// ── Popover UI ────────────────────────────────────────────────────────────────

fn build_colour_popover(
    buffer:   &gtk::TextBuffer,
    saved_sel: Rc<RefCell<Option<(i32, i32)>>>,
) -> Popover {
    let popover = Popover::new();
    popover.add_css_class("hl-popover");

    let root = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    let fg_label = Label::builder()
        .label("Text colour")
        .halign(gtk::Align::Start)
        .build();
    fg_label.add_css_class("hl-section-label");
    root.append(&fg_label);
    root.append(&swatch_row(buffer, FG_SWATCHES, &popover, saved_sel.clone()));

    let bg_label = Label::builder()
        .label("Highlight")
        .halign(gtk::Align::Start)
        .build();
    bg_label.add_css_class("hl-section-label");
    root.append(&bg_label);
    root.append(&swatch_row(buffer, BG_SWATCHES, &popover, saved_sel.clone()));

    let sep = gtk::Separator::new(Orientation::Horizontal);
    root.append(&sep);

    let clear_btn = Button::with_label("✕  Remove colour");
    clear_btn.add_css_class("hl-clear-btn");
    {
        let buf_c = buffer.clone();
        let pop_c = popover.clone();
        let sel_c = saved_sel;
        clear_btn.connect_clicked(move |_| {
            clear_all_highlight_tags(&buf_c, &sel_c.borrow());
            pop_c.popdown();
        });
    }
    root.append(&clear_btn);

    popover.set_child(Some(&root));
    popover
}

fn swatch_row(
    buffer:    &gtk::TextBuffer,
    swatches:  &'static [(&'static str, &'static str, &'static str)],
    popover:   &Popover,
    saved_sel: Rc<RefCell<Option<(i32, i32)>>>,
) -> Box {
    let row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .build();

    for swatch in swatches {
        let (tag_name, colour, glyph) = *swatch;

        let btn = Button::with_label(glyph);
        btn.add_css_class("hl-swatch-btn");
        btn.set_tooltip_text(Some(tag_name.trim_start_matches("hl-")));
        btn.set_cursor_from_name(Some("pointer"));

        let provider = gtk::CssProvider::new();
        provider.load_from_data(&format!(".hl-swatch-btn {{ color: {}; }}", colour));
        btn.style_context()
            .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let buf_ref = buffer.clone();
        let pop_ref = popover.clone();
        let sel_ref = saved_sel.clone();
        btn.connect_clicked(move |_| {
            apply_tag_to_saved_sel(&buf_ref, tag_name, &sel_ref.borrow());
            pop_ref.popdown();
        });

        row.append(&btn);
    }
    row
}


// ── Tag application / removal ─────────────────────────────────────────────────

fn apply_tag_to_saved_sel(
    buffer:   &gtk::TextBuffer,
    tag_name: &str,
    saved:    &Option<(i32, i32)>,
) {
    let Some((s_off, e_off)) = *saved else { return };
    let tag = match buffer.tag_table().lookup(tag_name) {
        Some(t) => t,
        None    => return,
    };

    let start = buffer.iter_at_offset(s_off);
    let end   = buffer.iter_at_offset(e_off);

    // Remove conflicting tags of the same kind first.
    let conflict_list: &[(&str, &str, &str)] = if tag_name.starts_with("hl-fg-") {
        FG_SWATCHES
    } else {
        BG_SWATCHES
    };
    for swatch in conflict_list {
        let (other_name, _, _) = *swatch;
        if let Some(other_tag) = buffer.tag_table().lookup(other_name) {
            buffer.remove_tag(&other_tag, &start, &end);
        }
    }

    buffer.apply_tag(&tag, &start, &end);

    // When a background highlight is applied, automatically force fg to black
    // for legibility.  The user can still override the fg colour afterwards.
    if tag_name.starts_with("hl-bg-") && tag_name != "hl-bg-none" {
        for swatch in FG_SWATCHES {
            let (other_name, _, _) = *swatch;
            if let Some(other_tag) = buffer.tag_table().lookup(other_name) {
                buffer.remove_tag(&other_tag, &start, &end);
            }
        }
        if let Some(black_tag) = buffer.tag_table().lookup("hl-fg-black") {
            buffer.apply_tag(&black_tag, &start, &end);
        }
    }
}

fn clear_all_highlight_tags(buffer: &gtk::TextBuffer, saved: &Option<(i32, i32)>) {
    let Some((s_off, e_off)) = *saved else { return };
    let start = buffer.iter_at_offset(s_off);
    let end   = buffer.iter_at_offset(e_off);
    for swatch in FG_SWATCHES.iter().chain(BG_SWATCHES.iter()) {
        let (name, _, _) = *swatch;
        if let Some(tag) = buffer.tag_table().lookup(name) {
            buffer.remove_tag(&tag, &start, &end);
        }
    }
}


// ── CSS ───────────────────────────────────────────────────────────────────────
pub const HIGHLIGHT_CSS: &str = r#"
.hl-popover {
    background: #161c24;
    border: 1px solid rgba(255,255,255,0.10);
    border-radius: 8px;
}

.hl-section-label {
    color: #5a7a8a;
    font-size: 8pt;
    font-weight: 600;
}

.hl-swatch-btn {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.10);
    border-radius: 4px;
    box-shadow: none;
    font-size: 14pt;
    min-width: 28px;
    min-height: 28px;
    padding: 2px 4px;
}

.hl-swatch-btn:hover {
    background: rgba(255,255,255,0.14);
    border-color: rgba(255,255,255,0.22);
}

.hl-clear-btn {
    background: transparent;
    border: 1px solid rgba(255,100,100,0.28);
    border-radius: 5px;
    box-shadow: none;
    color: #a06070;
    font-size: 8.5pt;
    font-weight: 600;
    padding: 4px 10px;
    min-height: 0;
}

.hl-clear-btn:hover {
    background: rgba(255,60,60,0.10);
    color: #d08090;
}
"#;


/*




// Text highlight and colour tagging — right-click context menu on the TextView.
//
// Pre-registers a fixed palette of foreground and background colour tags.
// The popover appears only when text is selected; clicking a swatch applies
// the tag to the selection.  "Remove" clears all highlight tags from the
// selected range.
//
// Why a static palette rather than an arbitrary colour picker:
//   GTK TextTag names must be stable across save/load round-trips.  Using
//   enum-style names (hl-fg-yellow, hl-bg-green …) lets the serialiser
//   persist them as tag names without encoding RGBA in the file format.

use std::{cell::RefCell, rc::Rc};

use gtk::{gdk, prelude::*, Box, Button, Label, Orientation, Popover, TextView};

// (tag-name, colour hex, swatch glyph)
const FG_SWATCHES: &[(&str, &str, &str)] = &[
    ("hl-fg-default", "#d8dee9", "●"), // almost-white
    ("hl-fg-black",   "#111111", "●"),
    ("hl-fg-yellow",  "#ffd060", "●"),
    ("hl-fg-green",   "#7ec8a0", "●"),
    ("hl-fg-red",     "#ff6b6b", "●"),
    ("hl-fg-blue",    "#61afef", "●"),
    ("hl-fg-purple",  "#c678dd", "●"),
    ("hl-fg-orange",  "#e5a050", "●"),
    ("hl-fg-teal",    "#56b6c2", "●"),
    ("hl-fg-pink",    "#e06c75", "●"),
];
// Catpuccin based custom color palette
const BG_SWATCHES: &[(&str, &str, &str)] = &[
    ("hl-bg-none",   "transparent", "□"),
    ("hl-bg-white",  "#ffffff", "▁"),
    ("hl-bg-yellow", "#f9d982", "▁"),
    ("hl-bg-green",  "#97d5a5", "▁"),
    ("hl-bg-pink",   "#e8a2c7", "▁"),
    ("hl-bg-orange", "#e8b07a", "▁"),
    ("hl-bg-blue",   "#7aaee6", "▁"),
    ("hl-bg-purple", "#b694d6", "▁"),
    ("hl-bg-teal",   "#84d4cc", "▁"),
    ("hl-bg-red",    "#e28d96", "▁"),
];


pub fn wire_text_highlight(view: &TextView) {
    let buffer = view.buffer();
    register_highlight_tags(&buffer);

    // The selection bounds are saved at right-click time (before the TextView's
    // built-in handler fires and moves the cursor, clearing the selection).
    // Swatch button clicks then use these saved bounds instead of asking the
    // buffer — which by that point has no selection.
    let saved_sel: Rc<RefCell<Option<(i32, i32)>>> = Rc::new(RefCell::new(None));

    let popover = build_colour_popover(&buffer, saved_sel.clone());
    popover.set_parent(view);
    popover.set_has_arrow(true);
    popover.set_position(gtk::PositionType::Bottom);

    let popover_ref  = popover.clone();
    let buf_for_gate = buffer.clone();
    let sel_writer   = saved_sel;

    let gesture = gtk::GestureClick::new();
    gesture.set_button(gdk::BUTTON_SECONDARY);
    // CAPTURE phase: our handler fires before the TextView's built-in
    // right-click handler, so the selection is still intact when we read it.
    // We do NOT claim the event — claiming creates a pointer grab that blocks
    // subsequent mouse clicks inside the popover.
    gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
    gesture.connect_pressed(move |_gest, _n, x, y| {
        let Some((start, end)) = buf_for_gate.selection_bounds() else { return };
        // Persist the selection as plain offsets — the buffer content won't
        // change between right-click and swatch click, so offsets stay valid.
        *sel_writer.borrow_mut() = Some((start.offset(), end.offset()));
        let rect = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
        popover_ref.set_pointing_to(Some(&rect));
        popover_ref.popup();
    });
    view.add_controller(gesture);
}


// ── Tag registration ──────────────────────────────────────────────────────────

fn register_highlight_tags(buffer: &gtk::TextBuffer) {
    let tt = buffer.tag_table();

    for swatch in FG_SWATCHES {
        let (name, colour, _) = *swatch;
        if tt.lookup(name).is_none() {
            if let Some(t) = buffer.create_tag(Some(name), &[]) {
                t.set_property("foreground", colour);
            }
        }
    }

    for swatch in BG_SWATCHES {
        let (name, colour, _) = *swatch;
        if tt.lookup(name).is_none() {
            if let Some(t) = buffer.create_tag(Some(name), &[]) {
                t.set_property("background", colour);
            }
        }
    }
}


// ── Popover UI ────────────────────────────────────────────────────────────────

fn build_colour_popover(
    buffer:   &gtk::TextBuffer,
    saved_sel: Rc<RefCell<Option<(i32, i32)>>>,
) -> Popover {
    let popover = Popover::new();
    popover.add_css_class("hl-popover");

    let root = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    let fg_label = Label::builder()
        .label("Text colour")
        .halign(gtk::Align::Start)
        .build();
    fg_label.add_css_class("hl-section-label");
    root.append(&fg_label);
    root.append(&swatch_row(buffer, FG_SWATCHES, &popover, saved_sel.clone()));

    let bg_label = Label::builder()
        .label("Highlight")
        .halign(gtk::Align::Start)
        .build();
    bg_label.add_css_class("hl-section-label");
    root.append(&bg_label);
    root.append(&swatch_row(buffer, BG_SWATCHES, &popover, saved_sel.clone()));

    let sep = gtk::Separator::new(Orientation::Horizontal);
    root.append(&sep);

    let clear_btn = Button::with_label("✕  Remove colour");
    clear_btn.add_css_class("hl-clear-btn");
    {
        let buf_c = buffer.clone();
        let pop_c = popover.clone();
        let sel_c = saved_sel;
        clear_btn.connect_clicked(move |_| {
            clear_all_highlight_tags(&buf_c, &sel_c.borrow());
            pop_c.popdown();
        });
    }
    root.append(&clear_btn);

    popover.set_child(Some(&root));
    popover
}

fn swatch_row(
    buffer:    &gtk::TextBuffer,
    swatches:  &'static [(&'static str, &'static str, &'static str)],
    popover:   &Popover,
    saved_sel: Rc<RefCell<Option<(i32, i32)>>>,
) -> Box {
    let row = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .build();

    for swatch in swatches {
        let (tag_name, colour, glyph) = *swatch;

        let btn = Button::with_label(glyph);
        btn.add_css_class("hl-swatch-btn");
        btn.set_tooltip_text(Some(tag_name.trim_start_matches("hl-")));
        btn.set_cursor_from_name(Some("pointer"));

        let provider = gtk::CssProvider::new();
        provider.load_from_data(&format!(".hl-swatch-btn {{ color: {}; }}", colour));
        btn.style_context()
            .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let buf_ref = buffer.clone();
        let pop_ref = popover.clone();
        let sel_ref = saved_sel.clone();
        btn.connect_clicked(move |_| {
            apply_tag_to_saved_sel(&buf_ref, tag_name, &sel_ref.borrow());
            pop_ref.popdown();
        });

        row.append(&btn);
    }
    row
}


// ── Tag application / removal ─────────────────────────────────────────────────

fn apply_tag_to_saved_sel(
    buffer:   &gtk::TextBuffer,
    tag_name: &str,
    saved:    &Option<(i32, i32)>,
) {
    let Some((s_off, e_off)) = *saved else { return };
    let tag = match buffer.tag_table().lookup(tag_name) {
        Some(t) => t,
        None    => return,
    };

    let start = buffer.iter_at_offset(s_off);
    let end   = buffer.iter_at_offset(e_off);

    // Remove conflicting tags of the same kind first.
    let conflict_list: &[(&str, &str, &str)] = if tag_name.starts_with("hl-fg-") {
        FG_SWATCHES
    } else {
        BG_SWATCHES
    };
    for swatch in conflict_list {
        let (other_name, _, _) = *swatch;
        if let Some(other_tag) = buffer.tag_table().lookup(other_name) {
            buffer.remove_tag(&other_tag, &start, &end);
        }
    }

    buffer.apply_tag(&tag, &start, &end);
}

fn clear_all_highlight_tags(buffer: &gtk::TextBuffer, saved: &Option<(i32, i32)>) {
    let Some((s_off, e_off)) = *saved else { return };
    let start = buffer.iter_at_offset(s_off);
    let end   = buffer.iter_at_offset(e_off);
    for swatch in FG_SWATCHES.iter().chain(BG_SWATCHES.iter()) {
        let (name, _, _) = *swatch;
        if let Some(tag) = buffer.tag_table().lookup(name) {
            buffer.remove_tag(&tag, &start, &end);
        }
    }
}


// ── CSS ───────────────────────────────────────────────────────────────────────
pub const HIGHLIGHT_CSS: &str = r#"
.hl-popover {
    background: #161c24;
    border: 1px solid rgba(255,255,255,0.10);
    border-radius: 8px;
}

.hl-section-label {
    color: #5a7a8a;
    font-size: 8pt;
    font-weight: 600;
}

.hl-swatch-btn {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.10);
    border-radius: 4px;
    box-shadow: none;
    font-size: 14pt;
    min-width: 28px;
    min-height: 28px;
    padding: 2px 4px;
}

.hl-swatch-btn:hover {
    background: rgba(255,255,255,0.14);
    border-color: rgba(255,255,255,0.22);
}

.hl-clear-btn {
    background: transparent;
    border: 1px solid rgba(255,100,100,0.28);
    border-radius: 5px;
    box-shadow: none;
    color: #a06070;
    font-size: 8.5pt;
    font-weight: 600;
    padding: 4px 10px;
    min-height: 0;
}

.hl-clear-btn:hover {
    background: rgba(255,60,60,0.10);
    color: #d08090;
}
"#;


*/
