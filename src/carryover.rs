use std::path::Path;
use anyhow::Result;

/// Create today's file with open todos carried over from the most recent
/// previous file. Returns `true` if the file was created, `false` if it
/// already existed (idempotent).
pub fn carry(notes_dir: &Path) -> Result<bool> {
    let today_path = crate::daily::today_path(notes_dir);

    if today_path.exists() {
        return Ok(false);
    }

    std::fs::create_dir_all(notes_dir)?;

    let today_str = crate::daily::today_date().to_string();

    let content = match crate::daily::prev_path(notes_dir)? {
        Some(prev) => {
            let prev_content = std::fs::read_to_string(&prev)?;
            let sections = crate::markdown::parse_sections(&prev_content);
            build_content(&today_str, &sections)
        }
        None => format!("# {}\n\n## inbox\n", today_str),
    };

    std::fs::write(&today_path, content)?;
    Ok(true)
}

fn build_content(date: &str, sections: &[crate::markdown::Section]) -> String {
    let mut out = format!("# {}\n", date);
    let mut carried_any = false;

    for section in sections {
        let carry: Vec<_> = section.todos.iter().filter(|t| t.state != crate::markdown::TodoState::Done).collect();
        if carry.is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str(&format!("## {}\n", section.name));
        for todo in carry {
            out.push_str(todo.state.sigil());
            out.push_str(&todo.text);
            out.push('\n');
            for note in &todo.note_lines {
                out.push_str(note);
                out.push('\n');
            }
        }
        carried_any = true;
    }

    if !carried_any {
        // Fresh day with no open todos — start with a blank inbox.
        out.push_str("\n## inbox\n");
    }

    out
}
