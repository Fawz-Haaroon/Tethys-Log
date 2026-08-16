use std::{fs, io, path::Path};

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

        // A note saved before undo history existed has no history section at
        // all -- split_document_and_history returns it unchanged as `current`
        // with an empty history, so it opens exactly as it always did.
        let (current, history) = split_document_and_history(&raw);

        let mut note = NoteDocument::new(note_identifier.into(), title.into());
        if let Some(p) = source_path {
            note = note.with_source_path(p.to_path_buf());
        }
        note.replace_content(vec![NoteNode::Paragraph(current)]);
        note.set_history(history);
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

        let on_disk = encode_document_with_history(&content, note.history());

        match note.source_path() {
            // Opened from outside ~/Tethys-Log/ -- save goes to the exact
            // file the user pointed us at, the same way a plain text editor
            // saves a .txt file back where it found it. No title-mirror
            // file either: the note already has a real, user-chosen name
            // and location, so there's nothing for a mirror to add.
            Some(external_path) => Self::persist_external(external_path, &on_disk),
            None                => Self::persist_managed(note, &on_disk),
        }
    }

    fn persist_external(path: &Path, on_disk: &str) {
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = write_atomically(path, on_disk);
    }

    fn persist_managed(note: &NoteDocument, on_disk: &str) {
        let path = note_path(note.note_identifier());
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = write_atomically(&path, on_disk);

        // Also keep a title-named file in ~/Tethys-Log/notes/ so the user
        // can see a human-readable filename in their file manager.
        // This file is always written in sync with the UUID file, history
        // section included -- if the user opens the mirror instead of the
        // UUID file (e.g. double-clicking it in their file manager), it
        // needs the same undo depth the canonical copy has.
        if let Some(tp) = Self::title_path(note.title()) {
            if let Some(dir) = tp.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = write_atomically(&tp, on_disk);
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

/// Writes `content` to `path` via write-temp-then-rename instead of a direct
/// fs::write. A direct write truncates the target file before the new bytes
/// land -- if the process dies mid-write (crash, OOM kill, power loss), the
/// note is left partially overwritten, not merely stale. Writing to a temp
/// file in the same directory (so the rename stays on one filesystem, which
/// is what makes it atomic) and renaming over the target means the target
/// is either the old complete file or the new complete file, never a
/// half-written one.
fn write_atomically(path: &Path, content: &str) -> io::Result<()> {
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no file name")
    })?;
    let tmp_path = path.with_file_name(format!("{}.tmp", file_name.to_string_lossy()));
    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, path)
}


// ── Undo history framing ────────────────────────────────────────────────────
//
// A .tlog file on disk is the current document text (whatever
// editor::canvas::codec's serialize_buffer produced -- this layer never
// looks inside it) and -- optionally -- a trailing log of prior states for
// editor::canvas::history to undo back into. The two are separated by
// HISTORY_MARKER, a Private Use Area sentinel that never appears in
// ordinary serialized text (same scheme as codec.rs's img/embed/video/
// highlight markers, for the same reason: valid, unremarkable UTF-8 text
// rather than a control byte that makes `file`/`git`/GitHub call the whole
// file binary). A note is read up to the first HISTORY_MARKER, or EOF if
// there isn't one -- which is exactly every note saved before undo history
// existed, so those keep opening completely unchanged.
//
// After the marker, each historical entry is framed as
// `<decimal byte length>\n<that many bytes of text>`, back to back with no
// separator between entries -- the length alone tells the reader exactly
// where one entry ends and the next begins, so a snapshot's own content can
// contain anything (newlines, digits, other sentinels) with no escaping.
// This is the same length-prefixed framing behind things like Bencode or
// Redis's RESP protocol; nothing novel, just boring and correct.
const HISTORY_MARKER: char = '\u{E005}';

/// Splits raw .tlog file text into (current document, prior states oldest
/// first). A malformed or truncated history tail is stopped at rather than
/// panicked on -- the current document before the marker is always well
/// formed on its own, so the worst a corrupt history section can do is lose
/// some undo depth, never the note itself.
///
/// pub(crate) rather than private: storage::open reads a native .tlog file
/// directly (CLI argument, file-manager double-click, the in-app Open
/// dialog) rather than through NoteStore::load, and needs the exact same
/// split -- otherwise the history section reads back as literal document
/// text, which is its own kind of corruption. One parser for the format,
/// two legitimate callers.
pub(crate) fn split_document_and_history(raw: &str) -> (String, Vec<String>) {
    let Some(marker_at) = raw.find(HISTORY_MARKER) else {
        return (raw.to_string(), Vec::new());
    };

    let current = raw[..marker_at].to_string();
    let mut rest = &raw[marker_at + HISTORY_MARKER.len_utf8()..];
    let mut history = Vec::new();

    while !rest.is_empty() {
        let Some(newline_at) = rest.find('\n') else { break };
        let Ok(len) = rest[..newline_at].parse::<usize>() else { break };

        let body_start = newline_at + 1;
        if rest.len() < body_start + len { break }

        history.push(rest[body_start..body_start + len].to_string());
        rest = &rest[body_start + len..];
    }

    (current, history)
}

/// The inverse of split_document_and_history: encodes `current` plus
/// `history` (oldest first) back into the on-disk format. A note with no
/// history yet round-trips as exactly `current` with no marker at all, so
/// this stays byte-for-byte the pre-history format until a note actually
/// has an undo step to remember.
fn encode_document_with_history(current: &str, history: &[String]) -> String {
    if history.is_empty() {
        return current.to_string();
    }

    let mut out = String::with_capacity(current.len() + history.iter().map(String::len).sum::<usize>());
    out.push_str(current);
    out.push(HISTORY_MARKER);
    for entry in history {
        out.push_str(&entry.len().to_string());
        out.push('\n');
        out.push_str(entry);
    }
    out
}
