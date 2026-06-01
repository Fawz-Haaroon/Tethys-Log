use std::{cell::RefCell, rc::Rc};

use gtk::{prelude::*, Box, Button, ScrolledWindow};
use glib::idle_add_local_once;

pub fn scroll_tab_into_view(
    tab_scroll: &ScrolledWindow,
    tab_list: &Rc<RefCell<Vec<Box>>>,
    index: usize,
) {
    if let Some(tab) = tab_list.borrow().get(index).cloned() {
        let s = tab_scroll.clone();
        idle_add_local_once(move || {
            let alloc = tab.allocation();
            let x = alloc.x() as f64;
            let w = alloc.width() as f64;
            let adj = s.hadjustment();
            let pos = adj.value();
            let page = adj.page_size();
            if x < pos {
                adj.set_value(x);
            } else if x + w > pos + page {
                adj.set_value((x + w - page).max(0.0));
            }
        });
    }
}

pub fn scroll_arrow_btn(label: &str) -> Button {
    let btn = Button::builder().label(label).build();
    btn.add_css_class("tab-scroll-btn");
    btn
}
