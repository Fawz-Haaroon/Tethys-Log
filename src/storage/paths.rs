// Canonical filesystem paths for all Tethys-Log data.
//
// Every module that needs a file location calls a function here.
// No other module should construct a `tethys-log` path by hand —
// doing so scatters the layout definition across the codebase and makes
// moving the data directory a multi-file change.

use std::path::PathBuf;

/// Returns the root data directory: `$XDG_DATA_HOME/tethys-log` when the
/// variable is set, otherwise `~/.local/share/tethys-log`.
///
/// XDG_DATA_HOME is the user's explicit override; honouring it is required
/// for correct behaviour in containers, CI, and multi-user setups where the
/// default home path cannot be assumed.
pub fn data_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join(".local").join("share"));
    base.join("tethys-log")
}

/// Returns the canonical path for a note file.
///
/// The extension `.tlog` and the `notes/` subdirectory are defined here and
/// nowhere else.  All callers use this function — the path formula has exactly
/// one definition.
pub fn note_path(note_identifier: &str) -> PathBuf {
    data_dir().join("notes").join(format!("{note_identifier}.tlog"))
}

/// Returns the path of the session file (open tabs, active tab index).
pub fn session_path() -> PathBuf {
    data_dir().join("session.json")
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
