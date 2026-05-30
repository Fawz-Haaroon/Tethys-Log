#[allow(unused_imports)]
use gtk::{CssProvider, gdk, prelude::*};

const FONT_DEFAULT_PT: f32 = 10.3;
const FONT_MAX_PT:     f32 = 28.0;
const FONT_MIN_PT:     f32 = 6.0;
const FONT_STEP_PT:    f32 = 0.5;

pub struct ZoomState {
    size:     std::cell::Cell<f32>,
    provider: CssProvider,
}

impl ZoomState {
    pub fn init() -> Self {
        let provider = CssProvider::new();
        let size     = std::cell::Cell::new(FONT_DEFAULT_PT);
        push_zoom_css(&provider, FONT_DEFAULT_PT);
        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
        Self { size, provider }
    }

    pub fn increase(&self) {
        let next = (self.size.get() + FONT_STEP_PT).min(FONT_MAX_PT);
        self.size.set(next);
        push_zoom_css(&self.provider, next);
    }

    pub fn decrease(&self) {
        let next = (self.size.get() - FONT_STEP_PT).max(FONT_MIN_PT);
        self.size.set(next);
        push_zoom_css(&self.provider, next);
    }

    pub fn reset(&self) {
        self.size.set(FONT_DEFAULT_PT);
        push_zoom_css(&self.provider, FONT_DEFAULT_PT);
    }
}

fn push_zoom_css(provider: &CssProvider, size: f32) {
    provider.load_from_data(&format!(
        "textview, text {{ font-size: {size:.1}pt; }}"
    ));
}

pub fn load_base_theme() {
    load_sheet(crate::editor::theme::BASE);
    load_sheet(crate::editor::theme::SEARCH);
    load_sheet(crate::editor::theme::VIM);
    load_sheet(crate::editor::theme::ATTACH);
    load_sheet(crate::editor::canvas::embed_widget::EMBED_CSS);
    load_sheet(crate::editor::canvas::video_widget::VIDEO_CSS);
}

fn load_sheet(css: &str) {
    let provider = CssProvider::new();
    provider.load_from_data(css);
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
