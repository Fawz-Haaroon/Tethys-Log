use std::fs;

use crate::{
    document::{node::NoteNode, note::NoteDocument},
    storage::paths::note_path,
};

pub struct NoteStore;

impl NoteStore {
    pub fn load(note_identifier: &str, title: &str) -> NoteDocument {
        let raw = fs::read_to_string(note_path(note_identifier)).unwrap_or_default();
        let mut note = NoteDocument::new(note_identifier.into(), title.into());
        // raw content may contain \x00img:filename\x00 markers — stored verbatim,
        // surface.rs hands it to deserialise_into_buffer which handles the rest
        note.replace_content(vec![NoteNode::Paragraph(raw)]);
        note
    }

    pub fn persist(note: &NoteDocument) {
        let path = note_path(note.note_identifier());
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        // write the full content string including any \x00img:...\x00 markers
        let content: String = note.content_nodes()
            .iter()
            .filter_map(|n| match n {
                NoteNode::Paragraph(p) => Some(p.as_str()),
                NoteNode::Image(_)     => None,
            })
            .collect();
        let _ = fs::write(path, content);
    }

    pub fn delete(note_identifier: &str) {
        let _ = fs::remove_file(note_path(note_identifier));
    }
}
