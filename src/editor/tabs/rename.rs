use std::{cell::RefCell, rc::Rc};

use gtk::{
    prelude::*,
    Align, Box, Button, Dialog, Entry, Label, Orientation, ResponseType,
};

use crate::{
    document::workspace::WorkspaceDocument,
    editor::tabs::active::{update_path_bar, tab_index_of},
    storage::{notes::NoteStore, session::SessionStore},
};

pub fn show_rename_dialog(
    parent:     &impl IsA<gtk::Window>,
    shell:      &Box,
    title_btn:  &Button,
    live_title: &Rc<RefCell<String>>,
    tab_list:   &Rc<RefCell<Vec<Box>>>,
    workspace:  &Rc<RefCell<WorkspaceDocument>>,
    subtitle:   &Label,
) {
    let current = title_btn.label().map(|s| s.to_string()).unwrap_or_default();

    let dialog = Dialog::builder()
        .title("Rename Note")
        .transient_for(parent)
        .modal(true)
        .use_header_bar(1)
        .build();

    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Rename", ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);

    let hint = Label::builder()
        .label("Note name")
        .halign(Align::Start)
        .margin_start(16)
        .margin_top(16)
        .margin_bottom(4)
        .build();
    hint.add_css_class("rename-hint");

    let entry = Entry::builder()
        .text(&current)
        .activates_default(true)
        .margin_start(16)
        .margin_end(16)
        .margin_top(8)
        .margin_bottom(16)
        .build();
    entry.add_css_class("rename-entry");
    entry.select_region(0, -1);

    let content = dialog.content_area();
    content.set_orientation(Orientation::Vertical);
    content.append(&hint);
    content.append(&entry);

    let shell      = shell.clone();
    let title_btn  = title_btn.clone();
    let live_title = live_title.clone();
    let tab_list   = tab_list.clone();
    let workspace  = workspace.clone();
    let subtitle   = subtitle.clone();

    dialog.connect_response(move |d, response| {
        if response == ResponseType::Accept {
            let trimmed = entry.text().trim().to_string();
            if !trimmed.is_empty() {
                // Remove the title-named shadow file for the old name so the
                // file manager shows the new name only after the next autosave.
                NoteStore::cleanup_title_file(&current);
                let index = tab_index_of(&tab_list, &shell);
                workspace.borrow_mut().rename_tab(index, trimmed.clone());
                title_btn.set_label(&trimmed);
                *live_title.borrow_mut() = trimmed;
                SessionStore::persist(&workspace.borrow());
                // update path subtitle so the new name is reflected immediately
                update_path_bar(&subtitle, &workspace);
            }
        }
        d.close();
    });

    dialog.present();
}
