// Front door for "the user handed us a filesystem path" -- CLI arguments
// (`tethys-log alya.tlog`), file-manager double-clicks (routed through the
// installed .desktop file's `Exec=... %F`), and the in-app Open-file dialog
// all funnel through open_path() so the three entry points behave the same.
//
// The dispatch rule:
//
//   .tlog, any case  -> opened AND saved in place at the exact path given,
//   (native format)     the same way a plain text editor treats a .txt
//                        file. This is what makes `tethys-log alya.tlog`
//                        behave the way a text-editor user expects, and
//                        what makes a not-yet-existing path work the same
//                        way `nvim newfile.txt` does -- an empty buffer
//                        that writes the file on first save.
//
//   anything else,   -> imported (storage::import): a managed copy is made
//   file exists          and edited from there; the original is never
//                         touched. Editing a foreign file in place isn't
//                         safe here -- attaching an image or video writes
//                         this format's sentinel markers into the buffer,
//                         which would corrupt a .py or .md file the user
//                         expects to stay plain text the moment they used
//                         that feature.
//
//   anything else,   -> nothing to import; starts a new managed note
//   file missing         titled after the path. Not tied to that path,
//                         for the same reason as above.
//
//   a directory      -> rejected. There's no per-folder workspace concept
//                        in Tethys-Log -- everything lives under the single
//                        ~/Tethys-Log/ store (see storage::paths).

use std::{
    hash::{Hash, Hasher},
    path::Path,
};

use crate::{
    document::{id::new_note_id, node::NoteNode, note::NoteDocument},
    storage::import::{self, ImportError},
};

#[derive(Debug)]
pub enum OpenError {
    NotUtf8,
    Io(std::io::Error),
    IsDirectory,
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotUtf8     => write!(f, "file is not valid UTF-8"),
            Self::Io(e)       => write!(f, "io error reading file: {e}"),
            Self::IsDirectory => write!(f, "is a folder -- open a file instead"),
        }
    }
}

impl From<ImportError> for OpenError {
    fn from(e: ImportError) -> Self {
        match e {
            ImportError::NotUtf8 => Self::NotUtf8,
            ImportError::Io(err) => Self::Io(err),
        }
    }
}

/// Opens an arbitrary filesystem path as a note ready to display in a tab.
/// See the module doc for the dispatch rule.
pub fn open_path(path: &Path) -> Result<NoteDocument, OpenError> {
    if path.is_dir() {
        return Err(OpenError::IsDirectory);
    }

    if is_native_format(path) {
        return open_native_in_place(path);
    }

    if path.exists() {
        return Ok(import::import_text_file(path)?);
    }

    Ok(new_untitled_from_name(path))
}

/// True for `.tlog` (case-insensitive) -- Tethys-Log's own format, safe to
/// edit and save directly at the given path. See the module doc.
pub fn is_native_format(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("tlog"))
}

/// The identifier a `.tlog` file at this path opens under. The same
/// absolute path always yields the same id, so re-opening a file that's
/// already open can focus its existing tab instead of duplicating it, and
/// its media directory (storage::paths::images_dir_for) and vim state stay
/// put across the file being closed and reopened -- including across app
/// restarts, since session.json round-trips both the id and the path.
///
/// Only meaningful for native-format paths -- foreign-format imports always
/// get a fresh id (see storage::import), matching the pre-existing
/// behaviour of the Open-file dialog: opening the same .md twice makes two
/// separate notes, because there's no single canonical copy to point back to.
///
/// Scar: the hash comes from std's DefaultHasher, which is fixed-seeded
/// (deterministic across runs of the same binary) but not guaranteed
/// stable across Rust std versions. Rebuilding with a different toolchain
/// could in principle change this id for a path that was already open in a
/// saved session. That's harmless for the note's content -- session.json
/// stores the real path too, so the text always reloads correctly -- the
/// only casualty would be an orphaned media directory for a rebuilt-and
/// -reopened external note that had images or videos attached. Accepted as
/// a rare, low-severity edge case rather than a reason to pull in a hashing
/// dependency for a single-user local app.
pub fn identifier_for_native_path(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    canonical.hash(&mut hasher);
    format!("ext-{:016x}", hasher.finish())
}

fn open_native_in_place(path: &Path) -> Result<NoteDocument, OpenError> {
    let raw = if path.exists() {
        std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::InvalidData { OpenError::NotUtf8 } else { OpenError::Io(e) }
        })?
    } else {
        String::new()
    };

    let title = file_name_or(path, "Untitled.tlog");
    let id    = identifier_for_native_path(path);

    let mut doc = NoteDocument::new(id, title).with_source_path(path.to_path_buf());
    doc.replace_content(vec![NoteNode::Paragraph(raw)]);
    Ok(doc)
}

/// A brand-new managed note, titled after a path that doesn't exist yet.
/// Not tied to that path -- see the module doc for why foreign formats
/// aren't edited in place.
fn new_untitled_from_name(path: &Path) -> NoteDocument {
    NoteDocument::new(new_note_id(), file_name_or(path, "Untitled"))
}

fn file_name_or(path: &Path, fallback: &str) -> String {
    path.file_name().and_then(|n| n.to_str()).unwrap_or(fallback).to_string()
}
