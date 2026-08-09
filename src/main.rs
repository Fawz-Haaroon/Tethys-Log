mod app;
mod document;
mod editor;
mod storage;

use gtk::{prelude::*, gio, Application};

fn main() {
    let app = Application::builder()
        .application_id("com.tethyslog.app")
        // Without this, launching with a file argument (`tethys-log
        // alya.tlog`, or the .desktop file's `Exec=... %F` via a
        // file-manager double-click) hits GLib's default "This application
        // can not open files" critical and never presents a window --
        // GApplication routes file arguments to the `open` signal, and it
        // has to be told the application implements that signal.
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    app.connect_activate(|app| { app::boot::build_window(app); });
    app.connect_open(|app, files, hint| app::boot::open_files(app, files, hint));
    app.run();
}
