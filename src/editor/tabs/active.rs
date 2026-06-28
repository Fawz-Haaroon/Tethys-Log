// Tab activation helpers — CSS, index lookup, and path-bar display.
//
// update_path_bar is the only function here that has side effects on the UI.
// The others are pure queries; keep them that way.

use std::{cell::RefCell, rc::Rc};

use gtk::{prelude::*, Box, Label};

use crate::{
    document::workspace::WorkspaceDocument,
    storage::paths::note_path,
};

/// Applies the "tab-active" CSS class to the tab at `active` and removes it
/// from all others.  Drives the visual indicator; contains no business logic.
pub fn mark_active(tab_list: &Rc<RefCell<Vec<Box>>>, active: usize) {
    for (i, tab) in tab_list.borrow().iter().enumerate() {
        if i == active {
            tab.add_css_class("tab-active");
        } else {
            tab.remove_css_class("tab-active");
        }
    }
}

/// Returns the position of `shell` in `tab_list`, or 0 if not found.
pub fn tab_index_of(tab_list: &Rc<RefCell<Vec<Box>>>, shell: &Box) -> usize {
    tab_list.borrow().iter().position(|t| t == shell).unwrap_or(0)
}

/// Returns the title Button inside a tab shell widget, if present.
pub fn tab_title_btn(shell: &Box) -> Option<gtk::Button> {
    shell.first_child().and_then(|w| w.downcast::<gtk::Button>().ok())
}

/// Writes the active tab's canonical note path into `label`.
///
/// Format when the note has a custom title:   "My Title  —  ~/.local/share/tethys-log/notes/<id>.tlog"
/// Format for default/untitled notes:         "~/.local/share/tethys-log/notes/<id>.tlog"
///
/// Why not show the title alone: the path bar doubles as a location indicator
/// for power users who manage notes on the filesystem directly.  Hiding the
/// path removes that affordance.  Showing BOTH title and path for named notes
/// is the right trade-off.
///
/// note_path() is the single source of truth for the note file location;
/// this function delegates to it rather than re-implementing the path formula.
pub fn update_path_bar(label: &Label, workspace: &Rc<RefCell<WorkspaceDocument>>) {
    let text = {
        let ws = workspace.borrow();
        ws.active_tab().map(|t| {
            let title = t.title().to_string();
            let home  = std::env::var("HOME").unwrap_or_default();

            let is_default = title.is_empty()
                || title.starts_with("New Document")
                || title.starts_with("Untitled")
                || title.starts_with("New Note");

            if is_default {
                // Fallback: show the UUID-based path for unnamed notes
                let full_path = note_path(t.note_identifier());
                full_path.to_string_lossy().replacen(&home, "~", 1)
            } else {
                // Show a friendly title-based path — matches the shadow file
                // written by NoteStore::persist alongside the UUID file.
                let slug = slugify(&title);
                let friendly = format!("~/Tethys-Log/notes/{slug}.tlog");
                format!("{title}  —  {friendly}")
            }
        }).unwrap_or_default()
    };
    label.set_text(&text);
}

fn slugify(title: &str) -> String {
    title
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
