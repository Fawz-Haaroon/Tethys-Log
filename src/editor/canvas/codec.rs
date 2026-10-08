use std::path::PathBuf;

use gtk::{prelude::*, TextBuffer, TextView};

use crate::editor::canvas::{
    embed::{parse_embed_tag, watch_url_from_embed_src, EMBED_OPEN, EMBED_OPEN_LEGACY, EMBED_TAG},
    embed_widget::EmbedCard,
    highlight::{highlight_tag_names, HL_CLOSE, HL_OPEN},
    image_widget::ImageWidget,
};

// Sentinel characters bracket an embedded image/video/embed reference in the
// serialized buffer text, e.g. `\u{E000}img:photo.png\u{E000}`. They live in
// the Unicode Private Use Area -- valid, unremarkable UTF-8 text -- rather
// than as C0 control characters. A NUL, SOH, or STX byte anywhere in a file
// is exactly what makes `file`, `git`, `less`, and GitHub's own viewer treat
// it as binary instead of text, which is what every .tlog note looked like
// to every tool except Tethys-Log itself before this change.
//
// The _LEGACY constants are the original control-character sentinels.
// deserialise_into_buffer still recognises them on read so notes saved
// before this fix keep opening correctly; every save from here on writes
// only the new sentinels, so each note upgrades itself the first time it's
// touched. Read old-or-new, write new-only -- the standard shape for a
// backward-compatible file-format migration.
//
// HL_OPEN / HL_CLOSE (defined in highlight.rs, next to the palette they
// serialise) use the same E0xx range for the same reason and slot into the
// marker search below alongside img/embed/video.
const VIDEO_OPEN: char = '\u{E002}';

// Separates a filename from an optional trailing "WxH" inside an img/video
// marker's payload, e.g. \u{E000}img:photo.png\u{E006}640x480\u{E000}. Not
// E005 -- that one's already storage::notes::HISTORY_MARKER, and reusing
// it here would make split_document_and_history find the first resized
// image in a note and mistake everything after it for the undo log.
const DIM_SEP: char = '\u{E006}';
const VIDEO_OPEN_LEGACY: char = '\x02';
const VIDEO_TAG: &str = "video:";

const IMG_OPEN: char = '\u{E000}';
const IMG_OPEN_LEGACY: char = '\x00';
const IMG_TAG: &str = "img:";

pub fn serialize_buffer(buffer: &TextBuffer) -> String {
    let mut out = String::new();
    let mut iter = buffer.start_iter();
    let end = buffer.end_iter();
    // Highlight tag names currently "open" in the output -- i.e. the tags
    // that applied to the last character written. When the set of tags at
    // the current position differs from this, a run has ended and/or begun,
    // and we bracket the transition with HL_CLOSE / HL_OPEN markers. See
    // highlight.rs for the marker format.
    let mut active_highlight: Vec<String> = Vec::new();

    while iter != end {
        let ch = iter.char();

        if ch == '\u{FFFC}' {
            // An embedded widget never carries highlight tags in the saved
            // file (only the plain-text runs around it do), so close
            // whatever highlight run was open before writing its marker.
            if !active_highlight.is_empty() {
                out.push(HL_CLOSE);
                active_highlight.clear();
            }

            let tags = iter.tags();

            let img_path = tags.iter().find_map(|tag| {
                let name = tag.name()?;
                name.strip_prefix("img-path:").map(|s| s.to_string())
            });

            let embed_src = tags.iter().find_map(|tag| {
                let name = tag.name()?;
                name.strip_prefix("embed-src:").map(|s| s.to_string())
            });

            let video_path = tags.iter().find_map(|tag| {
                let name = tag.name()?;
                name.strip_prefix("video-path:").map(|s| s.to_string())
            });

            if let Some(p) = img_path {
                out.push(IMG_OPEN);
                out.push_str(IMG_TAG);
                out.push_str(&p);
                push_dims(&mut out, widget_size_at(&iter));
                out.push(IMG_OPEN);
            } else if let Some(src) = embed_src {
                out.push(EMBED_OPEN);
                out.push_str(EMBED_TAG);
                out.push_str(&src);
                push_dims(&mut out, widget_size_at(&iter));
                out.push(EMBED_OPEN);
            } else if let Some(p) = video_path {
                out.push(VIDEO_OPEN);
                out.push_str(VIDEO_TAG);
                out.push_str(&p);
                push_dims(&mut out, widget_size_at(&iter));
                out.push(VIDEO_OPEN);
            }
            // orphaned FFFC — drop
        } else {
            let current_highlight = highlight_tag_names(&iter);
            if current_highlight != active_highlight {
                if !active_highlight.is_empty() {
                    out.push(HL_CLOSE);
                }
                if !current_highlight.is_empty() {
                    out.push(HL_OPEN);
                    out.push_str(&current_highlight.join(","));
                    out.push(HL_OPEN);
                }
                active_highlight = current_highlight;
            }
            out.push(ch);
        }

        if !iter.forward_char() {
            break;
        }
    }

    // The buffer ended mid-run (the last characters were highlighted) --
    // close it so the marker pair stays balanced.
    if !active_highlight.is_empty() {
        out.push(HL_CLOSE);
    }

    out
}

