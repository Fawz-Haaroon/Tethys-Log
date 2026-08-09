use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use gtk::{prelude::*, Box, Stack};

use crate::{
    document::workspace::WorkspaceDocument,
    editor::tabs::{
        active::mark_active,
        closed::{ClosedTab, REOPEN_HISTORY_CAP},
    },
    storage::session::SessionStore,
};

pub fn close_tab(
    shell: &Box,
    page_name: &str,
    stack: &Stack,
    tab_list: &Rc<RefCell<Vec<Box>>>,
    workspace: &Rc<RefCell<WorkspaceDocument>>,
    tab_inner: &Box,
    recently_closed: &Rc<RefCell<VecDeque<ClosedTab>>>,
) {
    if workspace.borrow().open_tabs().len() <= 1 {
        return;
    }

    let index = tab_list.borrow().iter().position(|t| t == shell).unwrap_or(0);

    let closed_entry = {
        let ws = workspace.borrow();
        ws.open_tabs().get(index).map(|t| ClosedTab {
            note_identifier: t.note_identifier().to_string(),
            title: t.title().to_string(),
            source_path: t.source_path().map(|p| p.to_path_buf()),
        })
    };
    if let Some(entry) = closed_entry {
        let mut history = recently_closed.borrow_mut();
        history.push_front(entry);
        history.truncate(REOPEN_HISTORY_CAP);
    }

    if let Some(page) = stack.child_by_name(page_name) {
        stack.remove(&page);
    }

    tab_inner.remove(shell);
    tab_list.borrow_mut().retain(|t| t != shell);
    workspace.borrow_mut().close_tab(index);

    let active_name = workspace.borrow()
        .active_tab()
        .map(|t| t.note_identifier().to_string());
    if let Some(name) = active_name {
        stack.set_visible_child_name(&name);
    }
    mark_active(tab_list, workspace.borrow().active_tab_index());
    SessionStore::persist(&workspace.borrow());
}
