use gtk::{prelude::*, TextBuffer, TextIter};

pub struct SearchState {
    pub query: String,
    pub case_sensitive: bool,
}

impl SearchState {
    pub fn new(query: impl Into<String>) -> Self {
        Self { query: query.into(), case_sensitive: false }
    }

    pub fn find_next(&self, _buffer: &TextBuffer, from: &TextIter) -> Option<(TextIter, TextIter)> {
        search_from(from, &self.query, self.case_sensitive, Direction::Forward)
    }

    pub fn find_prev(&self, _buffer: &TextBuffer, from: &TextIter) -> Option<(TextIter, TextIter)> {
        search_from(from, &self.query, self.case_sensitive, Direction::Backward)
    }

    pub fn replace_current(&self, buffer: &TextBuffer, replacement: &str) -> bool {
        let (mut start, mut end) = match buffer.selection_bounds() {
            Some(b) => b,
            None => return false,
        };
        let selected = buffer.text(&start, &end, false).to_string();
        let matches = if self.case_sensitive {
            selected == self.query
        } else {
            selected.to_lowercase() == self.query.to_lowercase()
        };
        if matches {
            buffer.begin_user_action();
            buffer.delete(&mut start, &mut end);
            buffer.insert(&mut start, replacement);
            buffer.end_user_action();
            true
        } else {
            false
        }
    }

    pub fn replace_all(&self, buffer: &TextBuffer, replacement: &str) -> usize {
        let mut count = 0;
        let mut from = buffer.start_iter();
        buffer.begin_user_action();
        while let Some((mut start, mut end)) = search_from(&from, &self.query, self.case_sensitive, Direction::Forward) {
            let next_offset = start.offset() + replacement.len() as i32;
            buffer.delete(&mut start, &mut end);
            buffer.insert(&mut start, replacement);
            from = buffer.iter_at_offset(next_offset);
            count += 1;
        }
        buffer.end_user_action();
        count
    }
}

enum Direction { Forward, Backward }

fn search_from(
    from: &TextIter,
    query: &str,
    case_sensitive: bool,
    dir: Direction,
) -> Option<(TextIter, TextIter)> {
    if query.is_empty() { return None; }

    let flags = if case_sensitive {
        gtk::TextSearchFlags::TEXT_ONLY
    } else {
        gtk::TextSearchFlags::TEXT_ONLY | gtk::TextSearchFlags::CASE_INSENSITIVE
    };

    match dir {
        Direction::Forward => from.forward_search(query, flags, None),
        Direction::Backward => from.backward_search(query, flags, None),
    }
}
