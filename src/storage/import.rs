// Foreign-file import — opens any text-based file in Tethys-Log.
//
// When the user opens a .txt, .md, .rs, or similar file, we make a copy in
// ~/Tethys-Log/imports/ as a record, and also register it in notes/ so the
// canvas can load it.  The original file is never modified.

use std::{fs, path::Path};

use crate::{
    document::{id::new_note_id, node::NoteNode, note::NoteDocument},
    storage::{notes::NoteStore, paths::imports_dir},
};

#[derive(Debug)]
pub enum ImportError {
    NotUtf8,
    Io(std::io::Error),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotUtf8  => write!(f, "file is not valid UTF-8"),
            Self::Io(e)    => write!(f, "io error reading file: {e}"),
        }
    }
}

/// Reads a foreign text file and registers it as a new note.
///
/// Returns a NoteDocument ready to open in a tab.
/// The original file is never modified.
pub fn import_text_file(source: &Path) -> Result<NoteDocument, ImportError> {
    let content = fs::read_to_string(source).map_err(|e| {
        if e.kind() == std::io::ErrorKind::InvalidData {
            ImportError::NotUtf8
        } else {
            ImportError::Io(e)
        }
    })?;

    let title = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Imported file")
        .to_string();

    let id_str = new_note_id();

    // keep an archive copy in imports/ for reference
    let imports = imports_dir();
    if let Ok(()) = fs::create_dir_all(&imports) {
        let archive = imports.join(format!("{id_str}.tlog"));
        let _ = fs::write(&archive, &content);
    }

    // save to notes/ so the canvas can load it normally
    let mut doc = NoteDocument::new(id_str, title);
    doc.replace_content(vec![NoteNode::Paragraph(content)]);
    NoteStore::persist(&doc);

    Ok(doc)
}

/// Returns the imports directory path for display purposes.
pub fn imports_location() -> std::path::PathBuf {
    imports_dir()
}
