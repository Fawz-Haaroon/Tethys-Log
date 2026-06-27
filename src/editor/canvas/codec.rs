use std::path::PathBuf;

use gtk::{prelude::*, TextBuffer, TextView};

use crate::editor::canvas::{
    embed::{parse_embed_tag, watch_url_from_embed_src, EMBED_OPEN, EMBED_TAG},
    embed_widget::EmbedCard,
    image_widget::ImageWidget,
};

const VIDEO_OPEN: char = '\x02';
const VIDEO_TAG:  &str = "video:";

const IMG_OPEN: char = '\x00';
const IMG_TAG:  &str = "img:";

pub fn serialize_buffer(buffer: &TextBuffer) -> String {
    let mut out  = String::new();
    let mut iter = buffer.start_iter();
    let end      = buffer.end_iter();

    while iter != end {
        let ch = iter.char();

        if ch == '\u{FFFC}' {
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
                out.push(IMG_OPEN);
            } else if let Some(src) = embed_src {
                out.push(EMBED_OPEN);
                out.push_str(EMBED_TAG);
                out.push_str(&src);
                out.push(EMBED_OPEN);
            } else if let Some(p) = video_path {
                out.push(VIDEO_OPEN);
                out.push_str(VIDEO_TAG);
                out.push_str(&p);
                out.push(VIDEO_OPEN);
            }
            // orphaned FFFC — drop
        } else {
            out.push(ch);
        }

        if !iter.forward_char() { break; }
    }

    out
}

pub fn deserialise_into_buffer(
    raw:       &str,
    buffer:    &TextBuffer,
    view:      &TextView,
    image_dir: &std::path::Path,
) {
    buffer.set_text("");
    let mut iter = buffer.end_iter();
    let mut rest = raw;

    while !rest.is_empty() {
        let img_pos   = rest.find(IMG_OPEN);
        let embed_pos = rest.find(EMBED_OPEN);
        let video_pos = rest.find(VIDEO_OPEN);

        let next: Option<(usize, u8)> = [
            img_pos.map(|p|   (p, 0u8)),
            embed_pos.map(|p| (p, 1u8)),
            video_pos.map(|p| (p, 2u8)),
        ]
        .into_iter().flatten().min_by_key(|(pos, _)| *pos);

        let (marker_start, kind) = match next {
            None    => { buffer.insert(&mut iter, rest); break; }
            Some(n) => n,
        };

        if marker_start > 0 {
            buffer.insert(&mut iter, &rest[..marker_start]);
        }

        let sentinel   = match kind { 0 => IMG_OPEN, 1 => EMBED_OPEN, _ => VIDEO_OPEN };
        let after_open = &rest[marker_start + sentinel.len_utf8()..];

        match after_open.find(sentinel) {
            None => { buffer.insert(&mut iter, after_open); break; }
            Some(close) => {
                let tag_content = &after_open[..close];
                rest = &after_open[close + sentinel.len_utf8()..];

                match kind {
                    0 => {
                        if let Some(filename) = tag_content.strip_prefix(IMG_TAG) {
                            let full_path = image_dir.join(filename);
                            let _ = insert_image_paintable_tagged(buffer, view, &mut iter, &full_path, filename);
                        }
                    }
                    1 => {
                        if let Some(src) = parse_embed_tag(tag_content) {
                            insert_embed_anchor(buffer, view, &mut iter, src);
                        }
                    }
                    _ => {
                        if let Some(filename) = tag_content.strip_prefix(VIDEO_TAG) {
                            // derive the note id from the image_dir path (last component)
                            // then resolve the video path through the canonical helper
                            let note_id = image_dir
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            let video_dir = crate::storage::paths::videos_dir_for(note_id);
                            let full_path = video_dir.join(filename);
                            insert_video_anchor(buffer, view, &mut iter, &full_path, filename);
                        }
                    }
                }
            }
        }
    }
}

pub fn insert_embed_anchor(
    buffer:    &TextBuffer,
    view:      &TextView,
    iter:      &mut gtk::TextIter,
    embed_src: &str,
) {
    // embed_src may be a YouTube /embed/ URL (stored from old notes) or a plain
    // watch/page URL (stored from new notes).  Derive the canonical watch URL.
    let watch_url = watch_url_from_embed_src(embed_src);

    let tag_name = format!("embed-src:{embed_src}");
    let tag = match buffer.tag_table().lookup(&tag_name) {
        Some(t) => t,
        None    => buffer.create_tag(Some(&tag_name), &[]).unwrap(),
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

    let card = EmbedCard::new(&watch_url);
    view.add_child_at_anchor(card.widget(), &anchor);
    card.widget().show();
}

pub fn insert_image_paintable_tagged(
    buffer:          &TextBuffer,
    view:            &TextView,
    iter:            &mut gtk::TextIter,
    full_path:       &std::path::Path,
    tag_name_suffix: &str,
) -> Result<(), String> {
    let tag_name = format!("img-path:{tag_name_suffix}");
    let tag = match buffer.tag_table().lookup(&tag_name) {
        Some(t) => t,
        None    => buffer.create_tag(Some(&tag_name), &[]).unwrap(),
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

    let widget = ImageWidget::new(full_path);
    view.add_child_at_anchor(widget.widget(), &anchor);
    widget.widget().show();

    Ok(())
}

pub fn insert_video_anchor(
    buffer:   &TextBuffer,
    view:     &TextView,
    iter:     &mut gtk::TextIter,
    path:     &std::path::Path,
    filename: &str,
) {
    use crate::editor::canvas::video_widget::VideoWidget;

    let tag_name = format!("video-path:{filename}");
    let tag = match buffer.tag_table().lookup(&tag_name) {
        Some(t) => t,
        None    => buffer.create_tag(Some(&tag_name), &[]).unwrap(),
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

    let widget = VideoWidget::new(path);
    view.add_child_at_anchor(widget.widget(), &anchor);
    widget.widget().show();
}

fn tag_fffc_at(buffer: &TextBuffer, tag: &gtk::TextTag, from_offset: i32, to_offset: i32) {
    let mut it   = buffer.iter_at_offset(from_offset);
    let     stop = buffer.iter_at_offset(to_offset);

    while it != stop {
        if it.char() == '\u{FFFC}' {
            let mut tag_end = it;
            tag_end.forward_char();
            buffer.apply_tag(tag, &it, &tag_end);
            return;
        }
        if !it.forward_char() { break; }
    }
}

pub fn filename_from_path(path: &std::path::Path) -> Option<String> {
    path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string())
}

pub fn image_dir_for_note(note_identifier: &str) -> PathBuf {
    crate::storage::paths::images_dir_for(note_identifier)
}

pub fn video_dir_for_note(note_identifier: &str) -> PathBuf {
    crate::storage::paths::videos_dir_for(note_identifier)
}
