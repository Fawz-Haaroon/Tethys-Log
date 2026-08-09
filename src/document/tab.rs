use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct WorkspaceTab {
    note_identifier: String,
    title: String,
    // accent color chosen via right-click → Highlight. stored as a CSS hex string
    // so session.rs can round-trip it without a color type dep.
    pub accent: Option<String>,
    // Set when this tab mirrors a file outside ~/Tethys-Log/ -- see
    // NoteDocument::source_path for what this changes about saving.
    source_path: Option<PathBuf>,
}

impl WorkspaceTab {
    pub fn new(note_identifier: String, title: String) -> Self {
        Self { note_identifier, title, accent: None, source_path: None }
    }

    pub fn note_identifier(&self) -> &str {
        &self.note_identifier
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    pub fn rename(&mut self, new_title: String) {
        self.title = new_title;
    }

    pub fn set_accent(&mut self, color: Option<String>) {
        self.accent = color;
    }
}

/// Everything needed to open a tab: which note, what to call it, and
/// optional presentation/origin metadata. A builder rather than more
/// `open_tab` parameters -- accent and source_path are both optional and
/// independent, and stacking two more `Option` positional args onto
/// `open_tab` would be unreadable at the call site.
pub struct TabSpec {
    note_identifier: String,
    title: String,
    accent: Option<String>,
    source_path: Option<PathBuf>,
}

impl TabSpec {
    pub fn new(note_identifier: String, title: String) -> Self {
        Self { note_identifier, title, accent: None, source_path: None }
    }

    pub fn with_accent(mut self, accent: Option<String>) -> Self {
        self.accent = accent;
        self
    }

    pub fn with_source_path(mut self, source_path: Option<PathBuf>) -> Self {
        self.source_path = source_path;
        self
    }

    pub(crate) fn into_tab(self) -> WorkspaceTab {
        let mut tab = WorkspaceTab::new(self.note_identifier, self.title);
        tab.accent = self.accent;
        tab.source_path = self.source_path;
        tab
    }
}
