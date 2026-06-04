// ClosedTab lives here so both close.rs and controller.rs can reference it
// without either owning it. the VecDeque cap of 10 is arbitrary but sane —
// nobody needs to reopen their 11th-most-recent tab.
#[derive(Clone)]
pub struct ClosedTab {
    pub note_identifier: String,
    pub title: String,
}

pub const REOPEN_HISTORY_CAP: usize = 10;
