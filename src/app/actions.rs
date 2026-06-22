use gtk::gio;
use gtk::prelude::*;

// Stable action names that the rest of the app (keybindings, menus) targets.
// Adding a new shortcut:  app.set_accels_for_action("win.search", &["<Ctrl>f"])
// Adding a menu entry:    gio::MenuItem::new(Some("Find..."), Some("win.search"))
pub fn register_window_actions(window: &gtk::ApplicationWindow) {
    for name in [
        "new-tab",
        "close-tab",
        "reopen-tab",
        "rename-tab",
        "open-file",
        "search",
        "zoom-in",
        "zoom-out",
        "zoom-reset",
    ] {
        window.add_action(&gio::SimpleAction::new(name, None));
    }
}
