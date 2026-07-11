# nr

> Find and run `package.json` scripts without memorizing their names.

`nr` is a keyboard-first terminal UI for npm, pnpm, Yarn, and Bun.

[![CI](https://github.com/juicyjusung/nr/actions/workflows/ci.yml/badge.svg)](https://github.com/juicyjusung/nr/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/juicyjusung/nr)](https://github.com/juicyjusung/nr/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![nr demo showing fuzzy script search, favorites, and monorepo package navigation](assets/demo.gif)

## Quick start

Choose an installation method for your platform.

### Homebrew — macOS and glibc Linux

```bash
brew install juicyjusung/tap/nr
```

### Scoop — Windows

Custom Scoop buckets require Git; run `scoop install git` first if Git is not already available.

```powershell
scoop bucket add juicyjusung https://github.com/juicyjusung/nr.git
scoop install juicyjusung/nr
```

### Shell installer — macOS and glibc Linux

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/juicyjusung/nr/main/install.sh | sh
```

The installer downloads the latest release for the detected OS and architecture, verifies its SHA-256 checksum, and installs `nr` to `~/.local/bin` by default.

Then run `nr` anywhere inside a project that has a `package.json` in the current directory or one of its parents:

```bash
cd path/to/project
nr
```

Start typing to filter project-wide scripts by script name, package, workspace path, or command. Press `Enter` to run the selected script, or `Tab` to choose environment files and add arguments first.

## More installation options

### Build and install with Cargo

Source-based installation requires Rust 1.86 or newer:

```bash
cargo install --locked --git https://github.com/juicyjusung/nr.git nr
```

The Git URL is intentional: the `nr` package name on crates.io belongs to a different project.

### Install to a custom directory

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/juicyjusung/nr/main/install.sh | \
  sh -s -- --install-dir /usr/local/bin
```

### Prebuilt binaries

[GitHub Releases](https://github.com/juicyjusung/nr/releases/latest) provides archives and SHA-256 checksums for:

| Operating system | Architectures |
| --- | --- |
| macOS | Intel (`x86_64`), Apple silicon (`aarch64`) |
| Linux with glibc | `x86_64`, `aarch64` |
| Windows | `x86_64`, `aarch64` |

Alpine/musl and 32-bit binaries are not currently published.

## What `nr` does

| Capability | Behavior |
| --- | --- |
| Find | Fuzzy-matches script names first, then package names, workspace paths, and command text across the whole project. |
| Prioritize | Ranks by fuzzy relevance while searching; with no query, puts favorites first and orders the rest by frecency. |
| Configure | Selects `.env*` files, accepts additional arguments, and previews the run before execution. |
| Browse workspaces | Shows root and workspace scripts together in Scripts while retaining the Packages tab for package-first navigation. |
| Detect and run | Chooses npm, pnpm, Yarn, or Bun from project metadata and invokes it as `<manager> run <script>`. |

`nr` is distributed as a single executable; you do not install a global Node package to launch it. The package-manager CLI used by your project must still be available in `PATH` so `nr` can run its scripts.

## Running scripts

There are two execution paths:

| Goal | Key | Result |
| --- | --- | --- |
| Run now | `Enter` on a script | Runs immediately with the detected package manager, without applying saved environment or argument settings. |
| Configure first | `Tab` on a script | Opens environment selection, argument input, and a final command preview. |

The configuration flow:

1. Finds `.env*` files in the selected package and, when applicable, the monorepo root.
2. Lets you select files and enter space-separated script arguments.
3. Shows the command, environment filenames, and working directory before execution.

Selected root environment files are loaded before package-local files, so package-local values take precedence. `nr` remembers arguments per script, argument history per project, and the most recently selected environment filenames for the project. Environment values are read only when the script runs; they are not copied into `nr`'s stored state.

Arguments are split on whitespace and passed directly to the package manager. Shell-style quoting and expansion are not interpreted.

## Key bindings

### Script and package lists

| Key | Action |
| --- | --- |
| Type / `Backspace` | Update the fuzzy-search query. |
| `↑` / `↓` | Move the selection; navigation wraps at either end. |
| `Enter` | Run a script, or open the selected package from the Packages tab. |
| `Tab` | Configure the selected script before running it. |
| `Space` | Toggle the selected script as a favorite. |
| `←` / `→` | Switch between Scripts and Packages when workspaces are available. |
| `Esc` | Go back, cancel configuration, or quit at the top level. |
| `Ctrl+C` | Quit from any screen. |

<details>
<summary>Configuration flow keys</summary>

| Step | Keys |
| --- | --- |
| Environment files | `↑` / `↓` to navigate, `Space` to select, `Enter` to continue, `Esc` to cancel. |
| Arguments | Type to edit; use `←` / `→`, `Home`, `End`, `Backspace`, and `Delete`; use `↑` / `↓` for history; press `Enter` to continue or `Esc` to go back. |
| Confirmation | `Enter` to execute or `Esc` to return to the argument editor. |

</details>

## Project and workspace discovery

Starting from the current directory, `nr` walks upward to find the nearest `package.json`. It then continues upward to find a monorepo root declared by either:

- a `workspaces` array or `workspaces.packages` array in `package.json`; or
- a `packages` list in `pnpm-workspace.yaml`.

The Scripts tab combines runnable scripts from the monorepo root, the launch package, and every declared workspace package. Each row identifies its root or package scope, and the selected script always runs in the directory that declares it. The Packages tab remains available for package-first navigation and resolves to the same tasks.

A monorepo root does not need scripts of its own. `nr` opens whenever any discovered package has a string-valued script and reports a project-wide no-scripts error only when the root and all declared workspaces are empty. Overlapping workspace patterns are deduplicated, and negative pnpm workspace patterns are respected.

## Package-manager detection

Detection is lockfile-first: Bun, pnpm, Yarn, then npm. If no supported lockfile exists, `nr` reads the `packageManager` field from `package.json`; if that is absent or unknown, it falls back to npm.

## Local state

Favorites, recent runs, saved arguments, argument history, and selected environment filenames are stored locally and isolated per project. `nr` uses `$XDG_CONFIG_HOME/nr` when set, otherwise the platform-native configuration directory, with project data under `projects/<project-id>/`.

Useful project-scoped reset commands:

```bash
nr --reset-favorites
nr --reset-recents
```

Run `nr --help` for built-in usage help.

## Building from source

```bash
git clone https://github.com/juicyjusung/nr.git
cd nr
cargo build --release --locked
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow and the exact CI-compatible checks to run before opening a pull request.

## Support

- See [CHANGELOG.md](CHANGELOG.md) for notable changes.
- Report bugs or request features in [GitHub Issues](https://github.com/juicyjusung/nr/issues).

## License

[MIT](LICENSE)
