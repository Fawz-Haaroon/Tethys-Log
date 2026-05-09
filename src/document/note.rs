use crate::document::node::NoteNode;

#[derive(Debug, Clone)]
pub struct NoteDocument {
    note_identifier: String,
    title: String,
    content_nodes: Vec<NoteNode>,
}

impl NoteDocument {
    pub fn new(note_identifier: String, title: String) -> Self {
        Self { note_identifier, title, content_nodes: Vec::new() }
    }

    pub fn note_identifier(&self) -> &str {
        &self.note_identifier
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn replace_content(&mut self, nodes: Vec<NoteNode>) {
        self.content_nodes = nodes;
    }

    pub fn content_nodes(&self) -> &[NoteNode] {
        &self.content_nodes
    }
}
