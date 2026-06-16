use std::{cell::RefCell, rc::Rc};

use gtk::{
    prelude::*,
    Box, Button, CheckButton, Entry, Label, Orientation, RevealerTransitionType,
};

use crate::editor::search::logic::SearchState;

pub struct SearchBar {
    revealer:      gtk::Revealer,
    query_entry:   Entry,
    #[allow(dead_code)]
    replace_entry: Entry,
    // Stored for go_next / go_prev (vim n / N) without opening the bar.
    state:         Rc<RefCell<Option<SearchState>>>,
    buffer:        gtk::TextBuffer,
}

impl SearchBar {
    pub fn new(buffer: gtk::TextBuffer) -> Self {
        let state: Rc<RefCell<Option<SearchState>>> = Rc::new(RefCell::new(None));

        let query_entry = Entry::builder()
            .placeholder_text("Find…")
            .hexpand(true)
            .build();
        query_entry.add_css_class("search-entry");

        let replace_entry = Entry::builder()
            .placeholder_text("Replace with…")
            .hexpand(true)
            .build();
        replace_entry.add_css_class("search-entry");

        let case_toggle = CheckButton::builder().label("Aa").build();

        let prev_btn        = nav_btn("↑");
        let next_btn        = nav_btn("↓");
        let replace_btn     = action_btn("Replace");
        let replace_all_btn = action_btn("All");
        let close_btn       = action_btn("✕");

        let match_label = Label::new(Some(""));
        match_label.add_css_class("search-match-count");

        let top_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .build();
        top_row.append(&query_entry);
        top_row.append(&prev_btn);
        top_row.append(&next_btn);
        top_row.append(&match_label);
        top_row.append(&case_toggle);
        top_row.append(&close_btn);

        let bottom_row = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .build();
        bottom_row.append(&replace_entry);
        bottom_row.append(&replace_btn);
        bottom_row.append(&replace_all_btn);

        let inner = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .margin_start(8).margin_end(8)
            .margin_top(4).margin_bottom(4)
            .build();
        inner.add_css_class("search-bar");
        inner.append(&top_row);
        inner.append(&bottom_row);

        let revealer = gtk::Revealer::builder()
            .transition_type(RevealerTransitionType::SlideDown)
            .transition_duration(120)
            .child(&inner)
            .reveal_child(false)
            .build();

        // query entry → update state + find first match
        {
            let state = state.clone();
            let buffer = buffer.clone();
            let case_toggle = case_toggle.clone();
            query_entry.connect_changed(move |entry| {
                let q = entry.text().to_string();
                let mut s = SearchState::new(&q);
                s.case_sensitive = case_toggle.is_active();
                let from = buffer.start_iter();
                s.find_next(&buffer, &from);
                *state.borrow_mut() = if q.is_empty() { None } else { Some(s) };
            });
        }

        {
            let state  = state.clone();
            let buffer = buffer.clone();
            next_btn.connect_clicked(move |_| {
                if let Some(s) = state.borrow().as_ref() {
                    let from = buffer.iter_at_mark(&buffer.get_insert());
                    if let Some((start, end)) = s.find_next(&buffer, &from) {
                        buffer.select_range(&start, &end);
                    }
                }
            });
        }

        {
            let state  = state.clone();
            let buffer = buffer.clone();
            prev_btn.connect_clicked(move |_| {
                if let Some(s) = state.borrow().as_ref() {
                    let from = buffer.iter_at_mark(&buffer.get_insert());
                    if let Some((start, end)) = s.find_prev(&buffer, &from) {
                        buffer.select_range(&start, &end);
                    }
                }
            });
        }

        {
            let state         = state.clone();
            let buffer        = buffer.clone();
            let replace_entry = replace_entry.clone();
            replace_btn.connect_clicked(move |_| {
                if let Some(s) = state.borrow().as_ref() {
                    let repl = replace_entry.text().to_string();
                    s.replace_current(&buffer, &repl);
                    let from = buffer.iter_at_mark(&buffer.get_insert());
                    s.find_next(&buffer, &from);
                }
            });
        }

        {
            let state         = state.clone();
            let buffer        = buffer.clone();
            let replace_entry = replace_entry.clone();
            replace_all_btn.connect_clicked(move |_| {
                if let Some(s) = state.borrow().as_ref() {
                    let repl = replace_entry.text().to_string();
                    s.replace_all(&buffer, &repl);
                }
            });
        }

        {
            let revealer = revealer.clone();
            close_btn.connect_clicked(move |_| {
                revealer.set_reveal_child(false);
            });
        }

        Self { revealer, query_entry, replace_entry, state, buffer }
    }

    pub fn widget(&self) -> &gtk::Revealer { &self.revealer }

    pub fn open(&self) {
        self.revealer.set_reveal_child(true);
        self.query_entry.grab_focus();
    }

    pub fn close(&self) {
        self.revealer.set_reveal_child(false);
    }

    pub fn is_open(&self) -> bool {
        self.revealer.reveals_child()
    }

    /// Navigate to the next search match (called from vim `n`).
    pub fn go_next(&self) {
        if let Some(s) = self.state.borrow().as_ref() {
            let from = self.buffer.iter_at_mark(&self.buffer.get_insert());
            if let Some((start, end)) = s.find_next(&self.buffer, &from) {
                self.buffer.select_range(&start, &end);
            }
        }
    }

    /// Navigate to the previous search match (called from vim `N`).
    pub fn go_prev(&self) {
        if let Some(s) = self.state.borrow().as_ref() {
            let from = self.buffer.iter_at_mark(&self.buffer.get_insert());
            if let Some((start, end)) = s.find_prev(&self.buffer, &from) {
                self.buffer.select_range(&start, &end);
            }
        }
    }
}

fn nav_btn(label: &str) -> Button {
    let b = Button::builder().label(label).build();
    b.add_css_class("search-nav-btn");
    b
}

fn action_btn(label: &str) -> Button {
    let b = Button::builder().label(label).build();
    b.add_css_class("search-action-btn");
    b
}
