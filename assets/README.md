# Demo Scripts for nr

This directory contains demo materials for the README.

## Generating the Demo GIF

### Prerequisites

Install VHS:
```bash
brew install vhs
```

Or see [VHS installation instructions](https://github.com/charmbracelet/vhs#installation).

### Generate Demo

From the project root:

```bash
# Build nr first
cargo build --release

# Make sure nr is in your PATH or use the built binary
export PATH="$PWD/target/release:$PATH"

# Generate the demo
vhs demo.tape
```

This will create `assets/demo.gif`.

### Customizing the Demo

Edit `demo.tape` to customize:
- **Theme**: Change `Set Theme` to any theme from [here](https://github.com/charmbracelet/vhs#themes)
- **Size**: Adjust `Set Width` and `Set Height`
- **Speed**: Modify `Sleep` durations
- **Content**: Change the `Type` commands to show different features

### Tips for Great Demos

1. **Keep it short**: 10-30 seconds is ideal
2. **Show key features**: Fuzzy search, navigation, favorites
3. **Use realistic examples**: Show real npm scripts
4. **Optimize file size**: Keep under 5MB for fast loading
5. **Test on GitHub**: Preview how it looks in dark/light themes

## Alternative: Manual Recording

If you prefer manual recording:

1. **Use Kap** (macOS): `brew install --cask kap`
2. **Use asciinema + agg**:
   ```bash
   brew install asciinema agg
   asciinema rec demo.cast
   agg demo.cast assets/demo.gif
   ```

## Updating the Demo

Whenever you update major features, regenerate:

```bash
vhs demo.tape
git add assets/demo.gif
git commit -m "docs: update demo gif"
```
