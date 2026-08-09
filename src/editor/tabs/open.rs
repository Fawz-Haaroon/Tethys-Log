// Shared "wire a NoteDocument into the tab strip and make it active"
// sequence -- a free function rather than a TabController method because
// one caller needs it: FileChooserNative::connect_response takes a 'static
// closure that only has the individual Rc fields it captured, not
// `&TabController` (the same constraint close_tab and build_tab already
// work under). TabController::present_tab is a thin `&self` wrapper around
// this for the other three callers, who don't have that constraint.

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use gtk::{prelude::*, Box, Label, ScrolledWindow, Stack};

use crate::{
    document::{note::NoteDocument, workspace::WorkspaceDocument},
    editor::{
        canvas::{vim::VimMode, EditorCanvas},
        tabs::{
            active::{mark_active, update_path_bar},
            canvas_store::CanvasStore,
            closed::ClosedTab,
            scroll::scroll_tab_into_view,
            widget::{build_tab, ApplyAccentFn},
        },
        workspace_view::apply_vim_mode_to_pill,
    },
    storage::session::SessionStore,
};

/// Builds the EditorCanvas for `doc`, wiring its vim-mode changes to the
/// status-bar pill. The one piece of canvas construction every call site
/// needs, pulled out so it's written once instead of three times over.
pub fn build_canvas(doc: &NoteDocument, vim_pill: &Rc<Label>) -> EditorCanvas {
    let pill_weak = vim_pill.downgrade();
    EditorCanvas::new(doc, move |mode: VimMode| {
        if let Some(pill) = pill_weak.upgrade() {
            apply_vim_mode_to_pill(&pill, mode);
        }
    })
}

/// Registers `doc`'s canvas in the stack and tab strip, focuses it, and
/// persists the session. Callers must have already registered the tab's
/// entry in the workspace model (`WorkspaceDocument::open_tab` or
/// `create_new_document`) before calling this -- it wires up what the
/// model entry describes, it does not create the entry itself.
#[allow(clippy::too_many_arguments)]
pub fn present_tab_ui(
    doc:             &NoteDocument,
    vim_pill:        &Rc<Label>,
    workspace:       &Rc<RefCell<WorkspaceDocument>>,
    tab_list:        &Rc<RefCell<Vec<Box>>>,
    stack:           &Stack,
    tab_inner:       &Box,
    tab_scroll:      &ScrolledWindow,
    recently_closed: &Rc<RefCell<VecDeque<ClosedTab>>>,
    canvases:        &Rc<RefCell<CanvasStore>>,
    subtitle:        &Label,
    apply_accent:    ApplyAccentFn,
) {
    let id    = doc.note_identifier().to_string();
    let title = doc.title().to_string();

    let canvas = build_canvas(doc, vim_pill);
    stack.add_named(canvas.widget(), Some(&id));
    canvases.borrow_mut().insert(canvas);

    let tab = build_tab(
        &title, id.clone(),
        stack, tab_list, workspace,
        tab_inner, recently_closed, subtitle,
        apply_accent,
    );
    tab_inner.append(&tab);
    tab_list.borrow_mut().push(tab);

    let new_index = workspace.borrow().open_tabs().len() - 1;
    workspace.borrow_mut().switch_to(new_index);
    stack.set_visible_child_name(&id);
    mark_active(tab_list, new_index);
    update_path_bar(subtitle, workspace);
    scroll_tab_into_view(tab_scroll, tab_list, new_index);
    SessionStore::persist(&workspace.borrow());
}
