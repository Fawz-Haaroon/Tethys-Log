// DocumentHistory — Tethys-Log's own linear undo/redo, independent of GTK.
//
// GTK4's TextBuffer undo only round-trips raw codepoints. Verified directly
// against the real API: a TextTag applied to any text -- highlight,
// img-path, video-path, embed-src, all of them -- is gone after a delete
// followed by buffer.undo(), even though the character itself comes back.
// There's no repairing that from outside the buffer; by the time undo()
// returns, the information about what the text meant is simply not there
// anymore. That's what embed_undo.rs was patching around, and why deleting
// an embedded image/video and undoing left an unreadable orphan character
// instead of the widget -- its premise (GTK restores the tag, just not the
// widget) doesn't hold.
//
// So instead of asking GTK to remember what a run of text meant, Tethys-Log
// keeps its own stack of full document snapshots -- the exact same
// serialized text codec.rs already round-trips correctly for save/load,
// reused here for undo/redo. An undo step doesn't repair anything; it just
// re-runs the load path against an earlier snapshot. Text, highlights, and
// media all come back because the load path was already correct -- undo
// stops being a special case.
//
// Redo is intentionally NOT persisted across a close/reopen -- see
// persisted_snapshots below. Undo is: that's the part of "any point in
// history" that was actually asked for, and persisting redo too would mean
// carrying a second ordered stack with its own invalidate-on-new-edit rule
// for a property most editors don't offer and nobody asked for here.

use std::{cell::RefCell, collections::VecDeque, path::Path, rc::Rc};

use gtk::{TextBuffer, TextView};

use crate::editor::canvas::codec::deserialise_into_buffer;

/// How many prior states of a note are kept. Generous on purpose -- each
/// entry is a serialized text string (KBs; the media it references lives in
/// the note's asset directories as it always has, never duplicated here) --
/// but bounded, because "keep every state forever" on a note edited for
/// years is an unbounded-disk-growth risk, not a feature anyone asked for.
pub const MAX_HISTORY_SNAPSHOTS_PER_NOTE: usize = 300;

pub struct DocumentHistory {
    past:     VecDeque<String>, // oldest..newest; back() is the most recent prior state
    future:   Vec<String>,      // redo stack, this session only
    baseline: String,           // the document's current settled state
}

impl DocumentHistory {
    /// `initial` is the content the buffer was just loaded with. `persisted_past`
    /// is whatever history NoteStore::load found already saved in the .tlog,
    /// oldest first -- the same order it's kept in here.
    pub fn new(initial: String, mut persisted_past: Vec<String>) -> Self {
        if persisted_past.len() > MAX_HISTORY_SNAPSHOTS_PER_NOTE {
            let excess = persisted_past.len() - MAX_HISTORY_SNAPSHOTS_PER_NOTE;
            persisted_past.drain(..excess);
        }
        Self {
            past:     persisted_past.into(),
            future:   Vec::new(),
            baseline: initial,
        }
    }

    /// Called once per debounced "the document has settled" event (see
    /// autosave.rs, which owns that timing) with the buffer's current
    /// serialized content. A no-op when `current` matches the baseline --
    /// which is exactly what happens right after undo()/redo() fire, since
    /// they already moved the baseline synchronously. That equality check
    /// is the whole reason a separate "don't record my own undo" flag isn't
    /// needed here.
    pub fn record_settled_state(&mut self, current: String) {
        if current == self.baseline {
            return;
        }
        self.past.push_back(std::mem::replace(&mut self.baseline, current));
        if self.past.len() > MAX_HISTORY_SNAPSHOTS_PER_NOTE {
            self.past.pop_front();
        }
        self.future.clear();
    }

    /// Moves one step back and returns the snapshot to load, or None at the
    /// start of history.
    fn step_back(&mut self) -> Option<String> {
        let previous = self.past.pop_back()?;
        self.future.push(std::mem::replace(&mut self.baseline, previous.clone()));
        Some(previous)
    }

    /// Moves one step forward and returns the snapshot to load, or None if
    /// nothing's been undone this session.
    fn step_forward(&mut self) -> Option<String> {
        let next = self.future.pop()?;
        self.past.push_back(std::mem::replace(&mut self.baseline, next.clone()));
        if self.past.len() > MAX_HISTORY_SNAPSHOTS_PER_NOTE {
            self.past.pop_front();
        }
        Some(next)
    }

    /// The undo stack, oldest first -- what NoteStore::persist writes back
    /// into the .tlog's history section on the next save.
    pub fn persisted_snapshots(&self) -> Vec<String> {
        self.past.iter().cloned().collect()
    }
}


// ── GTK-facing entry points ─────────────────────────────────────────────────
//
// The buffer swap itself lives here rather than on DocumentHistory so the
// undo/redo stack logic above stays plain data -- reasoned about (and
// tested) without a live GTK buffer. Same split codec.rs already draws
// between plain string handling and its GTK insertion helpers.

pub fn undo(history: &Rc<RefCell<DocumentHistory>>, buffer: &TextBuffer, view: &TextView, image_dir: &Path) {
    if let Some(snapshot) = history.borrow_mut().step_back() {
        deserialise_into_buffer(&snapshot, buffer, view, image_dir);
    }
}

pub fn redo(history: &Rc<RefCell<DocumentHistory>>, buffer: &TextBuffer, view: &TextView, image_dir: &Path) {
    if let Some(snapshot) = history.borrow_mut().step_forward() {
        deserialise_into_buffer(&snapshot, buffer, view, image_dir);
    }
}
