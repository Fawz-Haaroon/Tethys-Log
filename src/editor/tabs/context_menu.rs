use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use gtk::{prelude::*, Box, Button, Label, Orientation, Stack};

use crate::{
    document::workspace::WorkspaceDocument,
    editor::tabs::{
        active::update_path_bar,
        close::close_tab,
        closed::ClosedTab,
        rename::show_rename_dialog,
        widget::ApplyAccentFn,
    },
};

const ACCENTS: &[(&str, &str)] = &[
    ("Red",    "#c0504a"),
    ("Orange", "#c07a30"),
    ("Yellow", "#b09a20"),
    ("Green",  "#3a8f60"),
    ("Teal",   "#2a8a8a"),
    ("Blue",   "#3a70b0"),
    ("Purple", "#7a50a8"),
    ("Pink",   "#a04070"),
];

pub fn wire_right_click_menu(
    shell: &Box,
    title_btn: &Button,
    live_title: &Rc<RefCell<String>>,
    tab_list: &Rc<RefCell<Vec<Box>>>,
    workspace: &Rc<RefCell<WorkspaceDocument>>,
    stack: &Stack,
    tab_inner: &Box,
    recently_closed: &Rc<RefCell<VecDeque<ClosedTab>>>,
    subtitle: &Label,
    page_name: &str,
    apply_accent: ApplyAccentFn,
) {
    let tab_list = tab_list.clone();
    let workspace = workspace.clone();
    let stack = stack.clone();
    let tab_inner = tab_inner.clone();
    let recently_closed = recently_closed.clone();
    let subtitle = subtitle.clone();
    let shell_ref = shell.clone();
    let title_btn_ref = title_btn.clone();
    let live_title = live_title.clone();
    let page_name = page_name.to_string();

    let gesture = gtk::GestureClick::builder().button(3).build();
    gesture.set_propagation_phase(gtk::PropagationPhase::Bubble);
    gesture.connect_released(move |_, _, _, _| {
        show_tab_context_menu(
            &shell_ref, &title_btn_ref, &live_title,
            &tab_list, &workspace,
            &stack, &tab_inner, &recently_closed,
            &subtitle, &page_name, &apply_accent,
        );
    });
    shell.add_controller(gesture);
}

fn show_tab_context_menu(
    shell: &Box,
    title_btn: &Button,
    live_title: &Rc<RefCell<String>>,
    tab_list: &Rc<RefCell<Vec<Box>>>,
    workspace: &Rc<RefCell<WorkspaceDocument>>,
    stack: &Stack,
    tab_inner: &Box,
    recently_closed: &Rc<RefCell<VecDeque<ClosedTab>>>,
    subtitle: &Label,
    page_name: &str,
    apply_accent: &ApplyAccentFn,
) {
    let popup = gtk::Popover::new();
    popup.set_parent(shell);
    popup.set_has_arrow(false);

    let menu_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .build();
    menu_box.add_css_class("tab-menu");

    let rename_item    = menu_item_btn("Rename");
    let highlight_item = menu_item_btn("Highlight");
    let clear_item     = menu_item_btn("Clear Color");
    let sep            = gtk::Separator::new(Orientation::Horizontal);
    let close_item     = menu_item_btn("Close Tab");

    menu_box.append(&rename_item);
    menu_box.append(&highlight_item);
    menu_box.append(&clear_item);
    menu_box.append(&sep);
    menu_box.append(&close_item);
    popup.set_child(Some(&menu_box));

    {
        let shell = shell.clone();
        let title_btn = title_btn.clone();
        let live_title = live_title.clone();
        let tab_list = tab_list.clone();
        let workspace = workspace.clone();
        let subtitle_ref = subtitle.clone();
        let popup = popup.clone();
        rename_item.connect_clicked(move |_| {
            popup.popdown();
            let win = shell.root().and_then(|r| r.downcast::<gtk::Window>().ok());
            if let Some(w) = win {
                show_rename_dialog(&w, &shell, &title_btn, &live_title, &tab_list, &workspace, &subtitle_ref);
            }
        });
    }

    {
        let shell = shell.clone();
        let apply_accent = apply_accent.clone();
        let popup = popup.clone();
        highlight_item.connect_clicked(move |_| {
            popup.popdown();
            show_color_picker(&shell, &apply_accent);
        });
    }

    {
        let shell = shell.clone();
        let apply_accent = apply_accent.clone();
        let popup = popup.clone();
        clear_item.connect_clicked(move |_| {
            popup.popdown();
            apply_accent(&shell, None);
        });
    }

    {
        let shell = shell.clone();
        let page_name = page_name.to_string();
        let stack = stack.clone();
        let tab_list = tab_list.clone();
        let workspace = workspace.clone();
        let tab_inner = tab_inner.clone();
        let recently_closed = recently_closed.clone();
        let subtitle = subtitle.clone();
        let popup = popup.clone();
        close_item.connect_clicked(move |_| {
            popup.popdown();
            close_tab(
                &shell, &page_name, &stack, &tab_list,
                &workspace, &tab_inner, &recently_closed,
            );
            update_path_bar(&subtitle, &workspace);
        });
    }

    popup.popup();
}

fn show_color_picker(shell: &Box, apply_accent: &ApplyAccentFn) {
    let picker = gtk::Popover::new();
    picker.set_parent(shell);
    picker.set_has_arrow(false);

    let grid = gtk::Grid::builder()
        .row_spacing(4)
        .column_spacing(4)
        .margin_start(8)
        .margin_end(8)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    for (i, (name, hex)) in ACCENTS.iter().enumerate() {
        let swatch = Button::builder().build();
        swatch.set_size_request(22, 22);
        swatch.set_tooltip_text(Some(name));
        swatch.add_css_class("color-swatch");

        let provider = gtk::CssProvider::new();
        provider.load_from_data(&format!(
            "button.color-swatch {{ \
                background: {hex}; \
                border-radius: 50%; \
                border: 2px solid rgba(255,255,255,0.12); \
                min-width: 22px; min-height: 22px; \
                padding: 0; \
            }} \
            button.color-swatch:hover {{ \
                border-color: rgba(255,255,255,0.5); \
            }}"
        ));
        swatch.style_context()
            .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

        let shell = shell.clone();
        let apply = apply_accent.clone();
        let hex_owned = hex.to_string();
        let picker_ref = picker.clone();
        swatch.connect_clicked(move |_| {
            picker_ref.popdown();
            apply(&shell, Some(hex_owned.clone()));
        });

        grid.attach(&swatch, (i % 4) as i32, (i / 4) as i32, 1, 1);
    }

    picker.set_child(Some(&grid));
    picker.popup();
}

fn menu_item_btn(label: &str) -> Button {
    let btn = Button::builder().label(label).build();
    btn.add_css_class("tab-menu-item");
    btn
}
