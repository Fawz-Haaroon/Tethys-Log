#[derive(Clone, Debug)]
pub struct WorkspaceTab {
    note_identifier: String,
    title: String,
    // accent color chosen via right-click → Highlight. stored as a CSS hex string
    // so session.rs can round-trip it without a color type dep.
    pub accent: Option<String>,
}

impl WorkspaceTab {
    pub fn new(note_identifier: String, title: String) -> Self {
        Self { note_identifier, title, accent: None }
    }

    pub fn note_identifier(&self) -> &str {
        &self.note_identifier
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn rename(&mut self, new_title: String) {
        self.title = new_title;
    }

    pub fn set_accent(&mut self, color: Option<String>) {
        self.accent = color;
    }
}
