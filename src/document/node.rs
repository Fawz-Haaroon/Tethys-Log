#[derive(Debug, Clone)]
pub enum NoteNode {
    Paragraph(String),
    Image(ImageNode),
}

#[derive(Debug, Clone)]
pub struct ImageNode {
    pub filesystem_path: String,
}
