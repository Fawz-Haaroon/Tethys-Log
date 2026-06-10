// Drop target for local files dragged into the editor.
//
// Image files: imported into the note's image store first (copy-on-insert so
//              the note survives moves or deletions of the source), then
//              inserted as inline paintables.
// Video files: imported into the note's video store, inserted as gtk::Video
//              widgets — same copy-on-insert contract as images.
//
// IMPORTANT: always call import_image / import_video BEFORE inserting so
// the saved filename resolves correctly on the next load.  The codec stores
// only the bare filename; deserialization looks in the per-note store dir.

use gtk::{gdk, glib, prelude::*, TextView};

use crate::editor::canvas::codec::{
    filename_from_path, insert_image_paintable_tagged, insert_video_anchor,
};
use crate::storage::image_store::import_image;
use crate::storage::video_store::import_video;

pub fn wire_image_drop(view: &TextView, note_identifier: String) {
    let drop_tgt = gtk::DropTarget::new(glib::types::Type::STRING, gdk::DragAction::COPY);
    let view_ref = view.clone();

    drop_tgt.connect_drop(move |_, value, _, _| {
        let uri_list = match value.get::<String>() {
            Ok(s)  => s,
            Err(_) => return false,
        };

        let path = uri_list
            .lines()
            .find(|l| !l.starts_with('#') && !l.trim().is_empty())
            .and_then(|uri| glib::filename_from_uri(uri.trim()).ok())
            .map(|(p, _)| p);

        let path = match path {
            Some(p) => p,
            None    => return false,
        };

        if is_image_path(&path) {
            let stored = match import_image(&note_identifier, &path) {
                Ok(p)  => p,
                Err(_) => return false,
            };
            let filename = match filename_from_path(&stored) {
                Some(f) => f,
                None    => return false,
            };
            let buffer   = view_ref.buffer();
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            let _ = insert_image_paintable_tagged(&buffer, &view_ref, &mut iter, &stored, &filename);
            view_ref.scroll_mark_onscreen(&buffer.get_insert());
            return true;
        }

        if is_video_path(&path) {
            let stored = match import_video(&note_identifier, &path) {
                Ok(p)  => p,
                Err(_) => return false,
            };
            let filename = match filename_from_path(&stored) {
                Some(f) => f,
                None    => return false,
            };
            let buffer   = view_ref.buffer();
            let mut iter = buffer.iter_at_mark(&buffer.get_insert());
            insert_video_anchor(&buffer, &view_ref, &mut iter, &stored, &filename);
            view_ref.scroll_mark_onscreen(&buffer.get_insert());
            return true;
        }

        false
    });

    view.add_controller(drop_tgt);
}

fn is_image_path(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp")
    )
}

fn is_video_path(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("mp4" | "mkv" | "webm" | "mov" | "avi" | "ogv")
    )
}
