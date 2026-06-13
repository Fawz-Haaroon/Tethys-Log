// Ctrl+V intercept for embed-worthy URLs and iframe/blockquote snippets.
//
// Runs at Capture phase so it sees the keypress before GTK's built-in paste
// handler. If the clipboard text is recognisable as an embed, we insert an
// embed card and swallow the event. Otherwise we replicate the default paste.

use gtk::{gdk, glib, prelude::*, TextView};

use crate::editor::canvas::{
    codec::insert_embed_anchor,
    embed::{classify_url, EmbedKind},
};

pub fn wire_url_paste(view: &TextView) {
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

        // image pastes handled by clipboard_paste.rs — don't interfere
        let has_image = formats.contains_type(gdk::Texture::static_type())
            || formats.mime_types().iter().any(|m| m.starts_with("image/"));
        if has_image {
            return glib::Propagation::Proceed;
        }

        let has_text = formats.mime_types().iter().any(|m| {
            m.starts_with("text/plain") || *m == "UTF8_STRING" || *m == "STRING"
        });
        if !has_text {
            return glib::Propagation::Proceed;
        }

        let view_weak = view_ref.downgrade();

        clipboard.read_text_async(
            gtk::gio::Cancellable::NONE,
            move |result| {
                let text = match result {
                    Ok(Some(t)) => t.to_string(),
                    _           => return,
                };

                let view = match view_weak.upgrade() {
                    Some(v) => v,
                    None    => return,
                };

                match classify_url(text.trim()) {
                    Some(EmbedKind::YouTube { watch_url, .. }) |
                    Some(EmbedKind::Generic { watch_url, .. }) => {
                        let buffer   = view.buffer();
                        let mut iter = buffer.iter_at_mark(&buffer.get_insert());
                        insert_embed_anchor(&buffer, &view, &mut iter, &watch_url);
                        view.scroll_mark_onscreen(&buffer.get_insert());
                    }
                    None => {
                        let buffer = view.buffer();
                        buffer.delete_selection(true, true);
                        buffer.insert_at_cursor(&text);
                    }
                }
            },
        );

        glib::Propagation::Stop
    });

    view.add_controller(keys);
}
