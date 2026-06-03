use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use gtk::{prelude::*, Box, Button, Label, ScrolledWindow, Stack};

use crate::{
    document::{note::NoteDocument, workspace::WorkspaceDocument},
    editor::{
        canvas::{vim::VimMode, EditorCanvas},
        tabs::{
            active::{mark_active, update_path_bar, tab_title_btn},
            canvas_store::CanvasStore,
            close::close_tab,
            closed::ClosedTab,
            rename::show_rename_dialog,
            scroll::scroll_tab_into_view,
            widget::{build_tab, ApplyAccentFn},
        },
        workspace_view::apply_vim_mode_to_pill,
    },
    storage::{notes::NoteStore, session::SessionStore},
};

pub struct TabController {
    pub workspace:       Rc<RefCell<WorkspaceDocument>>,
    pub tab_list:        Rc<RefCell<Vec<Box>>>,
    pub stack:           Stack,
    pub tab_scroll:      ScrolledWindow,
    pub tab_inner:       Box,
    pub new_tab_btn:     Button,
    pub recently_closed: Rc<RefCell<VecDeque<ClosedTab>>>,
    pub subtitle:        Label,
    pub canvases:        Rc<RefCell<CanvasStore>>,
    pub apply_accent:    ApplyAccentFn,
    pub vim_pill:        Rc<Label>,
}

impl TabController {

    fn make_canvas(&self, note: &NoteDocument) -> EditorCanvas {
        let pill_weak = self.vim_pill.downgrade();
        EditorCanvas::new(note, move |mode: VimMode| {
            if let Some(pill) = pill_weak.upgrade() {
                apply_vim_mode_to_pill(&pill, mode);
            }
        })
    }

    fn active_id(&self) -> Option<String> {
        self.workspace.borrow()
            .active_tab()
            .map(|t| t.note_identifier().to_string())
    }

    // ── Attach file helpers ───────────────────────────────────────────────────

    /// Open image file chooser for the currently active note.
    pub fn attach_image(&self) {
        if let Some(id) = self.active_id() {
            let store = self.canvases.borrow();
            if let Some(canvas) = store.get(&id) {
                canvas.trigger_attach_image();
            }
        }
    }

    /// Open video file chooser for the currently active note.
    pub fn attach_video(&self) {
        if let Some(id) = self.active_id() {
            let store = self.canvases.borrow();
            if let Some(canvas) = store.get(&id) {
                canvas.trigger_attach_video();
            }
        }
    }

    // ── Core tab operations ───────────────────────────────────────────────────

    pub fn close_active(&self) {
        let active = self.workspace.borrow().active_tab_index();
        let shell  = self.tab_list.borrow().get(active).cloned();
        if let Some(shell) = shell {
            let page_name = shell.widget_name().to_string();
            close_tab(
                &shell, &page_name,
                &self.stack, &self.tab_list, &self.workspace,
                &self.tab_inner, &self.recently_closed,
            );
            self.canvases.borrow_mut().remove(&page_name);
            update_path_bar(&self.subtitle, &self.workspace);
        }
    }

    pub fn open_new(&self) {
        let (id, title) = {
            let mut ws = self.workspace.borrow_mut();
            let tab    = ws.create_new_document();
            (tab.note_identifier().to_string(), tab.title().to_string())
        };

        let note   = NoteDocument::new(id.clone(), title.clone());
        NoteStore::persist(&note);

        let canvas = self.make_canvas(&note);
        self.stack.add_named(canvas.widget(), Some(&id));
        self.canvases.borrow_mut().insert(canvas);

        let tab = build_tab(
            &title, id.clone(),
            &self.stack, &self.tab_list, &self.workspace,
            &self.tab_inner, &self.recently_closed, &self.subtitle,
            self.apply_accent.clone(),
        );
        self.tab_inner.append(&tab);
        self.tab_list.borrow_mut().push(tab);

        let new_index = self.workspace.borrow().open_tabs().len() - 1;
        self.workspace.borrow_mut().switch_to(new_index);
        self.stack.set_visible_child_name(&id);
        mark_active(&self.tab_list, new_index);
        update_path_bar(&self.subtitle, &self.workspace);
        scroll_tab_into_view(&self.tab_scroll, &self.tab_list, new_index);
        SessionStore::persist(&self.workspace.borrow());
    }

