# nr

TUI-based npm script runner with fuzzy search. Run scripts from your terminal interactively with lightning-fast fuzzy matching, favorites, and frecency-based suggestions.

## Features

- **Fuzzy search** - Instantly find scripts using fast nucleo-based fuzzy matching
- **Favorites** - Mark frequently-used scripts as favorites (persisted across sessions)
- **Recents** - Automatic frecency-based sorting with 14-day halflife decay
- **Monorepo support** - Full npm, yarn, pnpm, and bun workspace support with package-level script browsing
- **Package manager auto-detection** - Detects bun, pnpm, yarn, or npm from lockfiles
- **Dual-tab navigation** - Switch between root scripts and workspace packages
- **Terminal safety** - Graceful terminal restoration even on panic
- **Minimal binary** - ~1.1 MB release build with LTO optimizations

## Installation

### Via Cargo

```bash
cargo install --path .
```

### From Release Build

```bash
cargo build --release
cp target/release/nr /usr/local/bin/
```

### Requirements

- Rust 1.85+ (MSRV)
- A directory with `package.json`

## Usage

Simply run `nr` in a directory containing `package.json`:

```bash
nr
```

The TUI will open with an interactive list of available scripts. Use fuzzy search to narrow down results, navigate with arrow keys, and press Enter to execute.

```bash
nr --help       # Print help
nr --version    # Print version
```

## Key Bindings

| Key | Action |
|-----|--------|
| Up/Down (`↑` `↓`) | Navigate script list |
| Enter (`⏎`) | Execute selected script |
| Space (`␣`) | Toggle favorite for script |
| Left/Right (`←` `→`) | Switch tabs (Scripts/Packages) |
| Escape (`Esc`) | Quit (or go back from package scripts) |
| Ctrl+C | Force quit |
| Type | Search scripts (fuzzy filter) |
| Backspace | Delete search character |

## Configuration

### Storage Location

Configuration is stored in `~/.config/nr/`:

```
~/.config/nr/
├── favorites.json    # Favorites per project
└── recents.json      # Recent scripts with frecency scores
```

Projects are identified by SHA-256 hash of the monorepo root path, so separate workspaces maintain independent histories.

### Favorites

Toggle a script as favorite with Space while it's selected. Favorites are sorted first in the script list.

### Recents

Automatically tracks script execution history. Scripts are sorted by:
1. Favorites (if enabled)
2. Recency with frecency scoring (exponential decay over 14 days)
3. Fuzzy search match quality

## Monorepo Support

`nr` automatically detects and supports monorepos:

- **npm workspaces** - Reads `workspaces` field in root `package.json`
- **yarn workspaces** - Reads `workspaces` field in root `package.json`
- **pnpm workspaces** - Reads `pnpm-workspace.yaml`
- **bun workspaces** - Reads `workspaces` field in root `package.json`

When workspaces are detected, use the Packages tab (right arrow) to browse workspace packages and their scripts.

## Package Manager Detection

`nr` automatically detects your package manager based on lock files (checked in this order):

1. **bun** - `bun.lockb`
2. **pnpm** - `pnpm-lock.yaml`
3. **yarn** - `yarn.lock`
4. **npm** - `package-lock.json`

If no lock file is found, npm is used as fallback.

## Architecture

The project is organized into focused modules:

- **src/core/** - Project discovery, package manager detection, workspace scanning, script execution
- **src/store/** - Configuration and state persistence (favorites, recents, project IDs)
- **src/ui/** - TUI components (script list, package list, search, tabs, status bar)
- **src/fuzzy.rs** - Fuzzy matching wrapper around nucleo-matcher
- **src/sort.rs** - Frecency-based sorting algorithm
- **src/app.rs** - Central state machine and event loop
- **src/main.rs** - CLI entry point and lifecycle management

## Building from Source

### Prerequisites

- Rust 1.85 or later (install via [rustup](https://rustup.rs/))

### Build

```bash
# Debug build
cargo build

# Release build (optimized, ~1.1 MB)
cargo build --release
```

### Run Tests

```bash
cargo test
```

## Performance

The release build is optimized for minimal size and fast execution:

- LTO (Link-Time Optimization) enabled
- Strip symbols for minimal binary size
- Single codegen unit for better optimization
- Panic abort for reduced overhead

Binary size: ~1.1 MB (release build)

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.
