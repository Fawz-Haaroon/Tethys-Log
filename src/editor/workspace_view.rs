use std::{cell::RefCell, rc::Rc};

use gtk::{prelude::*, Align, Box, Button, Label, Orientation, Stack};

use crate::{
    document::workspace::WorkspaceDocument,
    editor::{
        canvas::vim::VimMode,
        tabs::{TabBar, TabController, active::update_path_bar},
    },
};

pub struct WorkspaceView {
    root:       Box,
    controller: Rc<TabController>,
}

impl WorkspaceView {
    pub fn new(workspace: WorkspaceDocument) -> Self {
        let workspace = Rc::new(RefCell::new(workspace));

        // Path subtitle — hexpand pushes the buttons+pill to the right,
        // and ellipsizes long paths so they never collide with anything.
        let subtitle = Label::builder()
            .halign(Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .margin_start(10)
            .margin_bottom(3)
            .hexpand(true)
            .build();
        subtitle.add_css_class("note-path-label");

        {
            let subtitle_ref = subtitle.clone();
            let click = gtk::GestureClick::builder().button(1).build();
            click.connect_released(move |_, _, _, _| {
                open_path_in_filemanager(&subtitle_ref);
            });
            subtitle.add_controller(click);
            subtitle.set_cursor(gtk::gdk::Cursor::from_name("pointer", None).as_ref());
        }

        // Vim mode indicator pill — status only, not interactive
        let vim_pill = Label::builder()
            .label("INSERT")
            .halign(Align::End)
            .margin_end(10)
            .margin_bottom(3)
            .tooltip_text(
                "Vim mode  |  Esc → Normal  |  i/a → Insert  |  v → Visual  |  / → Search  |  n/N → next/prev match",
            )
            .build();
        vim_pill.add_css_class("vim-pill");
        vim_pill.add_css_class("vim-insert");

        // Editor stack (tab pages live here)
        let stack = Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        // Tab bar + controller
        let vim_pill_ref = vim_pill.clone();
        let tab_bar = TabBar::new(workspace.clone(), stack.clone(), subtitle.clone(), vim_pill_ref);
        let controller = tab_bar.controller();

        // Attach buttons — live between the path label and the vim pill.
        // They use hexpand=false so the path label always wins any available
        // space; buttons never shift left even with very long paths.
        let img_btn = attach_btn("Image", "Attach a local image at the cursor");
        let vid_btn = attach_btn("Video", "Attach a local video at the cursor");

        {
            let ctrl = controller.clone();
            img_btn.connect_clicked(move |_| ctrl.attach_image());
        }
        {
            let ctrl = controller.clone();
            vid_btn.connect_clicked(move |_| ctrl.attach_video());
        }

        // Status bar row: [path ← hexpand] [Image] [Video] [NORM]
        let status_bar = Box::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build();
        status_bar.append(&subtitle);
        status_bar.append(&img_btn);
        status_bar.append(&vid_btn);
        status_bar.append(&vim_pill);

        update_path_bar(&subtitle, &workspace);

        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();
        root.append(&tab_bar.strip);
        root.append(&status_bar);
        root.append(&stack);

        Self { root, controller }
    }

    pub fn widget(&self)     -> &Box              { &self.root }
    pub fn controller(&self) -> Rc<TabController> { self.controller.clone() }
}


// ── helpers ───────────────────────────────────────────────────────────────────

fn attach_btn(label: &str, tooltip: &str) -> Button {
    let b = Button::builder()
        .label(label)
        .tooltip_text(tooltip)
        .build();
    b.add_css_class("attach-inline-btn");
    b.set_cursor_from_name(Some("pointer"));
    b
}

fn open_path_in_filemanager(label: &Label) {
    let text = label.text().to_string();
    if text.is_empty() { return; }
    // label may be "Title  —  ~/path/..." — extract the path part
    let path_part = if let Some(p) = text.splitn(2, "—").nth(1) {
        p.trim().to_string()
    } else {
        text
    };
    let expanded = if path_part.starts_with('~') {
        let home = std::env::var("HOME").unwrap_or_default();
        path_part.replacen('~', &home, 1)
    } else {
        path_part
    };
    let path = std::path::Path::new(&expanded);
    let dir  = path.parent().unwrap_or(path);
    let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
}


pub fn apply_vim_mode_to_pill(pill: &Label, mode: VimMode) {
    pill.remove_css_class("vim-insert");
    pill.remove_css_class("vim-normal");
    pill.remove_css_class("vim-visual");
    match mode {
        VimMode::Normal => { pill.set_text("NORMAL"); pill.add_css_class("vim-normal"); }
        VimMode::Insert => { pill.set_text("INSERT"); pill.add_css_class("vim-insert"); }
        VimMode::Visual => { pill.set_text("VISUAL"); pill.add_css_class("vim-visual"); }
    }
}
