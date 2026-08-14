// Minimal vim modal layer for Tethys Log.
//
// Modes: Normal, Insert, Visual
//
// Cursor shape:
//   Normal / Visual  →  block cursor  (set_overwrite(true))
//   Insert           →  I-beam cursor (set_overwrite(false))
//
// Normal mode motions
//   h j k l         — char/line navigation
//   w b             — word forward/backward
//   0 $             — line start / end
//   gg G            — buffer start / end
//
// Normal mode actions
//   x               — delete char under cursor
//   d               — delete current line
//   y               — yank current line
//   p               — paste yanked line below
//   u  Ctrl+r       — undo / redo
//   v               — enter Visual mode
//   /               — open search bar
//   n  N            — next / previous search match
//
// Visual mode
//   Movement keys   — extend selection from anchor to cursor
//   y               — yank selection, return to Normal
//   d / x           — delete selection, return to Normal
//   Esc             — cancel selection, return to Normal
//
// Normal → Insert:  i a o O A I
// Insert → Normal:  Escape

use std::{cell::RefCell, rc::Rc};

use gtk::{gdk, glib, prelude::*, TextView};

use crate::editor::canvas::{codec::image_dir_for_note, history::{self, DocumentHistory}};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
}

pub struct VimState {
    mode:            RefCell<VimMode>,
    yank:            RefCell<Option<String>>,
    g_pending:       RefCell<bool>,
    visual_start:    RefCell<Option<i32>>, // buffer char-offset of visual anchor
    // Stored so undo/redo can locate this note's image directory -- see
    // editor::canvas::history, which needs it to reconstruct embedded
    // media when loading a snapshot.
    pub note_identifier: String,
    pub history:     Rc<RefCell<DocumentHistory>>,
}

impl VimState {
    pub fn new(note_identifier: &str, history: Rc<RefCell<DocumentHistory>>) -> Rc<Self> {
        Rc::new(Self {
            mode:            RefCell::new(VimMode::Insert),
            yank:            RefCell::new(None),
            g_pending:       RefCell::new(false),
            visual_start:    RefCell::new(None),
            note_identifier: note_identifier.to_string(),
            history,
        })
    }

    pub fn mode(&self) -> VimMode { *self.mode.borrow() }

    fn set_mode(&self, m: VimMode) { *self.mode.borrow_mut() = m; }
}


pub fn wire_vim(
    view:             &TextView,
    state:            Rc<VimState>,
    on_mode_change:   impl Fn(VimMode)  + 'static,
    on_search:        impl Fn()         + 'static,
    on_search_next:   impl Fn()         + 'static,
    on_search_prev:   impl Fn()         + 'static,
) {
    let on_mode_change:  Rc<dyn Fn(VimMode)> = Rc::new(on_mode_change);
    let on_search:       Rc<dyn Fn()>        = Rc::new(on_search);
    let on_search_next:  Rc<dyn Fn()>        = Rc::new(on_search_next);
    let on_search_prev:  Rc<dyn Fn()>        = Rc::new(on_search_prev);

    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);

    let view_ref  = view.clone();
    let state_ref = state.clone();

    keys.connect_key_pressed(move |_, key, _kc, mods| {
        let ctrl  = mods.contains(gdk::ModifierType::CONTROL_MASK);
        let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);
        let mode  = state_ref.mode();

        match mode {
            VimMode::Insert => handle_insert(
                &view_ref, &state_ref, key, ctrl, shift, &on_mode_change,
            ),

            VimMode::Normal => handle_normal(
                &view_ref, &state_ref, key, ctrl, shift,
                &on_mode_change, &on_search, &on_search_next, &on_search_prev,
            ),

            VimMode::Visual => handle_visual(
                &view_ref, &state_ref, key, ctrl, shift, &on_mode_change,
            ),
        }
    });

    view.add_controller(keys);
}


// ── Insert mode ───────────────────────────────────────────────────────────────

