// EditorCanvas — one instance per open note tab.
//
// Layout (top → bottom):
//   SearchBar      — collapsible find/replace bar  (Ctrl+F or vim /)
//   ScrolledWindow — the GTK TextView with all content
//
// Attach buttons (Image / Video) have been moved out of this widget and live
// in the workspace status bar (workspace_view.rs), where they sit between the
// path label and the vim mode pill.  The EditorCanvas exposes:
//   trigger_attach_image()  — opens FileChooserNative for images at cursor
//   trigger_attach_video()  — opens FileChooserNative for videos at cursor
// Those methods are called by TabController when the status-bar buttons click.
//
// Syntax highlighting: wire_syntax_highlighting runs a debounced (400 ms)
// TextBuffer listener that colours ```lang…``` fences with Tokyo Night tags.
//
// Vim integration: wire_vim passes search callbacks so / opens the bar and
// n / N navigate matches without the bar needing to be visible.

use std::{cell::RefCell, rc::Rc};

use gtk::{
    prelude::*,
    Box, FileChooserAction, FileChooserNative, FileFilter, Orientation,
    ResponseType, ScrolledWindow, TextBuffer, TextView, WrapMode,
};

use crate::{
    document::note::NoteDocument,
    editor::{
        canvas::{
            autosave::wire_autosave,
            clipboard_paste::wire_clipboard_image_paste,
            codec::{
                deserialise_into_buffer, filename_from_path,
                image_dir_for_note, insert_image_paintable_tagged,
                insert_video_anchor, serialize_buffer,
            },
            highlight::wire_text_highlight,
            history::DocumentHistory,
            image_drop::wire_image_drop,
            syntax::wire_syntax_highlighting,
            url_paste::wire_url_paste,
            vim::{wire_vim, VimMode, VimState},
        },
        search::SearchBar,
    },
    storage::{image_store::import_image, video_store::import_video},
};

pub struct EditorCanvas {
    root:            Box,
    view:            TextView,
    search:          Rc<SearchBar>,
    buffer:          TextBuffer,
    note_identifier: String,
    pub vim:         Rc<VimState>,
}

