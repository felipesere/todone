mod carryover;
mod config;
mod daily;
mod highlight;
mod input;
mod markdown;
mod tui;

use bpaf::*;
use owo_colors::OwoColorize;
use std::path::Path;

#[derive(Debug, Clone)]
enum Cmd {
    Today,
    Add {
        text: Option<String>,
        project: Option<String>,
    },
    Carry,
    Edit,
    List {
        project: Option<String>,
        all: bool,
    },
    Done,
}

fn today_parser() -> impl Parser<Cmd> {
    pure(Cmd::Today)
        .to_options()
        .descr("Print path to today's file (creates it with carryover if needed)")
        .command("today")
}

fn add_parser() -> impl Parser<Cmd> {
    let text = positional::<String>("TEXT")
        .help("Todo text (omit for interactive prompt)")
        .optional();
    let project = long("project")
        .short('p')
        .env("TODONE_PROJECT")
        .help("Project section (default: inbox, or interactive if omitted)")
        .argument::<String>("PROJECT")
        .optional();
    construct!(Cmd::Add { project, text })
        .to_options()
        .descr("Add a todo to today's file")
        .command("add")
}

fn carry_parser() -> impl Parser<Cmd> {
    pure(Cmd::Carry)
        .to_options()
        .descr("Create today's file, carrying over open todos from the previous day (idempotent)")
        .command("carry")
}

fn edit_parser() -> impl Parser<Cmd> {
    pure(Cmd::Edit)
        .to_options()
        .descr("Open today's file in $EDITOR (creates it with carryover if needed)")
        .command("edit")
}

fn list_inner() -> impl Parser<Cmd> {
    let project = long("project")
        .short('p')
        .env("TODONE_PROJECT")
        .help("Filter by project section")
        .argument::<String>("PROJECT")
        .optional();
    let all = long("all")
        .short('a')
        .help("Show all projects, ignoring any --project / TODONE_PROJECT filter")
        .switch();
    construct!(Cmd::List { project, all })
}

fn list_parser() -> impl Parser<Cmd> {
    list_inner()
        .to_options()
        .descr("List open todos")
        .command("list")
}

fn done_parser() -> impl Parser<Cmd> {
    pure(Cmd::Done)
        .to_options()
        .descr("Interactively mark todos as done (TUI)")
        .command("done")
}

