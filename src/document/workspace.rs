use crate::document::{id::new_note_id, tab::WorkspaceTab};

pub struct WorkspaceDocument {
    open_tabs: Vec<WorkspaceTab>,
    active_tab_index: usize,
}

impl WorkspaceDocument {
    pub fn empty() -> Self {
        Self { open_tabs: Vec::new(), active_tab_index: 0 }
    }

    pub fn open_tab(&mut self, note_identifier: String, title: String) {
        self.open_tabs.push(WorkspaceTab::new(note_identifier, title));
    }

    pub fn open_tab_with_accent(
        &mut self,
        note_identifier: String,
        title: String,
        accent: Option<String>,
    ) {
        let mut tab = WorkspaceTab::new(note_identifier, title);
        tab.set_accent(accent);
        self.open_tabs.push(tab);
    }

    pub fn create_new_document(&mut self) -> &WorkspaceTab {
        self.open_tabs.push(WorkspaceTab::new(new_note_id(), "New Document".into()));
        self.active_tab_index = self.open_tabs.len() - 1;
        self.open_tabs.last().unwrap()
    }

    pub fn close_tab(&mut self, index: usize) {
        if index >= self.open_tabs.len() {
            return;
        }
        self.open_tabs.remove(index);
        if !self.open_tabs.is_empty() && self.active_tab_index >= self.open_tabs.len() {
            self.active_tab_index = self.open_tabs.len() - 1;
        }
    }

    pub fn switch_to(&mut self, index: usize) {
        if index < self.open_tabs.len() {
            self.active_tab_index = index;
        }
    }

    pub fn rename_tab(&mut self, index: usize, new_title: String) {
        if let Some(tab) = self.open_tabs.get_mut(index) {
            tab.rename(new_title);
        }
    }

    pub fn set_tab_accent(&mut self, index: usize, color: Option<String>) {
        if let Some(tab) = self.open_tabs.get_mut(index) {
            tab.set_accent(color);
        }
    }

    pub fn reorder_tabs(&mut self, new_order: Vec<WorkspaceTab>) {
        let active_id = self.open_tabs
            .get(self.active_tab_index)
            .map(|t| t.note_identifier().to_string());
        self.open_tabs = new_order;
        if let Some(id) = active_id {
            if let Some(pos) = self.open_tabs.iter().position(|t| t.note_identifier() == id) {
                self.active_tab_index = pos;
            }
        }
    }

    pub fn open_tabs(&self) -> &[WorkspaceTab] {
        &self.open_tabs
    }

    pub fn active_tab_index(&self) -> usize {
        self.active_tab_index
    }

    pub fn active_tab(&self) -> Option<&WorkspaceTab> {
        self.open_tabs.get(self.active_tab_index)
    }

    pub fn set_active_tab_index(&mut self, index: usize) {
        if index < self.open_tabs.len() {
            self.active_tab_index = index;
        }
    }

    // called on session restore to avoid "New Document (2)" collisions
    pub fn sync_untitled_counter(&mut self) {}
}
