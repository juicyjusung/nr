# CLAUDE.md

## Project Overview

**nr** - TUI-based npm script runner with fuzzy search, favorites, frecency sorting, and monorepo workspace support. Written in Rust.

Binary name: `nr`

## Build & Test

```bash
cargo build              # Debug build
cargo build --release    # Release build (~1.1 MB, LTO + strip + panic=abort)
cargo test               # Run all tests
cargo test sort          # Run sort module tests only
cargo clippy             # Lint
cargo fmt -- --check     # Format check
```

MSRV: Rust 1.85 (edition 2024)

## Architecture

```
src/
├── main.rs          # CLI entry, lifecycle, panic hook for terminal restoration
├── app.rs           # Central state machine (App struct), event loop, input handling
├── fuzzy.rs         # Fuzzy matching wrapper (nucleo-matcher)
├── sort.rs          # Frecency-based sorting algorithm + tests
├── core/            # Business logic (stateless)
│   ├── package_manager.rs  # Lockfile-based PM detection (bun > pnpm > yarn > npm)
│   ├── project_root.rs     # Two-phase upward traversal for package.json / monorepo root
│   ├── scripts.rs          # Load scripts from package.json
│   ├── workspaces.rs       # Glob-based workspace package scanning
│   ├── runner.rs           # Execute scripts via detected package manager
│   ├── env_files.rs        # Scan and load .env files (NEW)
│   └── package_json.rs     # Shared package.json parser
├── store/           # Persistence layer (~/.config/nr/)
│   ├── favorites.rs        # HashSet<String> of starred script keys
│   ├── recents.rs          # Frecency tracking (14-day halflife, 100 entry cap)
│   ├── script_configs.rs   # Per-script env/args configurations (NEW)
│   ├── args_history.rs     # Global args history (max 20 entries) (NEW)
│   ├── global_env.rs       # Global env file preferences (NEW)
│   ├── project_id.rs       # SHA-256 hash of project root path
│   └── config_path.rs      # XDG config directory
└── ui/              # Pure rendering functions (no state)
    ├── script_list.rs       # Scrollable list with ❯ cursor and ★ favorites
    ├── package_list.rs      # Workspace package list
    ├── search_input.rs      # Search input with block cursor
    ├── status_bar.rs        # Keyboard shortcut hints
    ├── tabs.rs              # Scripts / Packages tab bar
    ├── env_selector.rs      # .env file selection modal (NEW)
    ├── args_input.rs        # Arguments input with cursor editing (NEW)
    └── execution_confirm.rs # Execution preview modal (NEW)
```

### Key Patterns

- **Index-based filtering**: `Vec<usize>` indices into data vectors, avoids cloning
- **Pure UI functions**: All `ui/` modules are stateless `render_*` functions taking `&Frame`
- **Stateless core**: `core/` modules are pure functions, no shared state
- **State machine**: `App` struct owns all mutable state, `handle_key()` returns `Action` enum
- **Modal state management**: `AppMode` enum (Normal, ConfigureEnv, ConfigureArgs, ConfirmExecution)
- **Two-phase discovery**: Find nearest `package.json`, then search upward for monorepo root
- **Scroll management**: Viewport offset tracking via `ensure_scroll()` helper
- **Cursor position tracking**: Character-level cursor for text input with Left/Right/Home/End support

### Data Flow

1. `main.rs`: discover project root -> detect package manager -> load scripts -> scan workspaces
2. Load persisted favorites/recents/configs from `~/.config/nr/{project_id}/` keyed by SHA-256 project ID
3. Enter TUI event loop (`App::handle_key` -> `Action`)
4. On `Action::RunScript`: exit TUI, save state, exec script via `process::exit()`
5. Configuration flow (Tab key):
   - Scan .env files from package + root directories
   - Restore previous env/args from `script_configs.json`
   - User selects env files -> inputs args -> confirms
   - Save configuration per script key
   - Execute with injected env vars and additional arguments

### Sorting Algorithm (sort.rs)

- **No query**: Favorites (alphabetical) -> Frecency score -> Alphabetical
- **With query**: Fuzzy relevance -> Favorites break ties -> Frecency breaks ties
- Frecency formula: `count * 0.5^(age_days / 14)`

### Script Key Format

`{project_id}:{scope}:{name}` where scope is `root` or package name.

### Configuration Storage

```
~/.config/nr/{project_id}/
├── favorites.json         # Starred scripts
├── recents.json          # Frecency-tracked execution history
├── script_configs.json   # Per-script env/args configurations
├── args_history.json     # Global args history (max 20)
└── global_env.json       # Global env file preferences
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| ratatui + crossterm | TUI framework |
| nucleo-matcher | Fuzzy matching |
| serde + serde_json | JSON persistence |
| serde_yaml_ng | pnpm-workspace.yaml parsing |
| indexmap | Ordered script maps (preserve package.json order) |
| globset | Workspace glob patterns |
| sha2 | Project ID hashing |
| dirs | XDG config directory |
| anyhow + thiserror | Error handling |
| tempfile (dev) | Test fixtures |

## Conventions

- Error handling: `anyhow::Result` for app-level, `thiserror` for domain errors
- No `unwrap()` in production code paths; use `?` or explicit error handling
- Tests live in `#[cfg(test)] mod tests` within the same file
- UI functions take `(frame, area, &data, ...)` - never hold references to App
- Config storage: `~/.config/nr/` via `dirs::config_dir()`
