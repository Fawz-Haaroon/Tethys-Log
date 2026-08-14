use std::path::{Path, PathBuf};

use crate::document::node::NoteNode;

#[derive(Debug, Clone)]
pub struct NoteDocument {
    note_identifier: String,
    title: String,
    content_nodes: Vec<NoteNode>,
    // Set when this note mirrors a file living outside ~/Tethys-Log/ --
    // opened via a CLI argument, a file-manager double-click, or the Open
    // dialog on a native .tlog file. When present, storage::notes::NoteStore
    // saves directly to this path instead of the managed notes/ directory.
    // See storage::open for how notes end up with one of these.
    source_path: Option<PathBuf>,
    // Prior states of this note's content, oldest first -- read from the
    // .tlog's history section by NoteStore::load and written back by
    // NoteStore::persist. Empty for a note with no undo history yet, or one
    // saved before undo history existed. See codec::split_document_and_history
    // / encode_document_with_history for the on-disk format, and
    // editor::canvas::history for how entries are captured while editing.
    history: Vec<String>,
}

impl NoteDocument {
    pub fn new(note_identifier: String, title: String) -> Self {
        Self {
            note_identifier,
            title,
            content_nodes: Vec::new(),
            source_path: None,
            history: Vec::new(),
        }
    }

    /// Marks this note as mirroring an external file at `path` -- saves go
    /// there instead of the managed notes/ directory. Consuming builder so
    /// `NoteDocument::new(id, title).with_source_path(p)` reads as one
    /// construction, not a two-step mutation.
    pub fn with_source_path(mut self, path: PathBuf) -> Self {
        self.source_path = Some(path);
        self
    }

    pub fn note_identifier(&self) -> &str {
        &self.note_identifier
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    pub fn replace_content(&mut self, nodes: Vec<NoteNode>) {
        self.content_nodes = nodes;
    }

    pub fn content_nodes(&self) -> &[NoteNode] {
        &self.content_nodes
    }

    pub fn set_history(&mut self, history: Vec<String>) {
        self.history = history;
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }
}
