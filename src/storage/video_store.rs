// Local video storage — mirrors the image_store pattern.
//
// Videos live in ~/Tethys-Log/media/videos/<note_id>/<filename>.
// Copying on insert means the note survives moves or deletions of the source.

use std::{fs, path::{Path, PathBuf}};

use crate::storage::paths::videos_dir_for;

pub fn import_video(note_identifier: &str, source: &Path) -> Result<PathBuf, VideoImportError> {
    let filename = source.file_name().ok_or(VideoImportError::NoFilename)?;

    let dest_dir = video_dir_for(note_identifier);
    fs::create_dir_all(&dest_dir).map_err(VideoImportError::Io)?;

    let dest = dest_dir.join(filename);

    if dest == source {
        return Ok(dest);
    }

    fs::copy(source, &dest).map_err(VideoImportError::Io)?;
    Ok(dest)
}

pub fn video_dir_for(note_identifier: &str) -> PathBuf {
    videos_dir_for(note_identifier)
}

pub fn delete_videos_for(note_identifier: &str) {
    let _ = fs::remove_dir_all(video_dir_for(note_identifier));
}

#[derive(Debug)]
pub enum VideoImportError {
    NoFilename,
    Io(std::io::Error),
}

impl std::fmt::Display for VideoImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoFilename => write!(f, "source path has no filename component"),
            Self::Io(e)      => write!(f, "io error copying video: {e}"),
        }
    }
}
