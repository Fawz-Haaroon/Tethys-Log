use std::fs;

use crate::{
    document::{node::NoteNode, note::NoteDocument},
    storage::paths::{note_path, notes_dir},
};

pub struct NoteStore;

impl NoteStore {
    pub fn load(note_identifier: &str, title: &str) -> NoteDocument {
        let raw = fs::read_to_string(note_path(note_identifier)).unwrap_or_default();
        let mut note = NoteDocument::new(note_identifier.into(), title.into());
        note.replace_content(vec![NoteNode::Paragraph(raw)]);
        note
    }

    pub fn persist(note: &NoteDocument) {
        let path = note_path(note.note_identifier());
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let content: String = note.content_nodes()
            .iter()
            .filter_map(|n| match n {
                NoteNode::Paragraph(p) => Some(p.as_str()),
                NoteNode::Image(_)     => None,
            })
            .collect();
        let _ = fs::write(&path, &content);

        // Also keep a title-named file in ~/Tethys-Log/notes/ so the user
        // can see a human-readable filename in their file manager.
        // This file is always written in sync with the UUID file.
        if let Some(tp) = Self::title_path(note.title()) {
            if let Some(dir) = tp.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(tp, &content);
        }
    }

    pub fn delete(note_identifier: &str) {
        let _ = fs::remove_file(note_path(note_identifier));
    }

    /// Removes the title-named file for `old_title` before a rename so the
    /// old name disappears from the file manager immediately.
    pub fn cleanup_title_file(old_title: &str) {
        if let Some(tp) = Self::title_path(old_title) {
            let _ = fs::remove_file(tp);
        }
    }

    // ── internal helpers ──────────────────────────────────────────────────────

    fn title_path(title: &str) -> Option<std::path::PathBuf> {
        let slug = Self::slugify(title);
        if slug.is_empty() { return None; }
        // Don't create a shadow file for generic default names — only when the
        // user has given the note a real custom name.
        let lower = slug.as_str();
        if lower.starts_with("new-document")
            || lower.starts_with("untitled")
            || lower.starts_with("new-note")
        {
            return None;
        }
        Some(notes_dir().join(format!("{slug}.tlog")))
    }

    /// Convert a title into a filesystem-safe slug.
    /// e.g. "Ancient Machine Notes!" → "ancient-machine-notes"
    fn slugify(title: &str) -> String {
        title
            .chars()
            .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}
