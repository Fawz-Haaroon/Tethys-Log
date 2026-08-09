use std::{cell::RefCell, rc::Rc};

use gtk::{prelude::*, gio, Application, ApplicationWindow};
use gtk::glib;

use crate::{
    app::{actions, keybindings, zoom::{load_base_theme, ZoomState}},
    editor::{tabs::TabController, workspace_view::WorkspaceView},
    storage::session::SessionStore,
};

thread_local! {
    // The running instance's tab controller, once a window has been built.
    //
    // GApplication is single-instance by default: a second `tethys-log
    // somefile.tlog` while the app is already running doesn't start a new
    // process, it delivers `open` to *this* process. `open` needs to reach
    // the same tabs `activate` built, and GTK signal handlers only get
    // `&Application` -- there's no channel back to the TabController that
    // build_window already made through the signal machinery itself. A
    // thread-local slot is the standard escape hatch for exactly this
    // "one GObject-shaped app, one small piece of state every entry point
    // needs" situation, and it's sound here because GTK itself is
    // single-threaded -- every signal handler in this app already runs on
    // the one thread the main loop owns.
    static ACTIVE_CONTROLLER: RefCell<Option<Rc<TabController>>> = const { RefCell::new(None) };
}

pub fn build_window(app: &Application) -> Rc<TabController> {
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
    keybindings::attach(&window, tabs.clone(), zoom);
    ACTIVE_CONTROLLER.with(|cell| *cell.borrow_mut() = Some(tabs.clone()));
    window.present();
    tabs
}

/// Handles files passed on the command line (`tethys-log alya.tlog`) or
/// opened via the installed .desktop file's `Exec=... %F` -- a file-manager
/// double-click or "Open With Tethys Log". GApplication delivers these to
/// the `open` signal instead of `activate` (see main.rs), so on a first
/// launch with file arguments this builds the window itself, restoring the
/// previous session exactly as a plain launch would, and adds the
/// requested files as further tabs on top -- opening a file is additive,
/// never a reason to discard tabs that were already open. On an
/// already-running instance it reaches that same window through
/// ACTIVE_CONTROLLER and just adds tabs to it.
pub fn open_files(app: &Application, files: &[gio::File], _hint: &str) {
    let tabs = ACTIVE_CONTROLLER
        .with(|cell| cell.borrow().clone())
        .unwrap_or_else(|| build_window(app));

    for file in files {
        match file.path() {
            Some(path) => tabs.open_path(&path),
            None => eprintln!(
                "tethys-log: skipping a non-local file -- only paths on this machine are supported"
            ),
        }
    }

    // Bring the window forward. Matters most when the app was already
    // running: opening a file from the file manager should feel like
    // switching to Tethys Log, not like nothing happened.
    if let Some(window) = app.active_window() {
        window.present();
    }
}
