use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    document::{tab::TabSpec, workspace::WorkspaceDocument},
    storage::paths::session_path,
};

#[derive(Serialize, Deserialize)]
struct SessionSnapshot {
    tabs: Vec<TabEntry>,
    active_index: usize,
}

#[derive(Serialize, Deserialize)]
struct TabEntry {
    note_identifier: String,
    title: String,
    #[serde(default)]
    accent: Option<String>,
    // Present for tabs opened from outside ~/Tethys-Log/ (CLI, file
    // manager, or the Open dialog on a native .tlog file). `#[serde(default)]`
    // so session.json files written before this field existed still load.
    #[serde(default)]
    source_path: Option<PathBuf>,
}

pub struct SessionStore;

impl SessionStore {
    pub fn load() -> WorkspaceDocument {
        let mut workspace = WorkspaceDocument::empty();

        let saved = fs::read_to_string(session_path())
            .ok()
            .and_then(|raw| serde_json::from_str::<SessionSnapshot>(&raw).ok());

        match saved {
            Some(s) if !s.tabs.is_empty() => {
                for entry in s.tabs {
                    workspace.open_tab(
                        TabSpec::new(entry.note_identifier, entry.title)
                            .with_accent(entry.accent)
                            .with_source_path(entry.source_path),
                    );
                }
                workspace.set_active_tab_index(s.active_index);
                workspace.sync_untitled_counter();
            }
            _ => {
                workspace.create_new_document();
            }
        }

        workspace
    }

    pub fn persist(workspace: &WorkspaceDocument) {
        let snapshot = SessionSnapshot {
            tabs: workspace
                .open_tabs()
                .iter()
                .map(|t| TabEntry {
                    note_identifier: t.note_identifier().to_string(),
                    title: t.title().to_string(),
                    accent: t.accent.clone(),
                    source_path: t.source_path().map(|p| p.to_path_buf()),
                })
                .collect(),
            active_index: workspace.active_tab_index(),
        };

        if let Ok(json) = serde_json::to_string(&snapshot) {
            let path = session_path();
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(path, json);
        }
    }
}
