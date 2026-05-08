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

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Cmd {
    Today,
    Add {
        text: Option<String>,
        project: Option<String>,
    },
    Carry,
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

fn list_parser() -> impl Parser<Cmd> {
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
        list_parser(),
        done_parser()
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

fn highlight_text(text: &str) -> String {
    use highlight::Segment;
    let mut out = String::new();
    for seg in highlight::parse_segments(text) {
        match seg {
            Segment::Plain(s) => out.push_str(s),
            Segment::Mention(s) => out.push_str(&format!("{}", s.magenta())),
            Segment::Tag(s) => out.push_str(&format!("{}", s.cyan())),
            Segment::Code(s) => out.push_str(&format!("{}", s.bold())),
        }
    }
    out
}

fn print_todos(sections: &[markdown::Section], filter: Option<&str>) {
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
                highlight_text(&todo.text).dimmed().to_string()
            } else {
                highlight_text(&todo.text)
            };
            println!("  {} {text}", sigil.dimmed());
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
        Cmd::List { project, all } => {
            carryover::carry(&notes_dir)?;
            let path = daily::today_path(&notes_dir);
            let content = std::fs::read_to_string(&path)?;
            let sections = markdown::parse_sections(&content);
            let filter = if all { None } else { project.as_deref() };
            print_todos(&sections, filter);
        }
        Cmd::Done => {
            carryover::carry(&notes_dir)?;
            let path = daily::today_path(&notes_dir);
            tui::run(&path, &config.theme)?;
        }
    }

    Ok(())
}
