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
}

impl NoteDocument {
    pub fn new(note_identifier: String, title: String) -> Self {
        Self { note_identifier, title, content_nodes: Vec::new(), source_path: None }
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
}
