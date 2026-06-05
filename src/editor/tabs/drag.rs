use std::{cell::RefCell, rc::Rc};

use gtk::{gdk, glib, prelude::*, Box, Label};

use crate::{
    document::workspace::WorkspaceDocument,
    editor::tabs::active::{mark_active, update_path_bar},
    storage::session::SessionStore,
};

pub fn wire_drag_reorder(
    shell: &Box,
    tab_list: &Rc<RefCell<Vec<Box>>>,
    workspace: &Rc<RefCell<WorkspaceDocument>>,
    subtitle: &Label,
) {
    attach_drag_source(shell);
    attach_drop_target(shell, tab_list, workspace, subtitle);
}

fn attach_drag_source(shell: &Box) {
    let drag_src = gtk::DragSource::new();
    drag_src.set_actions(gdk::DragAction::MOVE);
    drag_src.set_propagation_phase(gtk::PropagationPhase::Bubble);
    {
        let shell_ref = shell.clone();
        drag_src.connect_prepare(move |_, _, _| {
            let val = glib::Value::from(shell_ref.widget_name().as_str());
            Some(gdk::ContentProvider::for_value(&val))
        });
    }
    shell.add_controller(drag_src);
}

fn attach_drop_target(
    shell: &Box,
    tab_list: &Rc<RefCell<Vec<Box>>>,
    workspace: &Rc<RefCell<WorkspaceDocument>>,
    subtitle: &Label,
) {
    let drop_tgt = gtk::DropTarget::new(glib::types::Type::STRING, gdk::DragAction::MOVE);
    let shell_ref = shell.clone();
    let tab_list = tab_list.clone();
    let workspace = workspace.clone();
    let subtitle = subtitle.clone();

    drop_tgt.connect_drop(move |_, value, _, _| {
        let dragged_name = match value.get::<String>() {
            Ok(s) => s,
            Err(_) => return false,
        };

        let (from, to) = {
            let tabs = tab_list.borrow();
            let f = tabs.iter().position(|t| t.widget_name() == dragged_name);
            let t = tabs.iter().position(|t| t == &shell_ref);
            (f, t)
        };

        if let (Some(from), Some(to)) = (from, to) {
            if from == to { return false; }

            {
                let mut tabs = tab_list.borrow_mut();
                let tab = tabs.remove(from);
                tabs.insert(to, tab);
            }

            {
                let mut ws = workspace.borrow_mut();
                let mut order = ws.open_tabs().to_vec();
                let entry = order.remove(from);
                order.insert(to, entry);
                ws.reorder_tabs(order);
            }

            let parent = shell_ref.parent().and_then(|p| p.downcast::<gtk::Box>().ok());
            if let Some(inner) = parent {
                let (moved, anchor) = {
                    let tabs = tab_list.borrow();
                    (tabs[to].clone(), tabs.get(to + 1).cloned())
                };
                inner.reorder_child_after(
                    &moved,
                    anchor.as_ref().map(|w| w.upcast_ref::<gtk::Widget>()),
                );
            }

            let active = workspace.borrow().active_tab_index();
            mark_active(&tab_list, active);
            update_path_bar(&subtitle, &workspace);
            SessionStore::persist(&workspace.borrow());
        }
        true
    });

    shell.add_controller(drop_tgt);
}
