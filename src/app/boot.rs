use std::rc::Rc;

use gtk::{prelude::*, Application, ApplicationWindow};
use gtk::glib;

use crate::{
    app::{actions, keybindings, zoom::{load_base_theme, ZoomState}},
    editor::workspace_view::WorkspaceView,
    storage::session::SessionStore,
};

pub fn build_window(app: &Application) {
    app.set_accels_for_action("window.close", &[]);
    load_base_theme();
    let zoom = Rc::new(ZoomState::init());
    let session = SessionStore::load();
    let workspace_view = WorkspaceView::new(session);
    let tabs = workspace_view.controller();
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Tethys Log")
        .default_width(1350)
        .default_height(820)
        .child(workspace_view.widget())
        .build();
    actions::register_window_actions(&window);
    window.connect_close_request(|_| glib::Propagation::Proceed);
    keybindings::attach(&window, tabs, zoom);
    window.present();
}
