// Embed undo — restores image/video/embed widgets after vim 'u' / Ctrl+Z.
//
// GTK4's TextBuffer undo history tracks text insertions and deletions but does
// NOT track child-anchor widget state.  When an embed (U+FFFC child anchor)
// is deleted and then undo'd, GTK restores the U+FFFC character — with its
// original "img-path:" / "video-path:" / "embed-src:" tag intact — but as
// plain text rather than a live child anchor.  The widget is therefore gone.
//
// restore_orphaned_embeds() is called after every successful buffer.undo().
// It scans the buffer for "naked" U+FFFC chars (child_anchor() == None but
// with an embed tag), then atomically replaces each one with a real child
// anchor + widget inside a begin_irreversible_action() block so the repair
// itself does not pollute the undo stack.

use std::path::PathBuf;

use gtk::{prelude::*, TextBuffer, TextView};

use crate::editor::canvas::{
    codec::{image_dir_for_note, video_dir_for_note},
    embed::watch_url_from_embed_src,
    embed_widget::EmbedCard,
    image_widget::ImageWidget,
    video_widget::VideoWidget,
};

// ── Internal embed-type enum ──────────────────────────────────────────────────

enum EmbedRestore {
    Image(PathBuf),  // absolute path to the stored image file
    Video(PathBuf),  // absolute path to the stored video file
    Embed(String),   // embed-src URL stored in the tag
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Call this immediately after `buffer.undo()` in every undo handler.
///
/// Finds any U+FFFC characters whose child anchor is gone (i.e. undo restored
/// the character as plain text but not the widget), reads their embed tag to
/// identify type and path/URL, then replaces them with a live anchor + widget.
///
/// The repair is wrapped in `begin_irreversible_action` so it does not add new
/// entries to the undo history.
pub fn restore_orphaned_embeds(
    buffer:          &TextBuffer,
    view:            &TextView,
    note_identifier: &str,
) {
    let orphans = collect_orphans(buffer, note_identifier);
    if orphans.is_empty() { return; }

    // Process in reverse offset order so earlier positions are not shifted by
    // later inserts (each replace is net-neutral in character count: delete
    // one FFFC, insert one anchor FFFC; but still safer to go in reverse).
    buffer.begin_irreversible_action();
    for (offset, kind) in orphans.into_iter().rev() {
        restore_one(buffer, view, offset, kind);
    }
    buffer.end_irreversible_action();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn collect_orphans(
    buffer:          &TextBuffer,
    note_identifier: &str,
) -> Vec<(i32, EmbedRestore)> {
    let mut result = Vec::new();
    let mut iter   = buffer.start_iter();
    let end        = buffer.end_iter();

    while iter != end {
        // U+FFFC with no live child anchor = undo-restored plain text embed
        if iter.char() == '\u{FFFC}' && iter.child_anchor().is_none() {
            let tags = iter.tags();
            let kind = tags.iter().find_map(|tag| {
                let name = tag.name()?;
                if let Some(fname) = name.strip_prefix("img-path:") {
                    let path = image_dir_for_note(note_identifier).join(fname);
                    Some(EmbedRestore::Image(path))
                } else if let Some(fname) = name.strip_prefix("video-path:") {
                    let path = video_dir_for_note(note_identifier).join(fname);
                    Some(EmbedRestore::Video(path))
                } else if let Some(src) = name.strip_prefix("embed-src:") {
                    Some(EmbedRestore::Embed(src.to_string()))
                } else {
                    None
                }
            });
            if let Some(k) = kind {
                result.push((iter.offset(), k));
            }
        }
        if !iter.forward_char() { break; }
    }
    result
}

fn restore_one(
    buffer: &TextBuffer,
    view:   &TextView,
    offset: i32,
    kind:   EmbedRestore,
) {
    // 1. Delete the orphaned plain-text FFFC
    let mut del_start = buffer.iter_at_offset(offset);
    let mut del_end   = buffer.iter_at_offset(offset + 1);
    buffer.delete(&mut del_start, &mut del_end);

    // 2. Insert a proper child anchor at the same position (net char-count: 0)
    let mut ins    = buffer.iter_at_offset(offset);
    let anchor     = buffer.create_child_anchor(&mut ins);

    // 3. Apply the embed-identity tag to the new FFFC so future serialization
    //    and future undo passes can still identify it.
    {
        let tag_start = buffer.iter_at_offset(offset);
        let tag_end   = buffer.iter_at_offset(offset + 1);
        let tag_name  = embed_tag_name(&kind);
        let tag = buffer.tag_table().lookup(&tag_name)
            .unwrap_or_else(|| buffer.create_tag(Some(&tag_name), &[]).unwrap());
        buffer.apply_tag(&tag, &tag_start, &tag_end);
    }

    // 4. Create the widget and attach it to the anchor
    match kind {
        EmbedRestore::Image(path) => {
            let w = ImageWidget::new(&path);
            view.add_child_at_anchor(w.widget(), &anchor);
            w.widget().show();
        }
        EmbedRestore::Video(path) => {
            let w = VideoWidget::new(&path);
            view.add_child_at_anchor(w.widget(), &anchor);
            w.widget().show();
        }
        EmbedRestore::Embed(src) => {
            let watch = watch_url_from_embed_src(&src);
            let c = EmbedCard::new(&watch);
            view.add_child_at_anchor(c.widget(), &anchor);
            c.widget().show();
        }
    }
}

fn embed_tag_name(kind: &EmbedRestore) -> String {
    match kind {
        EmbedRestore::Image(path) => format!(
            "img-path:{}",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("")
        ),
        EmbedRestore::Video(path) => format!(
            "video-path:{}",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("")
        ),
        EmbedRestore::Embed(src) => format!("embed-src:{src}"),
    }
}
