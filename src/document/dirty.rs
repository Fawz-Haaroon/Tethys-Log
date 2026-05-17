use std::{cell::Cell, rc::Rc};

// tracks whether a note has unsaved changes relative to what's on disk.
// intentionally not part of NoteDocument — the document doesn't know about
// save state, that's a UI/storage concern. caller owns the Rc.
#[derive(Clone, Debug)]
pub struct DirtyFlag(Rc<Cell<bool>>);

impl DirtyFlag {
    pub fn clean() -> Self {
        Self(Rc::new(Cell::new(false)))
    }

    pub fn mark_dirty(&self) {
        self.0.set(true);
    }

    pub fn mark_clean(&self) {
        self.0.set(false);
    }

    pub fn is_dirty(&self) -> bool {
        self.0.get()
    }
}
