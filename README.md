# codex-clean

A fast TUI tool for visualizing and cleaning up [Codex](https://openai.com/codex) worktrees.

Codex creates git worktrees in `~/.codex/worktrees/` for each thread. These accumulate over time and can consume significant disk space — especially for Rust projects with multi-GB `target/` directories. This tool lets you see what's there and reclaim space.

## Quick Start

```sh
cargo install codex-clean
codex-clean
```

That's it. The TUI launches and scans `~/.codex/worktrees/` automatically.

## Features

- **Visualize** all Codex worktrees with metadata: project name, git branch, size, build artifact size, linked thread name, and last update time
- **Clean build artifacts** for selected worktrees (Rust `target/`, Go `vendor/`, Node `node_modules/`, Python `.venv/` + `__pycache__/`)
- **Delete** entire worktrees (including git worktree metadata cleanup)
- **Sort** by size, name, artifact size, or last updated
- **Detail view** with full metadata for each worktree

## Screenshot

```
┌──────────────────────────────────── codex-clean ───────────────────────────────┐
│ 6 worktrees  |  Total: 3.3GB  |  Artifacts: 2.9GB  |  Sort: Size              │
├──┬──────┬───────────┬──────────────┬────────┬─────────┬─────────────┬──────────┤
│  │ ID   │ Project   │ Branch       │ Size   │Artifact │ Updated     │ Thread   │
│  │ cf52 │ my-app    │ feat/x       │ 2.2GB  │ 2.2GB   │ 2 days ago  │ Fix bug  │
│● │ e211 │ my-app    │ main         │ 697MB  │ 680MB   │ 1 week ago  │ Refactor │
│  │ 088a │ my-lib    │ (detached)   │ 11MB   │ 0B      │ 3 weeks ago │ (unknown)│
├──┴──────┴───────────┴──────────────┴────────┴─────────┴─────────────┴──────────┤
│ [↑↓] Navigate  [Space] Select  [c] Clean  [d] Delete  [q] Quit                │
└────────────────────────────────────────────────────────────────────────────────┘
```

## Installation

### From crates.io

```sh
cargo install codex-clean
```

### From source

```sh
git clone https://github.com/iamquang95/codex-clean.git
cd codex-clean
cargo install --path .
```

### Build manually

```sh
cargo build --release
# Binary at ./target/release/codex-clean (~800KB)
```

## Usage

```sh
codex-clean
```

The TUI launches and scans `~/.codex/worktrees/` automatically.

### Custom Codex home directory

By default, the tool looks for worktrees in `~/.codex/`. You can override this:

```sh
# Via CLI flag
codex-clean --codex-home /path/to/codex

# Via environment variable
export CODEX_HOME=/path/to/codex
codex-clean
```

Resolution order: `--codex-home` flag > `$CODEX_HOME` env var > `~/.codex`.

### Keybindings

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Space` | Toggle selection |
| `a` | Select / deselect all |
| `c` | Clean build artifacts for selected worktrees |
| `d` | Delete selected worktrees entirely |
| `Enter` | Show detail view |
| `s` | Cycle sort field (Size → Name → Artifacts → Updated) |
| `r` | Rescan worktrees |
| `q` / `Esc` | Quit |

### Cleaning artifacts

Pressing `c` removes build artifacts based on detected project type:

| Project Type | Detected By | Artifacts Removed |
|-------------|-------------|-------------------|
| Rust | `Cargo.toml` | `target/` |
| Go | `go.mod` | `vendor/` |
| Node.js | `package.json` | `node_modules/` |
| Python | `pyproject.toml`, `setup.py`, `requirements.txt` | `.venv/`, `__pycache__/` (recursive) |
| Unknown | (fallback) | `target/`, `node_modules/`, `.venv/`, `build/`, `dist/` |

A confirmation prompt is shown before any destructive action.

### Deleting worktrees

Pressing `d` fully removes selected worktrees:

1. Removes the git worktree metadata directory (e.g., `repo/.git/worktrees/name/`) to prevent stale refs in `git worktree list`
2. Removes the Codex worktree directory (`~/.codex/worktrees/{id}/`)

The Codex `session_index.jsonl` is not modified — Codex owns that file.

## How it works

Codex worktrees follow this structure:

```
~/.codex/worktrees/{hex-id}/{project-name}/
                                ├── .git     → gitdir: /path/to/repo/.git/worktrees/{name}
                                ├── src/
                                └── target/  → build artifacts
```

The tool resolves metadata by following the `.git` pointer:

1. `.git` file → parent repo's `.git/worktrees/{name}/`
2. `HEAD` → current branch
3. `codex-thread.json` → thread ID
4. `~/.codex/session_index.jsonl` → thread name and last update timestamp

Size computation uses a parallel filesystem walk via [rayon](https://docs.rs/rayon).

## Development

```sh
# Run in dev mode
cargo run

# Run tests
cargo test

# Build optimized release binary
cargo build --release
```

### Project structure

```
src/
  main.rs      — entry point, terminal setup/teardown
  model.rs     — data structures, size formatting, timestamp parsing
  scan.rs      — worktree discovery, metadata resolution, size computation
  app.rs       — app state machine, event loop, keybindings
  ui.rs        — TUI rendering (table, popups, help bar)
  cleanup.rs   — build artifact cleaning, worktree deletion
```

## Requirements

- Rust 1.70+
- macOS / Linux (uses `~/.codex/` directory)

## License

MIT
