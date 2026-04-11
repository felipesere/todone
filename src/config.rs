use std::path::PathBuf;

pub fn notes_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TODONE_DIR") {
        return PathBuf::from(dir);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".todone").join("notes")
}
