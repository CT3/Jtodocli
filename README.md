# jtodo

A simple terminal todo app written in Rust. Uses a markdown file (`todo.md`) as storage with standard checkbox syntax, so your tasks are readable anywhere.

Includes both a CLI for quick actions and a TUI (built with ratatui) for interactive use.

## Install

```sh
cargo install jtodo
```

Or build from source:

```sh
cargo build --release
# binary at target/release/jtodo
```

Or with just:

```sh
just release
```

## Usage

### CLI

```sh
jtodo add Buy groceries
jtodo add Fix the leaky faucet
jtodo list
jtodo done 1
jtodo delete 2
```

### TUI

Run `jtodo` with no arguments to launch the interactive interface.

| Key             | Action            |
|-----------------|-------------------|
| `j` / `k`      | Move up/down      |
| `Enter` / `Space` | Toggle done    |
| `d`            | Delete task        |
| `a`            | Add new task       |
| `q` / `Esc`   | Quit               |

### Setup

By default, jtodo reads from `todo.md` in the current directory. To point it at a different file:

```sh
jtodo setup ~/notes/todo.md
```

Config is stored at `~/.config/jtodo/config.toml`.

## File format

```md
- [ ] Pending task
- [x] Completed task
```