/// Reads the current size of whatever widget is anchored at `iter` --
/// image and video widgets always carry an explicit size_request, set at
/// construction and kept current by MediaFrame's drag-resize, so this is
/// the one true source for "what size did the user leave this at", read
/// fresh at save time rather than tracked in a second place that could
/// drift from what's actually on screen.
/// Reads the current size of whatever widget is anchored at `iter`. Image
/// and video widgets are themselves the resizable frame, so their own
/// size_request is the answer directly. An embed card is not -- its outer
/// box (header, play button, accent stripe) is a fixed-width panel, not
/// what the user resized -- so this also checks one level down for a
/// nested frame (the same "media-frame" CSS class media_frame.rs applies)
/// and reads that instead when the anchored widget itself isn't one.
/// Either way this is read fresh at save time rather than tracked in a
/// second place that could drift from what's actually on screen.
fn widget_size_at(iter: &gtk::TextIter) -> Option<(i32, i32)> {
    let anchor = iter.child_anchor()?;
    let widget = anchor.widgets().into_iter().next()?;
    let sized = if widget.has_css_class("media-frame") {
        widget
    } else {
        find_media_frame(&widget)?
    };
    let (w, h) = (sized.width_request(), sized.height_request());
    if w > 0 && h > 0 {
        Some((w, h))
    } else {
        None
    }
}

/// Depth-first search for a descendant carrying the "media-frame" CSS
/// class -- used to find an embed card's inline player once it's playing;
/// returns None for a card that's still showing its unplayed preview,
/// which correctly means "no explicit size yet, use the default."
fn find_media_frame(widget: &gtk::Widget) -> Option<gtk::Widget> {
    let mut child = widget.first_child();
    while let Some(w) = child {
        if w.has_css_class("media-frame") {
            return Some(w);
        }
        if let Some(found) = find_media_frame(&w) {
            return Some(found);
        }
        child = w.next_sibling();
    }
    None
}

fn push_dims(out: &mut String, size: Option<(i32, i32)>) {
    if let Some((w, h)) = size {
        out.push(DIM_SEP);
        out.push_str(&w.to_string());
        out.push('x');
        out.push_str(&h.to_string());
    }
}

/// Splits an img/video/embed marker's payload into the filename-or-url and
/// an optional parsed size. Anything that doesn't parse cleanly -- a note
/// saved before this field existed, or one that's been hand-edited into
/// something odd -- degrades to `None` rather than failing the whole load;
/// a missing size just means the widget falls back to its natural-size
/// default, the same as it always has.
fn split_payload_and_dims(content: &str) -> (&str, Option<(i32, i32)>) {
    let Some((payload, dims)) = content.split_once(DIM_SEP) else {
        return (content, None);
    };
    let Some((w, h)) = dims.split_once('x') else {
        return (payload, None);
    };
    match (w.parse::<i32>(), h.parse::<i32>()) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => (payload, Some((w, h))),
        _ => (payload, None),
    }
}

/// Finds the earlier of a sentinel's current and legacy form in `text`,
/// returning its byte offset and which literal character was found there.
/// The caller re-uses that exact character to find the matching close, so a
/// note written entirely in one form (the normal case -- see the migration
/// note above) round-trips correctly even though both forms are accepted.
fn nearest_sentinel(text: &str, current: char, legacy: char) -> Option<(usize, char)> {
    match (text.find(current), text.find(legacy)) {
        (Some(a), Some(b)) => Some(if a <= b { (a, current) } else { (b, legacy) }),
        (Some(a), None) => Some((a, current)),
        (None, Some(b)) => Some((b, legacy)),
        (None, None) => None,
    }
}

