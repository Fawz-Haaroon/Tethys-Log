use std::path::PathBuf;

// ClosedTab lives here so both close.rs and controller.rs can reference it
// without either owning it. the VecDeque cap of 10 is arbitrary but sane —
// nobody needs to reopen their 11th-most-recent tab.
#[derive(Clone)]
pub struct ClosedTab {
    pub note_identifier: String,
    pub title: String,
    // Carried over from WorkspaceTab::source_path so reopening a closed tab
    // that was opened from outside ~/Tethys-Log/ reloads from the right
    // place instead of an empty managed-storage slot.
    pub source_path: Option<PathBuf>,
}

pub const REOPEN_HISTORY_CAP: usize = 10;