fn parse_opts() -> OptionParser<Cmd> {
    construct!([
        today_parser(),
        add_parser(),
        carry_parser(),
        edit_parser(),
        list_parser(),
        done_parser(),
        list_inner()
    ])
    .to_options()
    .descr("todone — personal daily TODO tracker")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write content to path via a temp file + rename for atomic replacement.
pub(crate) fn atomic_write(path: &Path, content: &str) -> anyhow::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn to_owo_style(style: &config::ElementStyle) -> owo_colors::Style {
    use config::{NamedColor, TextStyle, ThemeColor};

    let mut owo = owo_colors::Style::new();
    if let Some(color) = &style.color {
        owo = match color {
            ThemeColor::Rgb { r, g, b } => owo.color(owo_colors::Rgb(*r, *g, *b)),
            ThemeColor::Named(n) => owo.color(match n {
                NamedColor::Black => owo_colors::AnsiColors::Black,
                NamedColor::Red => owo_colors::AnsiColors::Red,
                NamedColor::Green => owo_colors::AnsiColors::Green,
                NamedColor::Yellow => owo_colors::AnsiColors::Yellow,
                NamedColor::Blue => owo_colors::AnsiColors::Blue,
                NamedColor::Magenta => owo_colors::AnsiColors::Magenta,
                NamedColor::Cyan => owo_colors::AnsiColors::Cyan,
                NamedColor::White => owo_colors::AnsiColors::White,
                NamedColor::DarkGrey => owo_colors::AnsiColors::BrightBlack,
                NamedColor::DarkRed => owo_colors::AnsiColors::BrightRed,
                NamedColor::DarkGreen => owo_colors::AnsiColors::BrightGreen,
                NamedColor::DarkYellow => owo_colors::AnsiColors::BrightYellow,
                NamedColor::DarkBlue => owo_colors::AnsiColors::BrightBlue,
                NamedColor::DarkMagenta => owo_colors::AnsiColors::BrightMagenta,
                NamedColor::DarkCyan => owo_colors::AnsiColors::BrightCyan,
            }),
        };
    }
    for s in &style.styles {
        owo = match s {
            TextStyle::Bold => owo.bold(),
            TextStyle::Underline => owo.underline(),
            TextStyle::Italic => owo.italic(),
            TextStyle::Dimmed => owo.dimmed(),
        };
    }
    owo
}

fn highlight_text(text: &str, theme: &config::Theme) -> String {
    use highlight::Segment;
    let mut out = String::new();
    for seg in highlight::parse_segments(text) {
        match seg {
            Segment::Plain(s) => out.push_str(s),
            Segment::Mention(s) => out.push_str(&format!("{}", to_owo_style(&theme.mention).style(s))),
            Segment::Tag(s) => out.push_str(&format!("{}", to_owo_style(&theme.tag).style(s))),
            Segment::Code(s) => out.push_str(&format!("{}", to_owo_style(&theme.code).style(s))),
            Segment::Link { text, url } => {
                out.push_str(&format!(
                    "\x1b]8;;{url}\x1b\\{}\x1b]8;;\x1b\\",
                    to_owo_style(&theme.link).style(text)
                ));
            }
        }
    }
    out
}

fn print_todos(sections: &[markdown::Section], filter: Option<&str>, theme: &config::Theme) {
    let mut printed_any = false;
    for section in sections {
        if let Some(f) = filter {
            if section.name != f {
                continue;
            }
        }
        if section.todos.is_empty() {
            continue;
        }
        println!("{}", section.name.bold());
        for todo in &section.todos {
            let sigil = match todo.state {
                markdown::TodoState::InProgress => "[/]",
                markdown::TodoState::Done => "[x]",
                _ => "[ ]",
            };
            let text = if todo.state == markdown::TodoState::Done {
                highlight_text(&todo.text, theme).dimmed().to_string()
            } else {
                highlight_text(&todo.text, theme)
            };
            let marker = if todo.priority > 0 {
                format!("{} ", to_owo_style(&theme.priority).style("!".repeat(todo.priority as usize)))
            } else {
                String::new()
            };
            println!("  {} {marker}{text}", sigil.dimmed());
            for note in &todo.note_lines {
                println!("      {}", note.trim_start().dimmed());
            }
        }
        println!();
        printed_any = true;
    }
    if !printed_any {
        println!("{}", "No open todos.".dimmed());
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    let cmd = parse_opts().run();
    let config = config::load_config()?;
    let notes_dir = config.notes_dir;

    match cmd {
        Cmd::Today => {
            carryover::carry(&notes_dir)?;
            println!("{}", daily::today_path(&notes_dir).display());
        }
        Cmd::Add { text, project } => {
            carryover::carry(&notes_dir)?;
            let path = daily::today_path(&notes_dir);
            let content = std::fs::read_to_string(&path)?;
            let sections = markdown::parse_sections(&content);
            let project_names: Vec<&str> = sections.iter().map(|s| s.name.as_str()).collect();

            let (text, project) = match text {
                Some(t) => {
                    let p = match project {
                        Some(p) => p,
                        None => input::prompt_project(&project_names)?,
                    };
                    (t, p)
                }
                None => input::prompt_todo(&project_names)?,
            };

            let new_content = markdown::add_todo(&content, &project, &text);
            atomic_write(&path, &new_content)?;
        }
        Cmd::Carry => {
            let created = carryover::carry(&notes_dir)?;
            let path = daily::today_path(&notes_dir);
            if created {
                eprintln!("Created {}", path.display());
            } else {
                eprintln!("Already exists: {}", path.display());
            }
        }
        Cmd::Edit => {
            carryover::carry(&notes_dir)?;
            let path = daily::today_path(&notes_dir);
            let editor = std::env::var("EDITOR")
                .map_err(|_| anyhow::anyhow!("$EDITOR is not set"))?;

            let scratch = path.with_extension("edit.tmp");
            std::fs::copy(&path, &scratch)?;

            let status = std::process::Command::new(editor).arg(&scratch).status();

            let status = match status {
                Ok(s) => s,
                Err(e) => {
                    let _ = std::fs::remove_file(&scratch);
                    return Err(e.into());
                }
            };
            if !status.success() {
                let _ = std::fs::remove_file(&scratch);
                anyhow::bail!("editor exited with {status}, original file left untouched");
            }

            let new_content = std::fs::read_to_string(&scratch)?;
            std::fs::remove_file(&scratch)?;
            atomic_write(&path, &new_content)?;
        }
        Cmd::List { project, all } => {
            carryover::carry(&notes_dir)?;
            let path = daily::today_path(&notes_dir);
            let content = std::fs::read_to_string(&path)?;
            let sections = markdown::parse_sections(&content);
            let filter = if all { None } else { project.as_deref() };
            print_todos(&sections, filter, &config.theme);
        }
        Cmd::Done => {
            carryover::carry(&notes_dir)?;
            let path = daily::today_path(&notes_dir);
            tui::run(&path, &config.theme)?;
        }
    }

    Ok(())
}
