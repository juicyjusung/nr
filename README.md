# nr <sub>(npm run, but better)</sub>

> Interactive npm script runner for your terminal.

[![CI](https://github.com/juicyjusung/nr/actions/workflows/ci.yml/badge.svg)](https://github.com/juicyjusung/nr/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/juicyjusung/nr)](https://github.com/juicyjusung/nr/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![demo](assets/demo.gif)

<details>
<summary>📹 What's happening in the demo?</summary>

The demo showcases:
- **Fuzzy search**: Typing `tst` finds "test", `bld` finds "build" - no exact names needed!
- **Navigation**: Use `↑` `↓` arrow keys to browse scripts
- **Favorites**: Press `Space` to star your most-used scripts
- **Monorepo support**: Press `→` to switch to Packages tab, `Enter` to view package scripts, `←` to go back
- **Quick exit**: Press `Esc` to quit anytime

</details>

## Why nr?

`npm run` requires you to remember exact script names. `nr` gives you a fuzzy-searchable, interactive TUI — just type a few letters and hit enter.

## Features

- **Fuzzy search** — Find scripts instantly, no need to remember exact names
- **Favorites & recents** — Starred scripts float to the top; frecency-based sorting learns your habits
- **Monorepo support** — Works with npm, yarn, pnpm, and bun workspaces out of the box
- **Auto-detection** — Picks the right package manager from your lockfile
- **Fast & lightweight** — Single ~1 MB binary, no runtime dependencies

## Installation

### Homebrew (macOS / Linux)

```bash
brew install juicyjusung/tap/nr
```

### Shell script (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/juicyjusung/nr/main/install.sh | sh
```

### Cargo

```bash
cargo install --git https://github.com/juicyjusung/nr
```

### Scoop (Windows)

```powershell
scoop bucket add juicyjusung https://github.com/juicyjusung/nr
scoop install nr
```

### GitHub Releases

Pre-built binaries for all platforms are available on the [Releases](https://github.com/juicyjusung/nr/releases/latest) page.

## Usage

Run `nr` in any directory with a `package.json`:

```bash
nr
```

That's it. Start typing to search, arrow keys to navigate, enter to run.

## Key Bindings

| Key | Action |
|-----|--------|
| `↑` `↓` | Navigate scripts |
| `Enter` | Run selected script |
| `Space` | Toggle favorite |
| `←` `→` | Switch tabs (Scripts / Packages) |
| `Esc` | Quit or go back |
| Type | Fuzzy search |

## Monorepo Support

`nr` auto-detects workspaces (npm, yarn, pnpm, bun). Use the **Packages** tab to browse workspace packages and their scripts.

## Building from Source

Requires Rust 1.85+.

```bash
cargo build --release
```

### Generating Demo GIF

To regenerate the demo GIF:

```bash
# Install VHS
brew install vhs

# Generate demo
./scripts/generate-demo.sh
```

Or manually:

```bash
cargo build --release
export PATH="$PWD/target/release:$PATH"
vhs demo.tape
```

See `assets/README.md` for more details.

## Contributing

Contributions are welcome! Feel free to open issues and pull requests.

## License

[MIT](LICENSE)