pub fn deserialise_into_buffer(
    raw: &str,
    buffer: &TextBuffer,
    view: &TextView,
    image_dir: &std::path::Path,
) {
    buffer.set_text("");
    let mut iter = buffer.end_iter();
    let mut rest = raw;
    // Highlight tag names active for whatever plain text comes next --
    // populated by an HL_OPEN marker, cleared by HL_CLOSE. Applied to each
    // run of plain text as it's inserted; see insert_text_with_highlight.
    let mut active_highlight: Vec<String> = Vec::new();

    while !rest.is_empty() {
        let img_hit = nearest_sentinel(rest, IMG_OPEN, IMG_OPEN_LEGACY);
        let embed_hit = nearest_sentinel(rest, EMBED_OPEN, EMBED_OPEN_LEGACY);
        let video_hit = nearest_sentinel(rest, VIDEO_OPEN, VIDEO_OPEN_LEGACY);
        let hl_open_hit = rest.find(HL_OPEN).map(|p| (p, HL_OPEN));
        let hl_close_hit = rest.find(HL_CLOSE).map(|p| (p, HL_CLOSE));

        let next: Option<(usize, char, u8)> = [
            img_hit.map(|(p, c)| (p, c, 0u8)),
            embed_hit.map(|(p, c)| (p, c, 1u8)),
            video_hit.map(|(p, c)| (p, c, 2u8)),
            hl_open_hit.map(|(p, c)| (p, c, 3u8)),
            hl_close_hit.map(|(p, c)| (p, c, 4u8)),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|(pos, _, _)| *pos);

        let (marker_start, sentinel, kind) = match next {
            None => {
                insert_text_with_highlight(buffer, &mut iter, rest, &active_highlight);
                break;
            }
            Some(n) => n,
        };

        if marker_start > 0 {
            insert_text_with_highlight(buffer, &mut iter, &rest[..marker_start], &active_highlight);
        }

        // HL_CLOSE is a standalone marker with no bracketed payload -- unlike
        // the other four kinds, there's nothing to find a matching close for.
        if kind == 4 {
            active_highlight.clear();
            rest = &rest[marker_start + sentinel.len_utf8()..];
            continue;
        }

        let after_open = &rest[marker_start + sentinel.len_utf8()..];

        match after_open.find(sentinel) {
            None => {
                insert_text_with_highlight(buffer, &mut iter, after_open, &active_highlight);
                break;
            }
            Some(close) => {
                let tag_content = &after_open[..close];
                rest = &after_open[close + sentinel.len_utf8()..];

                match kind {
                    0 => {
                        if let Some(rest) = tag_content.strip_prefix(IMG_TAG) {
                            let (filename, size) = split_payload_and_dims(rest);
                            let full_path = image_dir.join(filename);
                            let _ = insert_image_paintable_tagged(
                                buffer, view, &mut iter, &full_path, filename, size,
                            );
                        }
                    }
                    1 => {
                        if let Some(raw) = parse_embed_tag(tag_content) {
                            let (src, size) = split_payload_and_dims(raw);
                            insert_embed_anchor(buffer, view, &mut iter, src, size);
                        }
                    }
                    2 => {
                        if let Some(rest) = tag_content.strip_prefix(VIDEO_TAG) {
                            let (filename, size) = split_payload_and_dims(rest);
                            // derive the note id from the image_dir path (last component)
                            // then resolve the video path through the canonical helper
                            let note_id =
                                image_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
                            let video_dir = crate::storage::paths::videos_dir_for(note_id);
                            let full_path = video_dir.join(filename);
                            insert_video_anchor(
                                buffer, view, &mut iter, &full_path, filename, size,
                            );
                        }
                    }
                    _ => {
                        // kind 3 (HL_OPEN): tag_content is the comma-separated
                        // list of highlight tags that apply to the text
                        // between here and the next HL_CLOSE.
                        active_highlight = tag_content
                            .split(',')
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .collect();
                    }
                }
            }
        }
    }
}

/// Inserts `text` at `iter` and, if any highlight tags are currently active,
/// applies them to the just-inserted range. This is the deserialise-side
/// counterpart to serialize_buffer's active_highlight tracking: every plain
/// text insertion during load goes through here so a saved highlight comes
/// back exactly where it was, rather than only the sentinel-bracketed
/// widgets being restored.
///
/// The tags must already be registered on `buffer`'s tag table --
/// register_highlight_tags is called before deserialise_into_buffer in
/// surface.rs for exactly this reason. A tag name that isn't found (a
/// hand-edited file, or a palette entry from a future version) is skipped
/// rather than treated as an error; a missing highlight is a cosmetic loss,
/// not a reason to fail loading the note.
fn insert_text_with_highlight(
    buffer: &TextBuffer,
    iter: &mut gtk::TextIter,
    text: &str,
    active_highlight: &[String],
) {
    if text.is_empty() {
        return;
    }

    let start_offset = iter.offset();
    buffer.insert(iter, text);

    if active_highlight.is_empty() {
        return;
    }

    let start = buffer.iter_at_offset(start_offset);
    let end = buffer.iter_at_offset(iter.offset());
    for name in active_highlight {
        if let Some(tag) = buffer.tag_table().lookup(name) {
            buffer.apply_tag(&tag, &start, &end);
        }
    }
}

