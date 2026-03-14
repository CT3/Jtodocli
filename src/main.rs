use std::fs::{self, File};
use std::io::{self, BufWriter, Write, stdout};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
struct Config {
    todo_file: Option<String>,
}

fn config_path() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("jtodo");
    fs::create_dir_all(&dir).ok();
    dir.join("config.toml")
}

fn load_config() -> Config {
    let path = config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config(config: &Config) {
    let path = config_path();
    let content = toml::to_string_pretty(config).expect("Failed to serialize config");
    fs::write(path, content).expect("Failed to write config");
}

fn todo_path() -> PathBuf {
    let config = load_config();
    match config.todo_file {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from("todo.md"),
    }
}

#[derive(Parser)]
#[command(name = "jtodo", about = "A simple todo CLI/TUI using a markdown file")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new task
    Add {
        /// The task description
        task: Vec<String>,
    },
    /// Mark a task as done by line number
    Done {
        /// Line number (1-based)
        line: usize,
    },
    /// Delete a task by line number
    Delete {
        /// Line number (1-based)
        line: usize,
    },
    /// List all tasks
    List,
    /// Set the path to the todo.md file
    Setup {
        /// Path to the todo file
        path: String,
    },
}

#[derive(Clone)]
struct Todo {
    text: String,
    done: bool,
}

impl Todo {
    fn to_line(&self) -> String {
        if self.done {
            format!("- [x] {}", self.text)
        } else {
            format!("- [ ] {}", self.text)
        }
    }

    fn write_line(&self, w: &mut impl Write) -> io::Result<()> {
        let marker = if self.done { 'x' } else { ' ' };
        write!(w, "- [{}] {}\n", marker, self.text)
    }
}

fn load_todos() -> Vec<Todo> {
    let path = todo_path();
    let content = fs::read_to_string(&path).unwrap_or_default();
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("- [x] ") {
                Some(Todo {
                    text: trimmed.strip_prefix("- [x] ").unwrap().to_string(),
                    done: true,
                })
            } else if trimmed.starts_with("- [ ] ") {
                Some(Todo {
                    text: trimmed.strip_prefix("- [ ] ").unwrap().to_string(),
                    done: false,
                })
            } else {
                None
            }
        })
        .collect()
}

fn save_todos(todos: &[Todo]) {
    let path = todo_path();
    let file = File::create(path).expect("Failed to create todo file");
    let mut writer = BufWriter::new(file);
    for todo in todos {
        todo.write_line(&mut writer)
            .expect("Failed to write todo line");
    }
    writer.flush().expect("Failed to flush todo file");
}

fn print_todos(todos: &[Todo]) {
    if todos.is_empty() {
        println!("No tasks yet. Add one with: jtodo add <task>");
        return;
    }
    for (i, todo) in todos.iter().enumerate() {
        let marker = if todo.done { "x" } else { " " };
        println!("  {}. [{}] {}", i + 1, marker, todo.text);
    }
}

