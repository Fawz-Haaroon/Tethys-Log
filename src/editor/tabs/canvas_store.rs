use std::collections::HashMap;

use crate::editor::canvas::EditorCanvas;

// keeps EditorCanvas instances alive after the GTK stack takes ownership
// of the widget tree. indexed by note_identifier so controller.rs can
// reach the active canvas for search, buffer access, etc.
pub struct CanvasStore {
    canvases: HashMap<String, EditorCanvas>,
}

impl CanvasStore {
    pub fn new() -> Self {
        Self { canvases: HashMap::new() }
    }

    pub fn insert(&mut self, canvas: EditorCanvas) {
        self.canvases.insert(canvas.note_identifier().to_string(), canvas);
    }

    pub fn remove(&mut self, note_identifier: &str) {
        self.canvases.remove(note_identifier);
    }

    pub fn get(&self, note_identifier: &str) -> Option<&EditorCanvas> {
        self.canvases.get(note_identifier)
    }
}
