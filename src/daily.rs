use std::path::{Path, PathBuf};

pub fn today_date() -> jiff::civil::Date {
    jiff::Zoned::now().date()
}

pub fn today_path(notes_dir: &Path) -> PathBuf {
    date_to_path(notes_dir, today_date())
}

pub fn date_to_path(notes_dir: &Path, date: jiff::civil::Date) -> PathBuf {
    notes_dir.join(format!("{}.md", date))
}

/// Find the most recent .md file in notes_dir whose name parses as a date
/// earlier than today.
pub fn prev_path(notes_dir: &Path) -> anyhow::Result<Option<PathBuf>> {
    if !notes_dir.exists() {
        return Ok(None);
    }

    let today = today_date();
    let mut candidates: Vec<PathBuf> = Vec::new();

    for entry in std::fs::read_dir(notes_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            if let Ok(date) = stem.parse::<jiff::civil::Date>() {
                if date < today {
                    candidates.push(path);
                }
            }
        }
    }

    // Filenames are YYYY-MM-DD.md, so lexicographic == date order.
    candidates.sort();
    Ok(candidates.into_iter().last())
}