fn run_tui(mut todos: Vec<Todo>) -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut list_state = ListState::default();
    if !todos.is_empty() {
        list_state.select(Some(0));
    }

    let mut input_mode = false;
    let mut input_buf = String::new();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(f.area());

            // Task list
            let items: Vec<ListItem> = todos
                .iter()
                .enumerate()
                .map(|(i, todo)| {
                    let marker = if todo.done { "x" } else { " " };
                    let style = if todo.done {
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::CROSSED_OUT)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ListItem::new(format!("{}. [{}] {}", i + 1, marker, todo.text)).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .title(" Tasks ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            f.render_stateful_widget(list, chunks[0], &mut list_state);

            // Input area
            let input_block = Block::default()
                .title(if input_mode {
                    " New task (Enter to add, Esc to cancel) "
                } else {
                    " Press 'a' to add "
                })
                .borders(Borders::ALL)
                .border_style(if input_mode {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Cyan)
                });

            let input = Paragraph::new(input_buf.as_str()).block(input_block);
            f.render_widget(input, chunks[1]);

            // Help bar
            let help = Paragraph::new(" j/k:move i/o:reorder Enter:toggle d:del a:add q:quit ")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(help, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if input_mode {
                match key.code {
                    KeyCode::Enter => {
                        let text = input_buf.trim().replace(['\n', '\r'], " ");
                        if !text.is_empty() {
                            todos.push(Todo { text, done: false });
                            save_todos(&todos);
                            if list_state.selected().is_none() {
                                list_state.select(Some(0));
                            }
                        }
                        input_buf.clear();
                        input_mode = false;
                    }
                    KeyCode::Esc => {
                        input_buf.clear();
                        input_mode = false;
                    }
                    KeyCode::Backspace => {
                        input_buf.pop();
                    }
                    KeyCode::Char(c) => {
                        input_buf.push(c);
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('j') | KeyCode::Down => {
                        if !todos.is_empty() {
                            let i = list_state.selected().unwrap_or(0);
                            let next = if i >= todos.len() - 1 { 0 } else { i + 1 };
                            list_state.select(Some(next));
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if !todos.is_empty() {
                            let i = list_state.selected().unwrap_or(0);
                            let next = if i == 0 { todos.len() - 1 } else { i - 1 };
                            list_state.select(Some(next));
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(i) = list_state.selected() {
                            if i < todos.len() {
                                todos[i].done = !todos[i].done;
                                save_todos(&todos);
                            }
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(i) = list_state.selected() {
                            if i < todos.len() {
                                todos.remove(i);
                                save_todos(&todos);
                                if todos.is_empty() {
                                    list_state.select(None);
                                } else if i >= todos.len() {
                                    list_state.select(Some(todos.len() - 1));
                                }
                            }
                        }
                    }
                    KeyCode::Char('i') => {
                        if let Some(i) = list_state.selected() {
                            if i > 0 {
                                todos.swap(i, i - 1);
                                save_todos(&todos);
                                list_state.select(Some(i - 1));
                            }
                        }
                    }
                    KeyCode::Char('o') => {
                        if let Some(i) = list_state.selected() {
                            if i + 1 < todos.len() {
                                todos.swap(i, i + 1);
                                save_todos(&todos);
                                list_state.select(Some(i + 1));
                            }
                        }
                    }
                    KeyCode::Char('a') => {
                        input_mode = true;
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Add { task }) => {
            let text = task.join(" ").replace(['\n', '\r'], " ");
            if text.is_empty() {
                eprintln!("Task description cannot be empty.");
                std::process::exit(1);
            }
            let mut todos = load_todos();
            todos.push(Todo {
                text: text.clone(),
                done: false,
            });
            save_todos(&todos);
            println!("Added: {}", text);
        }
        Some(Commands::Done { line }) => {
            let mut todos = load_todos();
            if line == 0 || line > todos.len() {
                eprintln!("Invalid line number. Use 'jtodo list' to see tasks.");
                std::process::exit(1);
            }
            todos[line - 1].done = true;
            save_todos(&todos);
            println!("Done: {}", todos[line - 1].text);
        }
        Some(Commands::Delete { line }) => {
            let mut todos = load_todos();
            if line == 0 || line > todos.len() {
                eprintln!("Invalid line number. Use 'jtodo list' to see tasks.");
                std::process::exit(1);
            }
            let removed = todos.remove(line - 1);
            save_todos(&todos);
            println!("Deleted: {}", removed.text);
        }
        Some(Commands::List) => {
            let todos = load_todos();
            print_todos(&todos);
        }
        Some(Commands::Setup { path }) => {
            let resolved = fs::canonicalize(&path).unwrap_or_else(|_| PathBuf::from(&path));
            let mut config = load_config();
            config.todo_file = Some(resolved.to_string_lossy().to_string());
            save_config(&config);
            println!("Todo file set to: {}", resolved.display());
        }
        None => {
            // No subcommand — launch TUI
            let todos = load_todos();
            if let Err(e) = run_tui(todos) {
                eprintln!("TUI error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
