use std::{fs, path::{Path, PathBuf}};

use crate::storage::paths::data_dir;

// images live in <data_dir>/images/<note_id>/<filename>
// copying on insert rather than referencing the original path means
// the note doesn't break if the user moves or deletes the source file.
pub fn import_image(note_identifier: &str, source: &Path) -> Result<PathBuf, ImageImportError> {
    let filename = source.file_name()
        .ok_or(ImageImportError::NoFilename)?;

    let dest_dir = image_dir_for(note_identifier);
    fs::create_dir_all(&dest_dir).map_err(ImageImportError::Io)?;

    let dest = dest_dir.join(filename);

    // don't re-copy if already in our image store — happens if user pastes the same file twice
    if dest == source {
        return Ok(dest);
    }

    fs::copy(source, &dest).map_err(ImageImportError::Io)?;
    Ok(dest)
}

pub fn image_dir_for(note_identifier: &str) -> PathBuf {
    data_dir().join("images").join(note_identifier)
}

pub fn delete_images_for(note_identifier: &str) {
    let _ = fs::remove_dir_all(image_dir_for(note_identifier));
}

#[derive(Debug)]
pub enum ImageImportError {
    NoFilename,
    Io(std::io::Error),
}

impl std::fmt::Display for ImageImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoFilename => write!(f, "source path has no filename component"),
            Self::Io(e) => write!(f, "io error copying image: {e}"),
        }
    }
}