fn handle_insert(
    view:           &TextView,
    state:          &Rc<VimState>,
    key:            gdk::Key,
    ctrl:           bool,
    shift:          bool,
    on_mode_change: &Rc<dyn Fn(VimMode)>,
) -> glib::Propagation {
    // Insert mode is the default, everyday typing mode, so the standard
    // Ctrl+Z / Ctrl+Shift+Z (and the equally common Ctrl+Y) editor bindings
    // live here rather than only under Normal mode's vim-style u / Ctrl+r.
    // These used to just fall through to GTK's own default TextView
    // keybinding, which worked only because buffer.set_enable_undo(true)
    // was on -- now that undo is ours (see surface.rs for why), it has to
    // be intercepted explicitly or Ctrl+Z would silently do nothing.
    //
    // Matching both `z`/`Z` (and `y`/`Y`) is not redundant: X11/GDK reports
    // a held Shift by changing the keyval itself, not just the modifier --
    // Shift+Z arrives as the distinct keyval Z, never as z with SHIFT_MASK
    // set. Comparing only against the lowercase keyval is why Ctrl+Z (undo)
    // worked and Ctrl+Shift+Z (redo) silently didn't. `shift` below is still
    // the right way to tell undo from redo apart -- caps lock alone also
    // produces the uppercase keyval but reports LOCK_MASK, not SHIFT_MASK,
    // so it can't be mistaken for a redo request.
    if ctrl && matches!(key, gdk::Key::z | gdk::Key::Z) {
        let buffer    = view.buffer();
        let image_dir = image_dir_for_note(&state.note_identifier);
        if shift {
            history::redo(&state.history, &buffer, view, &image_dir);
        } else {
            history::undo(&state.history, &buffer, view, &image_dir);
        }
        return glib::Propagation::Stop;
    }
    if ctrl && matches!(key, gdk::Key::y | gdk::Key::Y) {
        let buffer    = view.buffer();
        let image_dir = image_dir_for_note(&state.note_identifier);
        history::redo(&state.history, &buffer, view, &image_dir);
        return glib::Propagation::Stop;
    }

    if key == gdk::Key::Escape {
        // step cursor back one char (vim convention on Esc)
        let buffer = view.buffer();
        let iter   = buffer.iter_at_mark(&buffer.get_insert());
        if !iter.starts_line() {
            let mut back = iter;
            back.backward_char();
            buffer.place_cursor(&back);
        }
        enter_mode(view, state, VimMode::Normal, on_mode_change);
        return glib::Propagation::Stop;
    }
    glib::Propagation::Proceed
}


// ── Normal mode ───────────────────────────────────────────────────────────────

fn handle_normal(
    view:           &TextView,
    state:          &Rc<VimState>,
    key:            gdk::Key,
    ctrl:           bool,
    shift:          bool,
    on_mode_change: &Rc<dyn Fn(VimMode)>,
    on_search:      &Rc<dyn Fn()>,
    on_search_next: &Rc<dyn Fn()>,
    on_search_prev: &Rc<dyn Fn()>,
) -> glib::Propagation {
    let buffer = view.buffer();

    if ctrl {
        match key {
            gdk::Key::r | gdk::Key::R => {
                let image_dir = image_dir_for_note(&state.note_identifier);
                history::redo(&state.history, &buffer, view, &image_dir);
                return glib::Propagation::Stop;
            }
            _ => return glib::Propagation::Proceed,
        }
    }

    // gg double-g
    let g_was_pending = *state.g_pending.borrow();
    if g_was_pending {
        *state.g_pending.borrow_mut() = false;
        if key == gdk::Key::g {
            buffer.place_cursor(&buffer.start_iter());
            scroll_to_cursor(view);
            return glib::Propagation::Stop;
        }
    }

    match (shift, key) {
        // ── Insert transitions ─────────────────────────────────────────────
        (false, gdk::Key::i) => {
            enter_mode(view, state, VimMode::Insert, on_mode_change);
        }
        (false, gdk::Key::a) => {
            move_forward_char(&buffer);
            enter_mode(view, state, VimMode::Insert, on_mode_change);
        }
        (false, gdk::Key::o) => {
            move_to_line_end(&buffer);
            buffer.insert_at_cursor("\n");
            enter_mode(view, state, VimMode::Insert, on_mode_change);
        }
        (true, gdk::Key::O) => {
            move_to_line_start(&buffer);
            buffer.insert_at_cursor("\n");
            let iter = buffer.iter_at_mark(&buffer.get_insert());
            if let Some(prev) = iter_prev_line(&buffer, &iter) {
                buffer.place_cursor(&prev);
            }
            enter_mode(view, state, VimMode::Insert, on_mode_change);
        }
        (true, gdk::Key::A) => {
            move_to_line_end(&buffer);
            enter_mode(view, state, VimMode::Insert, on_mode_change);
        }
        (true, gdk::Key::I) => {
            move_to_line_start(&buffer);
            enter_mode(view, state, VimMode::Insert, on_mode_change);
        }

        // ── Visual transition ─────────────────────────────────────────────
        (false, gdk::Key::v) => {
            let iter = buffer.iter_at_mark(&buffer.get_insert());
            *state.visual_start.borrow_mut() = Some(iter.offset());
            enter_mode(view, state, VimMode::Visual, on_mode_change);
        }

        // ── Search ────────────────────────────────────────────────────────
        (false, gdk::Key::slash) => {
            on_search();
        }
        (false, gdk::Key::n) => {
            on_search_next();
        }
        (true, gdk::Key::N) => {
            on_search_prev();
        }

        // ── Motion ────────────────────────────────────────────────────────
        (false, gdk::Key::h) => {
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            iter.backward_char();
            buffer.place_cursor(&iter);
        }
        (false, gdk::Key::l) => { move_forward_char(&buffer); }
        (false, gdk::Key::j) => { move_line(&buffer, 1); }
        (false, gdk::Key::k) => { move_line(&buffer, -1); }
        (false, gdk::Key::w) => {
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            iter.forward_word_end();
            buffer.place_cursor(&iter);
        }
        (false, gdk::Key::b) => {
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            iter.backward_word_start();
            buffer.place_cursor(&iter);
        }
        (false, gdk::Key::_0) | (false, gdk::Key::Home) => {
            move_to_line_start(&buffer);
        }
        (true, gdk::Key::dollar) => { move_to_line_end(&buffer); }
        (false, gdk::Key::g) => {
            *state.g_pending.borrow_mut() = true;
            return glib::Propagation::Stop;
        }
        (true, gdk::Key::G) => {
            buffer.place_cursor(&buffer.end_iter());
            scroll_to_cursor(view);
        }

        // ── Edit ──────────────────────────────────────────────────────────
        (false, gdk::Key::x) => {
            let mut start = buffer.iter_at_mark(&buffer.get_insert());
            let mut end   = start;
            if !end.ends_line() {
                end.forward_char();
                buffer.delete(&mut start, &mut end);
            }
        }
        (false, gdk::Key::d) => { delete_current_line(&buffer); }
        (false, gdk::Key::y) => { yank_current_line(&buffer, state); }
        (false, gdk::Key::p) => { paste_after_line(&buffer, state); }
        (false, gdk::Key::u) => {
            let image_dir = image_dir_for_note(&state.note_identifier);
            history::undo(&state.history, &buffer, view, &image_dir);
        }

        // All other keys in Normal mode are swallowed.
        // Returning Proceed here would let unbound printable keys insert text
        // into the buffer while staying in Normal mode.
        _ => return glib::Propagation::Stop,
    }

    scroll_to_cursor(view);
    glib::Propagation::Stop
}