impl EditorCanvas {
    pub fn new(
        note:           &NoteDocument,
        on_mode_change: impl Fn(VimMode) + 'static,
    ) -> Self {
        let buffer = TextBuffer::new(None);
        // GTK's own undo is deliberately left off. It only round-trips raw
        // codepoints -- verified directly against the API that a TextTag
        // (highlight, img-path, video-path, embed-src, all of them) never
        // survives a delete+undo, even though the character does. That's
        // what used to leave an unreadable orphan character where a deleted
        // image/video/embed had been. editor::canvas::history replaces
        // buffer.undo()/redo() entirely with its own snapshot-based undo,
        // which round-trips everything because it reuses the same
        // deserialise_into_buffer that already loads a note correctly.
        buffer.set_enable_undo(false);

        let view   = TextView::builder()
            .buffer(&buffer)
            .left_margin(12)
            .right_margin(12)
            .top_margin(12)
            .bottom_margin(12)
            .pixels_above_lines(0)
            .pixels_below_lines(0)
            .pixels_inside_wrap(0)
            .wrap_mode(WrapMode::Word)
            .monospace(true)
            .cursor_visible(true)
            .build();

        let raw_content = note.content_nodes()
            .first()
            .and_then(|n| match n {
                crate::document::node::NoteNode::Paragraph(p) => Some(p.as_str()),
                _ => None,
            })
            .unwrap_or("");

        let image_dir = image_dir_for_note(note.note_identifier());

        // Text highlight — right-click colour swatch popover. Wired before
        // the deserialise below (rather than after, where the equivalent
        // call used to sit) because it's what registers the hl-fg-*/hl-bg-*
        // TextTags. deserialise_into_buffer restores saved highlights by
        // looking those tags up by name; registering them any later means
        // the lookup fails and every saved highlight silently vanishes on
        // load, which is the bug this ordering fixes.
        wire_text_highlight(&view);

        deserialise_into_buffer(raw_content, &buffer, &view, &image_dir);

        // The note's undo history: seeded with whatever prior states were
        // already saved in the .tlog (empty for a note with no undo history
        // yet), plus the content the buffer was just loaded with as the
        // starting baseline. Shared with VimState (u / Ctrl+r) and
        // wire_autosave (which is what actually commits and persists new
        // entries) below -- one instance for the life of this tab.
        let document_history = Rc::new(RefCell::new(DocumentHistory::new(
            serialize_buffer(&buffer),
            note.history().to_vec(),
        )));

        // wire_autosave is connected only after the buffer is fully loaded --
        // both the `changed` signal from deserialise's own inserts and the
        // `apply-tag` signal from restoring highlights above would otherwise
        // schedule a pointless save of the note onto itself the moment it's
        // opened.
        wire_autosave(
            &buffer,
            note.note_identifier().to_string(),
            note.title().to_string(),
            note.source_path().map(|p| p.to_path_buf()),
            document_history.clone(),
        );
        wire_image_drop(&view, note.note_identifier().to_string());
        wire_clipboard_image_paste(&view, note.note_identifier().to_string());
        wire_url_paste(&view);

        // Syntax highlighting — debounced 400 ms
        wire_syntax_highlighting(&view);

        // search bar — created BEFORE wire_vim so we can pass search callbacks
        let search = Rc::new(SearchBar::new(buffer.clone()));

        let vim_state = VimState::new(note.note_identifier(), document_history);
        {
            let s1 = search.clone();
            let s2 = search.clone();
            let s3 = search.clone();
            wire_vim(
                &view,
                vim_state.clone(),
                on_mode_change,
                move || { s1.open(); },
                move || { s2.go_next(); },
                move || { s3.go_prev(); },
            );
        }

        let viewport = ScrolledWindow::builder()
            .child(&view)
            .vexpand(true)
            .hexpand(true)
            .build();

        let root = Box::builder()
            .orientation(Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .build();
        root.append(search.widget());
        root.append(&viewport);

        Self {
            root,
            view,
            search,
            buffer,
            note_identifier: note.note_identifier().to_string(),
            vim: vim_state,
        }
    }

    pub fn widget(&self)          -> &Box        { &self.root }
    pub fn open_search(&self)                    { self.search.open(); }
    pub fn close_search(&self)                   { self.search.close(); }
    pub fn search_is_open(&self)  -> bool        { self.search.is_open() }
    pub fn buffer(&self)          -> &TextBuffer { &self.buffer }
    pub fn note_identifier(&self) -> &str        { &self.note_identifier }

    /// Open a FileChooserNative for images and insert at cursor.
    /// Called by TabController when the "Image" button in the status bar is clicked.
    pub fn trigger_attach_image(&self) {
        let win = self.root.root().and_then(|r| r.downcast::<gtk::Window>().ok());
        open_image_chooser(&self.buffer, &self.view, &self.note_identifier, win.as_ref());
    }

    /// Open a FileChooserNative for videos and insert at cursor.
    /// Called by TabController when the "Video" button in the status bar is clicked.
    pub fn trigger_attach_video(&self) {
        let win = self.root.root().and_then(|r| r.downcast::<gtk::Window>().ok());
        open_video_chooser(&self.buffer, &self.view, &self.note_identifier, win.as_ref());
    }
}


// ── image file chooser ────────────────────────────────────────────────────────

fn open_image_chooser(
    buffer:  &TextBuffer,
    view:    &TextView,
    note_id: &str,
    parent:  Option<&gtk::Window>,
) {
    let dlg = FileChooserNative::new(
        Some("Attach Image"),
        parent,
        FileChooserAction::Open,
        Some("Attach"),
        Some("Cancel"),
    );

    let filter = FileFilter::new();
    filter.set_name(Some("Images"));
    for mime in ["image/png", "image/jpeg", "image/gif", "image/webp", "image/avif"] {
        filter.add_mime_type(mime);
    }
    dlg.add_filter(&filter);

    let buf  = buffer.clone();
    let vw   = view.clone();
    let id   = note_id.to_string();
    let weak = dlg.downgrade();

    dlg.connect_response(move |_, resp| {
        let d = match weak.upgrade() { Some(x) => x, None => return };
        if resp == ResponseType::Accept {
            if let Some(path) = d.file().and_then(|f| f.path()) {
                if let Ok(stored) = import_image(&id, &path) {
                    if let Some(fname) = filename_from_path(&stored) {
                        let mut iter = buf.iter_at_mark(&buf.get_insert());
                        let _ = insert_image_paintable_tagged(&buf, &vw, &mut iter, &stored, &fname);
                        vw.scroll_mark_onscreen(&buf.get_insert());
                    }
                }
            }
        }
        d.hide();
    });

    dlg.show();
}


// ── video file chooser ────────────────────────────────────────────────────────

fn open_video_chooser(
    buffer:  &TextBuffer,
    view:    &TextView,
    note_id: &str,
    parent:  Option<&gtk::Window>,
) {
    let dlg = FileChooserNative::new(
        Some("Attach Video"),
        parent,
        FileChooserAction::Open,
        Some("Attach"),
        Some("Cancel"),
    );

    let filter = FileFilter::new();
    filter.set_name(Some("Videos"));
    for mime in [
        "video/mp4", "video/x-matroska", "video/webm",
        "video/quicktime", "video/x-msvideo", "video/ogg",
    ] { filter.add_mime_type(mime); }
    dlg.add_filter(&filter);

    let buf  = buffer.clone();
    let vw   = view.clone();
    let id   = note_id.to_string();
    let weak = dlg.downgrade();

    dlg.connect_response(move |_, resp| {
        let d = match weak.upgrade() { Some(x) => x, None => return };
        if resp == ResponseType::Accept {
            if let Some(path) = d.file().and_then(|f| f.path()) {
                if let Ok(stored) = import_video(&id, &path) {
                    if let Some(fname) = filename_from_path(&stored) {
                        let mut iter = buf.iter_at_mark(&buf.get_insert());
                        insert_video_anchor(&buf, &vw, &mut iter, &stored, &fname);
                        vw.scroll_mark_onscreen(&buf.get_insert());
                    }
                }
            }
        }
        d.hide();
    });

    dlg.show();
}
