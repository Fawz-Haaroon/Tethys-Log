use std::{cell::RefCell, collections::VecDeque, path::Path, rc::Rc};

use gtk::{prelude::*, Box, Button, Label, ScrolledWindow, Stack};

use crate::{
    document::{note::NoteDocument, tab::TabSpec, workspace::WorkspaceDocument},
    editor::{
        tabs::{
            active::{mark_active, update_path_bar, tab_title_btn},
            canvas_store::CanvasStore,
            close::close_tab,
            closed::ClosedTab,
            open::present_tab_ui,
            rename::show_rename_dialog,
            scroll::scroll_tab_into_view,
            widget::ApplyAccentFn,
        },
        workspace_view::apply_vim_mode_to_pill,
    },
    storage::{notes::NoteStore, open as storage_open, session::SessionStore},
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

    fn active_id(&self) -> Option<String> {
        self.workspace.borrow()
            .active_tab()
            .map(|t| t.note_identifier().to_string())
    }

    /// Wires `doc` into the stack and tab strip as the new active tab.
    /// Callers must have already registered its entry in the workspace
    /// model -- see present_tab_ui's doc comment.
    fn present_tab(&self, doc: &NoteDocument) {
        present_tab_ui(
            doc, &self.vim_pill,
            &self.workspace, &self.tab_list, &self.stack, &self.tab_inner,
            &self.tab_scroll, &self.recently_closed, &self.canvases, &self.subtitle,
            self.apply_accent.clone(),
        );
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

        let note = NoteDocument::new(id, title);
        NoteStore::persist(&note);
        self.present_tab(&note);
    }

    pub fn reopen_last_closed(&self) {
        let entry = self.recently_closed.borrow_mut().pop_front();
        if let Some(closed) = entry {
            let note = NoteStore::load(
                &closed.note_identifier, &closed.title, closed.source_path.as_deref(),
            );
            NoteStore::persist(&note);

            self.workspace.borrow_mut().open_tab(
                TabSpec::new(closed.note_identifier.clone(), closed.title.clone())
                    .with_source_path(closed.source_path.clone()),
            );
            self.present_tab(&note);
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

    // ── Opening files ────────────────────────────────────────────────────────

    /// Opens `path` as a tab -- the entry point CLI arguments and
    /// file-manager "Open With" activations reach through app::boot, and
    /// what the Open-file dialog below hands off to as well. A `.tlog`
    /// path that's already open focuses its existing tab instead of
    /// duplicating it (see storage::open for why this dedup only applies
    /// to the native format).
    pub fn open_path(&self, path: &Path) {
        if storage_open::is_native_format(path) {
            let id = storage_open::identifier_for_native_path(path);
            let already_open = self.workspace.borrow()
                .open_tabs().iter()
                .position(|t| t.note_identifier() == id);
            if let Some(index) = already_open {
                self.jump_to(index);
                return;
            }
        }

        match storage_open::open_path(path) {
            Ok(doc) => {
                NoteStore::persist(&doc);
                self.workspace.borrow_mut().open_tab(
                    TabSpec::new(doc.note_identifier().to_string(), doc.title().to_string())
                        .with_source_path(doc.source_path().map(|p| p.to_path_buf())),
                );
                self.present_tab(&doc);
            }
            Err(e) => eprintln!("tethys-log: could not open {}: {e}", path.display()),
        }
    }

    /// Open a native file-chooser dialog and open the selected file as a
    /// new tab -- same dispatch rule as open_path (native .tlog in place,
    /// everything else imported as a managed copy), just reached by
    /// browsing instead of a path the caller already had.
    pub fn open_file_dialog(&self) {
        let parent = self.stack
            .root()
            .and_then(|r| r.downcast::<gtk::Window>().ok());

        let dialog = gtk::FileChooserNative::new(
            Some("Open File"),
            parent.as_ref(),
            gtk::FileChooserAction::Open,
            Some("Open"),
            Some("Cancel"),
        );
        dialog.set_modal(true);

        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Text files"));
        filter.add_pattern("*.tlog");
        filter.add_mime_type("application/x-tethys-log");
        for mime in &["text/plain", "text/markdown", "text/x-rust",
                      "text/x-python", "text/javascript", "text/x-go",
                      "text/x-toml", "text/x-yaml"] {
            filter.add_mime_type(mime);
        }
        dialog.add_filter(&filter);

        // clone the Rc fields needed inside the response callback -- this
        // closure is 'static (owned by the dialog until it's dismissed), so
        // it can't borrow &self; present_tab_ui takes the same fields
        // directly for exactly this reason.
        let workspace       = Rc::clone(&self.workspace);
        let tab_list        = Rc::clone(&self.tab_list);
        let recently_closed = Rc::clone(&self.recently_closed);
        let canvases        = Rc::clone(&self.canvases);
        let stack           = self.stack.clone();
        let tab_inner       = self.tab_inner.clone();
        let tab_scroll      = self.tab_scroll.clone();
        let subtitle        = self.subtitle.clone();
        let vim_pill        = Rc::clone(&self.vim_pill);
        let apply_accent    = self.apply_accent.clone();

        dialog.connect_response(move |d, response| {
            if response != gtk::ResponseType::Accept { return; }

            let path: std::path::PathBuf = match d.file().and_then(|f| f.path()) {
                Some(p) => p,
                None    => return,
            };

            let doc = match storage_open::open_path(&path) {
                Ok(d)  => d,
                Err(e) => {
                    eprintln!("tethys-log: could not open {}: {e}", path.display());
                    return;
                }
            };

            NoteStore::persist(&doc);
            workspace.borrow_mut().open_tab(
                TabSpec::new(doc.note_identifier().to_string(), doc.title().to_string())
                    .with_source_path(doc.source_path().map(|p| p.to_path_buf())),
            );

            present_tab_ui(
                &doc, &vim_pill,
                &workspace, &tab_list, &stack, &tab_inner,
                &tab_scroll, &recently_closed, &canvases, &subtitle,
                apply_accent.clone(),
            );
        });

        dialog.show();
    }
}
