use std::{fs, path::Path};

use crate::{
    document::{node::NoteNode, note::NoteDocument},
    storage::paths::{note_path, notes_dir},
};

pub struct NoteStore;

impl NoteStore {
    /// Loads a note by identifier, reading from `source_path` when given
    /// instead of the managed notes/ directory. `source_path` comes from
    /// NoteDocument::source_path / WorkspaceTab::source_path -- present for
    /// a note opened from outside ~/Tethys-Log/ (CLI, file-manager, or the
    /// Open dialog on a native .tlog file), used when session-restoring
    /// such a tab and when reopening one that was just closed.
    pub fn load(note_identifier: &str, title: &str, source_path: Option<&Path>) -> NoteDocument {
        let raw = match source_path {
            Some(p) => fs::read_to_string(p).unwrap_or_default(),
            None    => fs::read_to_string(note_path(note_identifier)).unwrap_or_default(),
        };

        let mut note = NoteDocument::new(note_identifier.into(), title.into());
        if let Some(p) = source_path {
            note = note.with_source_path(p.to_path_buf());
        }
        note.replace_content(vec![NoteNode::Paragraph(raw)]);
        note
    }

    pub fn persist(note: &NoteDocument) {
        let content: String = note.content_nodes()
            .iter()
            .filter_map(|n| match n {
                NoteNode::Paragraph(p) => Some(p.as_str()),
                NoteNode::Image(_)     => None,
            })
            .collect();

        match note.source_path() {
            // Opened from outside ~/Tethys-Log/ -- save goes to the exact
            // file the user pointed us at, the same way a plain text editor
            // saves a .txt file back where it found it. No title-mirror
            // file either: the note already has a real, user-chosen name
            // and location, so there's nothing for a mirror to add.
            Some(external_path) => Self::persist_external(external_path, &content),
            None                => Self::persist_managed(note, &content),
        }
    }

    fn persist_external(path: &Path, content: &str) {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(path, content);
    }

    fn persist_managed(note: &NoteDocument, content: &str) {
        let path = note_path(note.note_identifier());
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(&path, content);

        // Also keep a title-named file in ~/Tethys-Log/notes/ so the user
        // can see a human-readable filename in their file manager.
        // This file is always written in sync with the UUID file.
        if let Some(tp) = Self::title_path(note.title()) {
            if let Some(dir) = tp.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(tp, content);
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