pub fn insert_embed_anchor(
    buffer: &TextBuffer,
    view: &TextView,
    iter: &mut gtk::TextIter,
    embed_src: &str,
    initial_size: Option<(i32, i32)>,
) {
    // embed_src may be a YouTube /embed/ URL (stored from old notes) or a plain
    // watch/page URL (stored from new notes).  Derive the canonical watch URL.
    let watch_url = watch_url_from_embed_src(embed_src);

    let tag_name = format!("embed-src:{embed_src}");
    let tag = match buffer.tag_table().lookup(&tag_name) {
        Some(t) => t,
        None => buffer.create_tag(Some(&tag_name), &[]).unwrap(),
    };

    if iter.offset() > 0 {
        let prev = buffer.iter_at_offset(iter.offset() - 1);
        if buffer.text(&prev, iter, false) != "\n" {
            buffer.insert(iter, "\n");
        }
    }

    let before_offset = iter.offset();
    let anchor = buffer.create_child_anchor(iter);
    buffer.insert(iter, "\n");

    tag_fffc_at(buffer, &tag, before_offset, iter.offset());

    let card = EmbedCard::new(&watch_url, initial_size);
    view.add_child_at_anchor(card.widget(), &anchor);
    card.widget().show();
}

pub fn insert_image_paintable_tagged(
    buffer: &TextBuffer,
    view: &TextView,
    iter: &mut gtk::TextIter,
    full_path: &std::path::Path,
    tag_name_suffix: &str,
    initial_size: Option<(i32, i32)>,
) -> Result<(), String> {
    let tag_name = format!("img-path:{tag_name_suffix}");
    let tag = match buffer.tag_table().lookup(&tag_name) {
        Some(t) => t,
        None => buffer.create_tag(Some(&tag_name), &[]).unwrap(),
    };

    if iter.offset() > 0 {
        let prev = buffer.iter_at_offset(iter.offset() - 1);
        if buffer.text(&prev, iter, false) != "\n" {
            buffer.insert(iter, "\n");
        }
    }

    let before_offset = iter.offset();
    let anchor = buffer.create_child_anchor(iter);
    buffer.insert(iter, "\n");

    tag_fffc_at(buffer, &tag, before_offset, iter.offset());

    let widget = ImageWidget::new(full_path, initial_size);
    view.add_child_at_anchor(widget.widget(), &anchor);
    widget.widget().show();

    Ok(())
}

pub fn insert_video_anchor(
    buffer: &TextBuffer,
    view: &TextView,
    iter: &mut gtk::TextIter,
    path: &std::path::Path,
    filename: &str,
    initial_size: Option<(i32, i32)>,
) {
    use crate::editor::canvas::video_widget::VideoWidget;

    let tag_name = format!("video-path:{filename}");
    let tag = match buffer.tag_table().lookup(&tag_name) {
        Some(t) => t,
        None => buffer.create_tag(Some(&tag_name), &[]).unwrap(),
    };

    if iter.offset() > 0 {
        let prev = buffer.iter_at_offset(iter.offset() - 1);
        if buffer.text(&prev, iter, false) != "\n" {
            buffer.insert(iter, "\n");
        }
    }

    let before_offset = iter.offset();
    let anchor = buffer.create_child_anchor(iter);
    buffer.insert(iter, "\n");

    tag_fffc_at(buffer, &tag, before_offset, iter.offset());

    let widget = VideoWidget::new(path, initial_size);
    view.add_child_at_anchor(widget.widget(), &anchor);
    widget.widget().show();
}

fn tag_fffc_at(buffer: &TextBuffer, tag: &gtk::TextTag, from_offset: i32, to_offset: i32) {
    let mut it = buffer.iter_at_offset(from_offset);
    let stop = buffer.iter_at_offset(to_offset);

    while it != stop {
        if it.char() == '\u{FFFC}' {
            let mut tag_end = it;
            tag_end.forward_char();
            buffer.apply_tag(tag, &it, &tag_end);
            return;
        }
        if !it.forward_char() {
            break;
        }
    }
}

pub fn filename_from_path(path: &std::path::Path) -> Option<String> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
}

pub fn image_dir_for_note(note_identifier: &str) -> PathBuf {
    crate::storage::paths::images_dir_for(note_identifier)
}

pub fn video_dir_for_note(note_identifier: &str) -> PathBuf {
    crate::storage::paths::videos_dir_for(note_identifier)
}