// ── Visual mode ───────────────────────────────────────────────────────────────

fn handle_visual(
    view:           &TextView,
    state:          &Rc<VimState>,
    key:            gdk::Key,
    ctrl:           bool,
    shift:          bool,
    on_mode_change: &Rc<dyn Fn(VimMode)>,
) -> glib::Propagation {
    let _ = ctrl;
    let buffer = view.buffer();

    match (shift, key) {
        // cancel selection
        (_, gdk::Key::Escape) => {
            buffer.place_cursor(&buffer.iter_at_mark(&buffer.get_insert()));
            *state.visual_start.borrow_mut() = None;
            enter_mode(view, state, VimMode::Normal, on_mode_change);
        }

        // yank selection → Normal
        (false, gdk::Key::y) => {
            if let Some((sel_start, sel_end)) = buffer.selection_bounds() {
                let text = buffer.text(&sel_start, &sel_end, false).to_string();
                *state.yank.borrow_mut() = Some(text);
            }
            buffer.place_cursor(&buffer.iter_at_mark(&buffer.get_insert()));
            *state.visual_start.borrow_mut() = None;
            enter_mode(view, state, VimMode::Normal, on_mode_change);
        }

        // delete selection → Normal
        (false, gdk::Key::d) | (false, gdk::Key::x) => {
            if let Some((mut s, mut e)) = buffer.selection_bounds() {
                let text = buffer.text(&s, &e, false).to_string();
                *state.yank.borrow_mut() = Some(text);
                buffer.delete(&mut s, &mut e);
            }
            *state.visual_start.borrow_mut() = None;
            enter_mode(view, state, VimMode::Normal, on_mode_change);
        }

        // movement in visual mode — extends selection from visual_start
        (false, gdk::Key::h) => {
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            iter.backward_char();
            extend_selection(&buffer, state, &iter);
        }
        (false, gdk::Key::l) => {
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            if !iter.ends_line() { iter.forward_char(); }
            extend_selection(&buffer, state, &iter);
        }
        (false, gdk::Key::j) => {
            let target = line_moved_iter(&buffer, 1);
            extend_selection(&buffer, state, &target);
        }
        (false, gdk::Key::k) => {
            let target = line_moved_iter(&buffer, -1);
            extend_selection(&buffer, state, &target);
        }
        (false, gdk::Key::w) => {
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            iter.forward_word_end();
            extend_selection(&buffer, state, &iter);
        }
        (false, gdk::Key::b) => {
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            iter.backward_word_start();
            extend_selection(&buffer, state, &iter);
        }
        (false, gdk::Key::_0) => {
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            iter.set_line_offset(0);
            extend_selection(&buffer, state, &iter);
        }
        (true, gdk::Key::dollar) => {
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            if !iter.ends_line() { iter.forward_to_line_end(); }
            extend_selection(&buffer, state, &iter);
        }

        _ => {} // swallow all other keys in visual mode
    }

    scroll_to_cursor(view);
    glib::Propagation::Stop
}

