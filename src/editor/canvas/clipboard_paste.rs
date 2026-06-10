use std::path::PathBuf;

use gtk::{gdk, glib, prelude::*, TextView};

use crate::{
    editor::canvas::codec::{filename_from_path, insert_image_paintable_tagged},
    storage::paths::data_dir,
};

pub fn wire_clipboard_image_paste(view: &TextView, note_identifier: String) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);

    let view_ref = view.clone();
    keys.connect_key_pressed(move |_, key, _, mods| {
        let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
        if !ctrl || (key != gdk::Key::v && key != gdk::Key::V) {
            return glib::Propagation::Proceed;
        }

        let display = match gdk::Display::default() {
            Some(d) => d,
            None    => return glib::Propagation::Proceed,
        };
        let clipboard = display.clipboard();
        let formats   = clipboard.formats();

        let has_image = formats.contains_type(gdk::Texture::static_type())
            || formats.mime_types().iter().any(|m| m.starts_with("image/"));

        if !has_image {
            return glib::Propagation::Proceed;
        }

        let id        = note_identifier.clone();
        let view_weak = view_ref.downgrade();

        clipboard.read_texture_async(
            gtk::gio::Cancellable::NONE,
            move |result| {
                let texture = match result {
                    Ok(Some(t)) => t,
                    _           => return,
                };
                let dest = match save_texture(&texture, &id) {
                    Some(p) => p,
                    None    => return,
                };
                let filename = match filename_from_path(&dest) {
                    Some(f) => f,
                    None    => return,
                };
                if let Some(view) = view_weak.upgrade() {
                    let buffer = view.buffer();
                    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
                    let _ = insert_image_paintable_tagged(&buffer, &view, &mut iter, &dest, &filename);
                    view.scroll_mark_onscreen(&buffer.get_insert());
                }
            },
        );

        glib::Propagation::Stop
    });

    view.add_controller(keys);
}

fn save_texture(texture: &gdk::Texture, note_identifier: &str) -> Option<PathBuf> {
    let dir = data_dir().join("images").join(note_identifier);
    std::fs::create_dir_all(&dir).ok()?;
    let micros = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or(0);
    let dest = dir.join(format!("paste-{micros}.png"));
    texture.save_to_png(dest.to_str()?).ok()?;
    Some(dest)
}
