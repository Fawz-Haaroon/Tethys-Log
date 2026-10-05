// ImageWidget — inline image displayed as a child anchor in the TextView.
//
// Sizing: falls back to the image's own natural size (capped at 1400px
// wide so one huge photo can't blow out the canvas) only when no size was
// recovered from the note itself. codec.rs passes that recovered size in
// as `initial_size` on every load -- see codec.rs's DIM_SEP for where it
// lives in the .tlog marker. Once built, all resizing goes through
// MediaFrame; this file no longer tracks size itself at all.

use std::path::{Path, PathBuf};

use gtk::{gdk, gdk_pixbuf, prelude::*, Overlay, Picture};

use crate::editor::canvas::{media_frame::MediaFrame, viewer};

const MIN_W: i32 = 120;
const MIN_H: i32 = 80;
const NATURAL_CAP_W: i32 = 1400;

pub struct ImageWidget {
    frame: MediaFrame,
}

impl ImageWidget {
    pub fn new(path: &Path, initial_size: Option<(i32, i32)>) -> Self {
        let pixbuf = gdk_pixbuf::Pixbuf::from_file(path).ok();

        let natural = pixbuf
            .as_ref()
            .map(|p| {
                let (w, h) = (p.width(), p.height());
                if w <= NATURAL_CAP_W {
                    (w, h)
                } else {
                    let scale = NATURAL_CAP_W as f64 / w as f64;
                    (NATURAL_CAP_W, (h as f64 * scale).round() as i32)
                }
            })
            .unwrap_or((800, 600));
        let (init_w, init_h) = initial_size.unwrap_or(natural);

        let picture = Picture::new();
        if let Some(pb) = &pixbuf {
            let texture = gdk::Texture::for_pixbuf(pb);
            picture.set_paintable(Some(&texture));
        }
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_can_shrink(true);

        let owned_path: PathBuf = path.to_path_buf();
        let frame = MediaFrame::new(&picture, init_w, init_h, MIN_W, MIN_H);
        frame.widget().add_css_class("image-widget");
        frame.connect_expand(move || viewer::show_image(&owned_path));

        Self { frame }
    }

    pub fn widget(&self) -> &Overlay {
        self.frame.widget()
    }
}

pub const IMAGE_CSS: &str = r#"
.image-widget {
    margin-top: 6px;
    margin-bottom: 2px;
}
"#;
