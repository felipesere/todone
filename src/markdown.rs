#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoState {
    Open,
    InProgress,
    Done,
}

impl TodoState {
    pub fn sigil(self) -> &'static str {
        match self {
            TodoState::Open => "- [ ] ",
            TodoState::InProgress => "- [/] ",
            TodoState::Done => "- [x] ",
        }
    }

    pub fn next(self) -> TodoState {
        match self {
            TodoState::Open => TodoState::InProgress,
            TodoState::InProgress => TodoState::Done,
            TodoState::Done => TodoState::Open,
        }
    }
}

pub struct Todo {
    pub text: String,
    pub state: TodoState,
    pub note_lines: Vec<String>,
    pub line_no: usize, // 0-indexed line in the source file
}

pub struct Section {
    pub name: String,
    pub todos: Vec<Todo>,
}

/// Parse a file's content into sections. Lines that are not section headings,
/// todos, or indented notes (freeform text, blank lines, the "# DATE" header)
/// are ignored — they are preserved in the file but not tracked by the parser.
pub fn parse_sections(content: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut current_section: Option<Section> = None;
    let mut current_todo: Option<Todo> = None;

    for (line_no, line) in content.lines().enumerate() {
        if let Some(name) = line.strip_prefix("## ") {
            flush_todo(&mut current_todo, &mut current_section);
            if let Some(sec) = current_section.take() {
                sections.push(sec);
            }
            current_section = Some(Section { name: name.to_string(), todos: Vec::new() });
        } else if let Some(text) = line.strip_prefix("- [ ] ") {
            flush_todo(&mut current_todo, &mut current_section);
            if current_section.is_some() {
                current_todo = Some(Todo { text: text.to_string(), state: TodoState::Open, note_lines: Vec::new(), line_no });
            }
        } else if let Some(text) = line.strip_prefix("- [/] ") {
            flush_todo(&mut current_todo, &mut current_section);
            if current_section.is_some() {
                current_todo = Some(Todo { text: text.to_string(), state: TodoState::InProgress, note_lines: Vec::new(), line_no });
            }
        } else if let Some(text) = line.strip_prefix("- [x] ").or_else(|| line.strip_prefix("- [X] ")) {
            flush_todo(&mut current_todo, &mut current_section);
            if current_section.is_some() {
                current_todo = Some(Todo { text: text.to_string(), state: TodoState::Done, note_lines: Vec::new(), line_no });
            }
        } else if line.starts_with("  ") || line.starts_with('\t') {
            if let Some(todo) = current_todo.as_mut() {
                todo.note_lines.push(line.to_string());
            }
        }
        // All other lines (blank, "# DATE", freeform) are left untouched in the file.
    }

    flush_todo(&mut current_todo, &mut current_section);
    if let Some(sec) = current_section {
        sections.push(sec);
    }

    sections
}

fn flush_todo(current_todo: &mut Option<Todo>, current_section: &mut Option<Section>) {
    if let (Some(todo), Some(sec)) = (current_todo.take(), current_section.as_mut()) {
        sec.todos.push(todo);
    }
}

/// Insert `- [ ] text` into the named project section of `content`,
/// preserving all other content exactly. If the section doesn't exist,
/// appends it at the end. Atomic-safe: returns new content as a String.
pub fn add_todo(content: &str, project: &str, text: &str) -> String {
    let has_trailing_newline = content.ends_with('\n');
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let target = format!("## {}", project);
    let new_todo = format!("- [ ] {}", text);

    if let Some(section_start) = lines.iter().position(|l| l == &target) {
        // Find the end of this section: next "## " heading or EOF.
        let section_end = lines[section_start + 1..]
            .iter()
            .position(|l| l.starts_with("## "))
            .map(|i| section_start + 1 + i)
            .unwrap_or(lines.len());

        // Insert after the last non-empty line inside the section.
        let insert_at = lines[section_start + 1..section_end]
            .iter()
            .rposition(|l| !l.trim().is_empty())
            .map(|i| section_start + 1 + i + 1)
            .unwrap_or(section_start + 1);

        lines.insert(insert_at, new_todo);
    } else {
        // Section not found — append at end.
        if lines.last().map(|l| !l.trim().is_empty()).unwrap_or(false) {
            lines.push(String::new());
        }
        lines.push(target);
        lines.push(new_todo);
    }

    let mut result = lines.join("\n");
    if has_trailing_newline {
        result.push('\n');
    }
    result
}

/// Set the state of the todo at the given (0-indexed) line number.
/// Replaces the sigil in-place. Line count is unchanged.
pub fn set_state(content: &str, line_no: usize, state: TodoState) -> String {
    let has_trailing = content.ends_with('\n');
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    if let Some(line) = lines.get_mut(line_no) {
        let text = line
            .strip_prefix("- [ ] ")
            .or_else(|| line.strip_prefix("- [/] "))
            .or_else(|| line.strip_prefix("- [x] "))
            .or_else(|| line.strip_prefix("- [X] "))
            .map(|t| t.to_string());
        if let Some(text) = text {
            *line = format!("{}{}", state.sigil(), text);
        }
    }
    let mut result = lines.join("\n");
    if has_trailing {
        result.push('\n');
    }
    result
}
