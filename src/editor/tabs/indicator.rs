use gtk::{prelude::*, Box};

// shows a small dot on the tab when its note has unsaved changes.
// the dot is a separate label child appended after the close button —
// removing and re-adding is cheaper than fighting CSS visibility on GTK.
const DIRTY_DOT: &str = "·";
const DOT_CSS_CLASS: &str = "tab-dirty-dot";

pub fn show_dirty(shell: &Box) {
    if has_dot(shell) { return; }
    let dot = gtk::Label::new(Some(DIRTY_DOT));
    dot.add_css_class(DOT_CSS_CLASS);
    shell.append(&dot);
}

pub fn clear_dirty(shell: &Box) {
    let dot = find_dot(shell);
    if let Some(w) = dot {
        shell.remove(&w);
    }
}

fn has_dot(shell: &Box) -> bool {
    find_dot(shell).is_some()
}

fn find_dot(shell: &Box) -> Option<gtk::Widget> {
    let mut child = shell.first_child();
    while let Some(w) = child {
        if w.css_classes().iter().any(|c| c.as_str() == DOT_CSS_CLASS) {
            return Some(w);
        }
        child = w.next_sibling();
    }
    None
}
