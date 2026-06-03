use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use gtk::{prelude::*, Align, Box, Button, Label, Orientation, Stack};

use crate::{
    document::workspace::WorkspaceDocument,
    editor::tabs::{
        active::{mark_active, update_path_bar, tab_index_of},
        close::close_tab,
        closed::ClosedTab,
        context_menu::wire_right_click_menu,
        drag::wire_drag_reorder,
    },
    storage::session::SessionStore,
};

pub const TAB_WIDTH: i32 = 200;
const TAB_TITLE_MAX_CHARS: i32 = 18;

// ApplyAccent: (shell, color_or_none) — injected from bar.rs after controller exists.
// Using a trait object so widget.rs stays controller-free.
pub type ApplyAccentFn = Rc<dyn Fn(&Box, Option<String>)>;

pub fn build_tab(
    title: &str,
    page_name: String,
    stack: &Stack,
    tab_list: &Rc<RefCell<Vec<Box>>>,
    workspace: &Rc<RefCell<WorkspaceDocument>>,
    tab_inner: &Box,
    recently_closed: &Rc<RefCell<VecDeque<ClosedTab>>>,
    subtitle: &Label,
    apply_accent: ApplyAccentFn,
) -> Box {
    let shell = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(0)
        .hexpand(false)
        .build();
    shell.set_size_request(TAB_WIDTH, -1);
    shell.set_overflow(gtk::Overflow::Hidden);
    shell.add_css_class("tab");
    shell.set_widget_name(&page_name);

    let live_title: Rc<RefCell<String>> = Rc::new(RefCell::new(title.to_string()));

    shell.set_has_tooltip(true);
    {
        let live_title = live_title.clone();
        shell.connect_query_tooltip(move |_, _, _, _, tip| {
            tip.set_text(Some(live_title.borrow().as_str()));
            true
        });
    }

    let title_btn = Button::builder().label(title).hexpand(true).build();
    title_btn.add_css_class("tab-title");
    if let Some(lbl) = title_btn.child().and_then(|w| w.downcast::<gtk::Label>().ok()) {
        lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
        lbl.set_max_width_chars(TAB_TITLE_MAX_CHARS);
        lbl.set_halign(Align::Start);
        lbl.set_hexpand(true);
    }

    {
        let stack = stack.clone();
        let tab_list = tab_list.clone();
        let workspace = workspace.clone();
        let page_name = page_name.clone();
        let shell_ref = shell.clone();
        let subtitle = subtitle.clone();
        title_btn.connect_clicked(move |_| {
            stack.set_visible_child_name(&page_name);
            let index = tab_index_of(&tab_list, &shell_ref);
            workspace.borrow_mut().switch_to(index);
            mark_active(&tab_list, index);
            update_path_bar(&subtitle, &workspace);
            SessionStore::persist(&workspace.borrow());
        });
    }

    // middle-click close
    {
        let stack = stack.clone();
        let tab_list = tab_list.clone();
        let workspace = workspace.clone();
        let tab_inner = tab_inner.clone();
        let recently_closed = recently_closed.clone();
        let shell_ref = shell.clone();
        let page_name = page_name.clone();
        let subtitle = subtitle.clone();
        let mid = gtk::GestureClick::builder().button(2).build();
        mid.connect_released(move |_, _, _, _| {
            close_tab(
                &shell_ref, &page_name,
                &stack, &tab_list, &workspace,
                &tab_inner, &recently_closed,
            );
            update_path_bar(&subtitle, &workspace);
        });
        shell.add_controller(mid);
    }

    wire_right_click_menu(
        &shell, &title_btn, &live_title,
        tab_list, workspace, stack, tab_inner, recently_closed, subtitle,
        &page_name, apply_accent,
    );

    let close_btn = Button::builder().label("×").build();
    close_btn.add_css_class("tab-close");

    {
        let stack = stack.clone();
        let tab_list = tab_list.clone();
        let workspace = workspace.clone();
        let tab_inner = tab_inner.clone();
        let shell_ref = shell.clone();
        let page_name = page_name.clone();
        let recently_closed = recently_closed.clone();
        let subtitle = subtitle.clone();
        close_btn.connect_clicked(move |_| {
            close_tab(
                &shell_ref, &page_name,
                &stack, &tab_list, &workspace,
                &tab_inner, &recently_closed,
            );
            update_path_bar(&subtitle, &workspace);
        });
    }

    wire_drag_reorder(&shell, tab_list, workspace, subtitle);

    shell.append(&title_btn);
    shell.append(&close_btn);
    shell
}
