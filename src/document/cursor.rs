// per-tab cursor and scroll position — restored when switching back to a tab
// or on session reload. stored in memory only; not persisted to disk yet
// because the offset is buffer-relative and survives tab switches fine.
// TODO(2025): persist to session.json so positions survive app restarts too.
#[derive(Clone, Debug, Default)]
pub struct TabViewState {
    pub cursor_offset: u32,
    pub scroll_fraction: f64,
}

impl TabViewState {
    pub fn at_cursor(cursor_offset: u32) -> Self {
        Self { cursor_offset, scroll_fraction: 0.0 }
    }
}
