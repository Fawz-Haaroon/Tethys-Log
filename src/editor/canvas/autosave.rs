use std::{cell::RefCell, path::PathBuf, rc::Rc};

use gtk::{TextBuffer, prelude::*};
use glib::{ControlFlow, timeout_add_local};

use crate::{
    document::note::NoteDocument,
    document::node::NoteNode,
    editor::canvas::codec::serialize_buffer,
    storage::notes::NoteStore,
};

const AUTOSAVE_QUIET_MS: u64 = 700;

/// `source_path` is `Some` for a note opened from outside `~/Tethys-Log/`
/// (CLI, file manager, or the Open dialog on a native .tlog file) --
/// carrying it through here is what makes autosave write back to that exact
/// file instead of the managed notes/ directory. See NoteDocument::source_path.
pub fn wire_autosave(
    buffer:          &TextBuffer,
    note_identifier: String,
    title:           String,
    source_path:     Option<PathBuf>,
) {
    let generation = Rc::new(RefCell::new(0u64));
    let watched    = buffer.clone();

    buffer.connect_changed(move |_| {
        *generation.borrow_mut() += 1;
        let pending = *generation.borrow();

        let generation      = generation.clone();
        let watched         = watched.clone();
        let note_identifier = note_identifier.clone();
        let title           = title.clone();
        let source_path     = source_path.clone();

        timeout_add_local(std::time::Duration::from_millis(AUTOSAVE_QUIET_MS), move || {
            if *generation.borrow() != pending {
                return ControlFlow::Break;
            }

            // serialize_buffer encodes paintables as sentinel-bracketed
            // markers (e.g. \u{E000}img:filename\u{E000}) so images survive
            // the save/load round-trip
            let content = serialize_buffer(&watched);

            let mut draft = NoteDocument::new(note_identifier.clone(), title.clone());
            if let Some(ref path) = source_path {
                draft = draft.with_source_path(path.clone());
            }
            draft.replace_content(vec![NoteNode::Paragraph(content)]);
            NoteStore::persist(&draft);

            ControlFlow::Break
        });
    });
}
