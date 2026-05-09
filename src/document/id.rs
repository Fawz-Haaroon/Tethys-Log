// microsecond timestamp as note ID — cheap, unique enough for single-user local files,
// no UUID dep needed. collision window is ~1µs which requires two notes created in the
// same scheduler tick; acceptable given this is a single-threaded desktop app.
pub fn new_note_id() -> String {
    format!("note-{}", epoch_micros())
}

fn epoch_micros() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}
