#[allow(dead_code)]
use std::path::PathBuf;

use gtk::{prelude::*, TextBuffer, TextIter};

use crate::{
    document::node::ImageNode,
    storage::image_store::{import_image, ImageImportError},
};

pub fn insert_image_at(
    buffer: &TextBuffer,
    iter: &mut TextIter,
    note_identifier: &str,
    source_path: PathBuf,
) -> Result<ImageNode, ImageInsertError> {
    let stored = import_image(note_identifier, &source_path)
        .map_err(ImageInsertError::Import)?;

    let placeholder = format!("\n[image:{}]\n", stored.to_string_lossy());
    buffer.insert(iter, &placeholder);

    Ok(ImageNode {
        filesystem_path: stored.to_string_lossy().into_owned(),
    })
}

pub fn insert_image_at_cursor(
    buffer: &TextBuffer,
    note_identifier: &str,
    source_path: PathBuf,
) -> Result<ImageNode, ImageInsertError> {
    let mut iter = buffer.iter_at_mark(&buffer.get_insert());
    insert_image_at(buffer, &mut iter, note_identifier, source_path)
}

#[derive(Debug)]
pub enum ImageInsertError {
    Import(ImageImportError),
}

impl std::fmt::Display for ImageInsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Import(e) => write!(f, "failed to import image: {e}"),
        }
    }
}
