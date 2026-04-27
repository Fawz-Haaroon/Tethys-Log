mod app;
mod document;
mod editor;
mod storage;

use gtk::{prelude::*, Application};

fn main() {
    let app = Application::builder()
        .application_id("com.tethyslog.app")
        .build();
    app.connect_activate(app::boot::build_window);
    app.run();
}