    pub fn reopen_last_closed(&self) {
        let entry = self.recently_closed.borrow_mut().pop_front();
        if let Some(closed) = entry {
            let note = NoteStore::load(&closed.note_identifier, &closed.title);
            NoteStore::persist(&note);

            let canvas = self.make_canvas(&note);
            self.stack.add_named(canvas.widget(), Some(&closed.note_identifier));
            self.canvases.borrow_mut().insert(canvas);

            self.workspace.borrow_mut().open_tab(
                closed.note_identifier.clone(),
                closed.title.clone(),
            );

            let tab = build_tab(
                &closed.title, closed.note_identifier.clone(),
                &self.stack, &self.tab_list, &self.workspace,
                &self.tab_inner, &self.recently_closed, &self.subtitle,
                self.apply_accent.clone(),
            );
            self.tab_inner.append(&tab);
            self.tab_list.borrow_mut().push(tab);

            let new_index = self.workspace.borrow().open_tabs().len() - 1;
            self.workspace.borrow_mut().switch_to(new_index);
            self.stack.set_visible_child_name(&closed.note_identifier);
            mark_active(&self.tab_list, new_index);
            update_path_bar(&self.subtitle, &self.workspace);
            scroll_tab_into_view(&self.tab_scroll, &self.tab_list, new_index);
            SessionStore::persist(&self.workspace.borrow());
        }
    }

    pub fn toggle_search(&self) {
        if let Some(id) = self.active_id() {
            let store = self.canvases.borrow();
            if let Some(canvas) = store.get(&id) {
                if canvas.search_is_open() { canvas.close_search(); }
                else                       { canvas.open_search();  }
            }
        }
    }

    pub fn cycle_next(&self) {
        let count = self.workspace.borrow().open_tabs().len();
        if count < 2 { return; }
        let next = (self.workspace.borrow().active_tab_index() + 1) % count;
        self.jump_to(next);
    }

    pub fn cycle_prev(&self) {
        let count = self.workspace.borrow().open_tabs().len();
        if count < 2 { return; }
        let active = self.workspace.borrow().active_tab_index();
        self.jump_to(if active == 0 { count - 1 } else { active - 1 });
    }

    pub fn jump_to(&self, index: usize) {
        if index >= self.workspace.borrow().open_tabs().len() { return; }
        self.workspace.borrow_mut().switch_to(index);
        let name = self.workspace.borrow()
            .open_tabs().get(index)
            .map(|t| t.note_identifier().to_string());
        if let Some(name) = name {
            self.stack.set_visible_child_name(&name);
        }
        mark_active(&self.tab_list, index);
        update_path_bar(&self.subtitle, &self.workspace);
        scroll_tab_into_view(&self.tab_scroll, &self.tab_list, index);
        SessionStore::persist(&self.workspace.borrow());

        if let Some(id) = self.active_id() {
            let store = self.canvases.borrow();
            if let Some(canvas) = store.get(&id) {
                apply_vim_mode_to_pill(&self.vim_pill, canvas.vim.mode());
            }
        }
    }

    pub fn rename_active(&self) {
        let active = self.workspace.borrow().active_tab_index();
        let shell  = self.tab_list.borrow().get(active).cloned();
        if let Some(shell) = shell {
            let title_btn = tab_title_btn(&shell);
            let win       = shell.root().and_then(|r| r.downcast::<gtk::Window>().ok());
            if let (Some(btn), Some(w)) = (title_btn, win) {
                let live_title = Rc::new(RefCell::new(
                    btn.label().map(|s| s.to_string()).unwrap_or_default(),
                ));
                show_rename_dialog(
                    &w, &shell, &btn, &live_title,
                    &self.tab_list, &self.workspace, &self.subtitle,
                );
            }
        }
    }
}
