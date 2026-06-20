// Canonical filesystem paths for all Tethys-Log data.
//
// ~/Tethys-Log/ is a plain, visible folder in the user's home directory.
// Every module that needs a file location calls a function here.
// No other module constructs a Tethys-Log path by hand.
//
// Folder layout:
//   ~/Tethys-Log/
//   ├── notes/          native .tlog files created in the app
//   ├── imports/        copies of external files opened in the app
//   ├── drafts/         unsaved / in-progress notes
//   ├── media/
//   │   ├── images/     images attached to notes, grouped by note id
//   │   └── videos/     videos attached to notes, grouped by note id
//   └── session.json    open tabs and active-tab index

use std::path::PathBuf;

/// Root of all Tethys-Log data: ~/Tethys-Log/
///
/// A plain, visible home-directory folder — users can open, copy, and back
/// up their notes like any other files.  No XDG indirection here because
/// Tethys-Log data is first-class user content, not application cache.
pub fn storage_root() -> PathBuf {
    home_dir().join("Tethys-Log")
}

/// ~/Tethys-Log/notes/ — native notes created inside the app.
pub fn notes_dir() -> PathBuf {
    storage_root().join("notes")
}

/// ~/Tethys-Log/imports/ — copies of external files the user opened.
pub fn imports_dir() -> PathBuf {
    storage_root().join("imports")
}

/// ~/Tethys-Log/drafts/ — unsaved / in-progress notes.
pub fn drafts_dir() -> PathBuf {
    storage_root().join("drafts")
}

/// ~/Tethys-Log/media/images/<note_id>/ — images for a specific note.
pub fn images_dir_for(note_identifier: &str) -> PathBuf {
    storage_root().join("media").join("images").join(note_identifier)
}

/// ~/Tethys-Log/media/videos/<note_id>/ — videos for a specific note.
pub fn videos_dir_for(note_identifier: &str) -> PathBuf {
    storage_root().join("media").join("videos").join(note_identifier)
}

/// Returns the canonical path for a native note file.
///
/// All note-related code goes through this function — the extension and
/// directory layout are defined here and nowhere else.
pub fn note_path(note_identifier: &str) -> PathBuf {
    notes_dir().join(format!("{note_identifier}.tlog"))
}

/// Returns the path for an imported (foreign-file copy) note.
pub fn import_path(note_identifier: &str) -> PathBuf {
    imports_dir().join(format!("{note_identifier}.tlog"))
}

/// Returns the path for an unsaved draft note.
pub fn draft_path(note_identifier: &str) -> PathBuf {
    drafts_dir().join(format!("{note_identifier}.tlog"))
}

/// Returns the path of the session file (open tabs, active tab index).
pub fn session_path() -> PathBuf {
    storage_root().join("session.json")
}

/// Kept for callers that need the storage root without a specific subdir.
/// Prefer the typed helpers (notes_dir, imports_dir, etc.) over this.
pub fn data_dir() -> PathBuf {
    storage_root()
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