fn extend_selection(buffer: &gtk::TextBuffer, state: &Rc<VimState>, new_end: &gtk::TextIter) {
    let anchor_off = state.visual_start.borrow().unwrap_or_else(|| {
        buffer.iter_at_mark(&buffer.get_insert()).offset()
    });
    let anchor = buffer.iter_at_offset(anchor_off);
    buffer.select_range(&anchor, new_end);
}

fn line_moved_iter(buffer: &gtk::TextBuffer, delta: i32) -> gtk::TextIter {
    let iter   = buffer.iter_at_mark(&buffer.get_insert());
    let col    = iter.line_offset();
    let line   = iter.line();
    let target = (line + delta).max(0).min(buffer.end_iter().line());
    let mut new_iter = buffer.iter_at_line(target).unwrap_or_else(|| buffer.end_iter());
    let line_len = {
        let mut e = new_iter;
        e.forward_to_line_end();
        e.line_offset()
    };
    new_iter.set_line_offset(col.min(line_len));
    new_iter
}


// ── mode-change helper ────────────────────────────────────────────────────────

fn enter_mode(
    view:           &TextView,
    state:          &Rc<VimState>,
    mode:           VimMode,
    on_mode_change: &Rc<dyn Fn(VimMode)>,
) {
    state.set_mode(mode);
    // block cursor in Normal/Visual; I-beam in Insert
    view.set_overwrite(mode != VimMode::Insert);
    on_mode_change(mode);
}


// ── motion helpers ────────────────────────────────────────────────────────────

fn move_forward_char(buffer: &gtk::TextBuffer) {
    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
    if !iter.ends_line() {
        iter.forward_char();
        buffer.place_cursor(&iter);
    }
}

fn move_to_line_start(buffer: &gtk::TextBuffer) {
    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
    iter.set_line_offset(0);
    buffer.place_cursor(&iter);
}

fn move_to_line_end(buffer: &gtk::TextBuffer) {
    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
    if !iter.ends_line() { iter.forward_to_line_end(); }
    buffer.place_cursor(&iter);
}

fn move_line(buffer: &gtk::TextBuffer, delta: i32) {
    let iter   = buffer.iter_at_mark(&buffer.get_insert());
    let col    = iter.line_offset();
    let line   = iter.line();
    let target = (line + delta).max(0).min(buffer.end_iter().line());
    let mut new_iter = buffer.iter_at_line(target).unwrap_or_else(|| buffer.end_iter());
    let line_len = {
        let mut e = new_iter;
        e.forward_to_line_end();
        e.line_offset()
    };
    new_iter.set_line_offset(col.min(line_len));
    buffer.place_cursor(&new_iter);
}

fn iter_prev_line(buffer: &gtk::TextBuffer, iter: &gtk::TextIter) -> Option<gtk::TextIter> {
    let line = iter.line();
    if line == 0 { return None; }
    buffer.iter_at_line(line - 1)
}


// ── edit helpers ──────────────────────────────────────────────────────────────

fn delete_current_line(buffer: &gtk::TextBuffer) {
    let iter  = buffer.iter_at_mark(&buffer.get_insert());
    let line  = iter.line();
    let mut start = buffer.iter_at_line(line).unwrap_or_else(|| buffer.start_iter());
    let mut end   = start;
    if end.forward_line() {
        buffer.delete(&mut start, &mut end);
    } else {
        end.forward_to_line_end();
        buffer.delete(&mut start, &mut end);
    }
}

fn yank_current_line(buffer: &gtk::TextBuffer, state: &Rc<VimState>) {
    let iter  = buffer.iter_at_mark(&buffer.get_insert());
    let line  = iter.line();
    let start = buffer.iter_at_line(line).unwrap_or_else(|| buffer.start_iter());
    let mut end = start;
    end.forward_to_line_end();
    let text = buffer.text(&start, &end, false).to_string();
    *state.yank.borrow_mut() = Some(text);
}

fn paste_after_line(buffer: &gtk::TextBuffer, state: &Rc<VimState>) {
    let text = state.yank.borrow().clone();
    if let Some(t) = text {
        let iter  = buffer.iter_at_mark(&buffer.get_insert());
        let mut end = iter;
        end.forward_to_line_end();
        buffer.place_cursor(&end);
        buffer.insert_at_cursor(&format!("\n{t}"));
    }
}

fn scroll_to_cursor(view: &TextView) {
    let buffer = view.buffer();
    view.scroll_mark_onscreen(&buffer.get_insert());
}
