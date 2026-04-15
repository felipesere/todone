use std::io::{self, Write};
use std::path::Path;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal,
};

use crate::markdown::{self, TodoState};

struct Item {
    section: String,
    text: String,
    state: TodoState,
    line_no: usize,
    original_state: TodoState,
}

pub fn run(path: &Path) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(path)?;
    let sections = markdown::parse_sections(&content);

    let mut items: Vec<Item> = sections
        .iter()
        .flat_map(|s| {
            s.todos.iter().map(move |t| Item {
                section: s.name.clone(),
                text: t.text.clone(),
                state: t.state,
                line_no: t.line_no,
                original_state: t.state,
            })
        })
        .collect();

    if items.is_empty() {
        println!("No todos.");
        return Ok(());
    }

    let mut cursor_idx = 0usize;
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let run_result = event_loop(&mut stdout, &mut items, &mut cursor_idx);

    execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
    terminal::disable_raw_mode()?;
    run_result?;

    // Persist any changed items (set_state does in-place line replacement,
    // line count stays the same so line_no references remain valid).
    let mut content = std::fs::read_to_string(path)?;
    for item in &items {
        if item.state != item.original_state {
            content = markdown::set_state(&content, item.line_no, item.state);
        }
    }
    crate::atomic_write(path, &content)?;

    Ok(())
}

fn event_loop(
    stdout: &mut impl Write,
    items: &mut [Item],
    cursor: &mut usize,
) -> anyhow::Result<()> {
    loop {
        render(stdout, items, *cursor)?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Char('j') | KeyCode::Down => {
                    if *cursor + 1 < items.len() {
                        *cursor += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if *cursor > 0 {
                        *cursor -= 1;
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    items[*cursor].state = items[*cursor].state.next();
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn render(stdout: &mut impl Write, items: &[Item], cursor: usize) -> anyhow::Result<()> {
    queue!(stdout, terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;

    let mut prev_section = String::new();
    for (i, item) in items.iter().enumerate() {
        if item.section != prev_section {
            if i > 0 {
                queue!(stdout, Print("\r\n"))?;
            }
            queue!(
                stdout,
                SetAttribute(Attribute::Bold),
                Print(format!("  {}\r\n", item.section)),
                SetAttribute(Attribute::Reset),
            )?;
            prev_section.clone_from(&item.section);
        }

        let checkbox = match item.state {
            TodoState::Open => "[ ]",
            TodoState::InProgress => "[/]",
            TodoState::Done => "[x]",
        };

        if i == cursor {
            queue!(
                stdout,
                SetAttribute(Attribute::Reverse),
                Print(format!("> {} {}\r\n", checkbox, item.text)),
                SetAttribute(Attribute::Reset),
            )?;
        } else if item.state == TodoState::Done {
            queue!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("  {} {}\r\n", checkbox, item.text)),
                ResetColor,
            )?;
        } else if item.state == TodoState::InProgress {
            queue!(
                stdout,
                SetForegroundColor(Color::Yellow),
                Print(format!("  {} {}\r\n", checkbox, item.text)),
                ResetColor,
            )?;
        } else {
            queue!(stdout, Print(format!("  {} {}\r\n", checkbox, item.text)))?;
        }
    }

    let (_, rows) = terminal::size()?;
    queue!(
        stdout,
        cursor::MoveTo(0, rows - 1),
        SetForegroundColor(Color::DarkGrey),
        Print("j/k: move  space: cycle state  q: save & quit"),
        ResetColor,
    )?;

    stdout.flush()?;
    Ok(())
}
