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

pub fn run(path: &Path, theme: &crate::config::Theme) -> anyhow::Result<()> {
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

    let run_result = event_loop(&mut stdout, &mut items, &mut cursor_idx, theme);

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
    theme: &crate::config::Theme,
) -> anyhow::Result<()> {
    loop {
        render(stdout, items, *cursor, theme)?;

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

fn queue_highlighted<W: Write>(
    w: &mut W,
    text: &str,
    base: &crate::config::ElementStyle,
    theme: &crate::config::Theme,
) -> anyhow::Result<()> {
    use crate::highlight::Segment;
    for seg in crate::highlight::parse_segments(text) {
        match seg {
            Segment::Plain(s) => queue!(w, Print(s))?,
            Segment::Mention(s) => {
                apply_element_style(w, &theme.mention)?;
                queue!(w, Print(s))?;
                reset_to_element_style(w, base)?;
            }
            Segment::Tag(s) => {
                apply_element_style(w, &theme.tag)?;
                queue!(w, Print(s))?;
                reset_to_element_style(w, base)?;
            }
            Segment::Code(s) => {
                apply_element_style(w, &theme.code)?;
                queue!(w, Print(s))?;
                reset_to_element_style(w, base)?;
            }
        }
    }
    Ok(())
}

fn render(stdout: &mut impl Write, items: &[Item], cursor: usize, theme: &crate::config::Theme) -> anyhow::Result<()> {
    queue!(stdout, terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;

    let mut prev_section = String::new();
    for (i, item) in items.iter().enumerate() {
        if item.section != prev_section {
            if i > 0 {
                queue!(stdout, Print("\r\n"))?;
            }
            apply_element_style(stdout, &theme.heading)?;
            queue!(stdout, Print(format!("  {}\r\n", item.section)))?;
            queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
            prev_section.clone_from(&item.section);
        }

        let checkbox = match item.state {
            TodoState::Open       => "[ ]",
            TodoState::InProgress => "[/]",
            TodoState::Done       => "[x]",
        };

        let line_style = match item.state {
            TodoState::Open       => &theme.open_todo,
            TodoState::InProgress => &theme.in_progress,
            TodoState::Done       => &theme.done,
        };

        if i == cursor {
            queue!(stdout, SetAttribute(Attribute::Reverse), Print(format!("> {} ", checkbox)), SetAttribute(Attribute::Reset))?;
            queue_highlighted(stdout, &item.text, line_style, theme)?;
            queue!(stdout, Print("\r\n"))?;
        } else {
            apply_element_style(stdout, line_style)?;
            queue!(stdout, Print(format!("  {} ", checkbox)))?;
            queue_highlighted(stdout, &item.text, line_style, theme)?;
            queue!(stdout, ResetColor, SetAttribute(Attribute::Reset), Print("\r\n"))?;
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

fn to_ct_color(c: &crate::config::ThemeColor) -> crossterm::style::Color {
    use crate::config::{NamedColor, ThemeColor};
    use crossterm::style::Color;
    match c {
        ThemeColor::Rgb { r, g, b } => Color::Rgb { r: *r, g: *g, b: *b },
        ThemeColor::Named(n) => match n {
            NamedColor::Black       => Color::Black,
            NamedColor::Red         => Color::Red,
            NamedColor::Green       => Color::Green,
            NamedColor::Yellow      => Color::Yellow,
            NamedColor::Blue        => Color::Blue,
            NamedColor::Magenta     => Color::Magenta,
            NamedColor::Cyan        => Color::Cyan,
            NamedColor::White       => Color::White,
            NamedColor::DarkGrey    => Color::DarkGrey,
            NamedColor::DarkRed     => Color::DarkRed,
            NamedColor::DarkGreen   => Color::DarkGreen,
            NamedColor::DarkYellow  => Color::DarkYellow,
            NamedColor::DarkBlue    => Color::DarkBlue,
            NamedColor::DarkMagenta => Color::DarkMagenta,
            NamedColor::DarkCyan    => Color::DarkCyan,
        },
    }
}

fn to_ct_attr(s: crate::config::TextStyle) -> crossterm::style::Attribute {
    use crate::config::TextStyle;
    use crossterm::style::Attribute;
    match s {
        TextStyle::Bold      => Attribute::Bold,
        TextStyle::Underline => Attribute::Underlined,
        TextStyle::Italic    => Attribute::Italic,
        TextStyle::Dimmed    => Attribute::Dim,
    }
}

fn apply_element_style<W: std::io::Write>(
    w: &mut W,
    style: &crate::config::ElementStyle,
) -> anyhow::Result<()> {
    if let Some(c) = &style.color {
        queue!(w, SetForegroundColor(to_ct_color(c)))?;
    }
    for &s in &style.styles {
        queue!(w, SetAttribute(to_ct_attr(s)))?;
    }
    Ok(())
}

fn reset_to_element_style<W: std::io::Write>(
    w: &mut W,
    style: &crate::config::ElementStyle,
) -> anyhow::Result<()> {
    queue!(w, ResetColor, SetAttribute(Attribute::Reset))?;
    apply_element_style(w, style)
}
