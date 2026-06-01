use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use gtk::{prelude::*, Box, Button, Label, Orientation, ScrolledWindow, Stack};

use crate::{
    document::workspace::WorkspaceDocument,
    editor::{
        canvas::EditorCanvas,
        tabs::{
            active::mark_active,
            canvas_store::CanvasStore,
            closed::ClosedTab,
            controller::TabController,
            scroll::scroll_arrow_btn,
            widget::{build_tab, ApplyAccentFn, TAB_WIDTH},
        },
        workspace_view::apply_vim_mode_to_pill,
    },
    storage::{notes::NoteStore, session::SessionStore},
};

pub struct TabBar {
    pub strip:  Box,
    controller: Rc<TabController>,
}

impl TabBar {
    pub fn new(
        workspace: Rc<RefCell<WorkspaceDocument>>,
        stack:     Stack,
        subtitle:  Label,
        vim_pill:  Label,
    ) -> Self {
        let tab_inner = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .build();

        let tab_scroll = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(false)
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Never)
            .build();
        tab_scroll.set_child(Some(&tab_inner));

        let scroll_left  = scroll_arrow_btn("‹");
        let scroll_right = scroll_arrow_btn("›");

        {
            let s = tab_scroll.clone();
            scroll_left.connect_clicked(move |_| {
                let adj = s.hadjustment();
                adj.set_value((adj.value() - TAB_WIDTH as f64).max(adj.lower()));
            });
        }
        {
            let s = tab_scroll.clone();
            scroll_right.connect_clicked(move |_| {
                let adj = s.hadjustment();
                adj.set_value((adj.value() + TAB_WIDTH as f64).min(adj.upper() - adj.page_size()));
            });
        }

        let new_tab_btn = Button::builder().label("+").build();
        new_tab_btn.add_css_class("new-tab-btn");

        let strip = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .hexpand(true)
            .build();
        strip.add_css_class("tab-row");
        strip.append(&scroll_left);
        strip.append(&tab_scroll);
        strip.append(&new_tab_btn);
        strip.append(&scroll_right);

        let tab_list:        Rc<RefCell<Vec<Box>>>               = Rc::new(RefCell::new(Vec::new()));
        let recently_closed: Rc<RefCell<VecDeque<ClosedTab>>>    = Rc::new(RefCell::new(VecDeque::new()));
        let canvases:        Rc<RefCell<CanvasStore>>            = Rc::new(RefCell::new(CanvasStore::new()));
        let vim_pill         = Rc::new(vim_pill);

        let active_index = workspace.borrow().active_tab_index();
        let tab_count    = workspace.borrow().open_tabs().len();

        let apply_accent: ApplyAccentFn = {
            let workspace = workspace.clone();
            let tab_list  = tab_list.clone();
            Rc::new(move |shell: &Box, color: Option<String>| {
                let index = tab_list.borrow().iter().position(|t| t == shell).unwrap_or(0);
                workspace.borrow_mut().set_tab_accent(index, color.clone());
                for c in shell.css_classes() {
                    if c.starts_with("tab-accent-") { shell.remove_css_class(&c); }
                }
                if let Some(ref c) = color {
                    shell.add_css_class(&format!("tab-accent-{}", c.trim_start_matches('#')));
                }
                SessionStore::persist(&workspace.borrow());
            })
        };

        // build_canvas: constructs an EditorCanvas wired to the vim-pill.
        // vim_pill is Rc so we clone the Rc cheaply on each call and downgrade inside.
        let make_canvas = |note: &crate::document::note::NoteDocument| -> EditorCanvas {
            let pill_weak = vim_pill.downgrade();
            EditorCanvas::new(note, move |mode| {
                if let Some(pill) = pill_weak.upgrade() {
                    apply_vim_mode_to_pill(&pill, mode);
                }
            })
        };

        for i in 0..tab_count {
            let (id, title, accent) = {
                let ws = workspace.borrow();
                let t  = &ws.open_tabs()[i];
                (t.note_identifier().to_string(), t.title().to_string(), t.accent.clone())
            };

            let note   = NoteStore::load(&id, &title);
            NoteStore::persist(&note);
            let canvas = make_canvas(&note);

            stack.add_named(canvas.widget(), Some(&id));
            canvases.borrow_mut().insert(canvas);

            let tab = build_tab(
                &title, id.clone(),
                &stack, &tab_list, &workspace,
                &tab_inner, &recently_closed, &subtitle,
                apply_accent.clone(),
            );

            if let Some(ref hex) = accent {
                tab.add_css_class(&format!("tab-accent-{}", hex.trim_start_matches('#')));
            }

            tab_inner.append(&tab);
            tab_list.borrow_mut().push(tab);
        }

        if let Some(t) = workspace.borrow().open_tabs().get(active_index) {
            stack.set_visible_child_name(t.note_identifier());
        }
        mark_active(&tab_list, active_index);

        let controller = Rc::new(TabController {
            workspace,
            tab_list,
            stack,
            tab_scroll,
            tab_inner,
            new_tab_btn: new_tab_btn.clone(),
            recently_closed,
            subtitle,
            canvases,
            apply_accent,
            vim_pill,
        });

        {
            let ctrl = controller.clone();
            new_tab_btn.connect_clicked(move |_| ctrl.open_new());
        }

        Self { strip, controller }
    }

    pub fn controller(&self) -> Rc<TabController> {
        self.controller.clone()
    }
}
