#!/usr/bin/env bash
set -e

echo "🎬 Generating demo GIF for nr..."
echo ""

# Check if vhs is installed
if ! command -v vhs &> /dev/null; then
    echo "❌ VHS is not installed."
    echo ""
    echo "Install it with:"
    echo "  brew install vhs"
    echo ""
    echo "Or see: https://github.com/charmbracelet/vhs#installation"
    exit 1
fi

# Check if nr is built
if [ ! -f "target/release/nr" ]; then
    echo "📦 Building nr first..."
    cargo build --release
    echo "✅ Build complete"
    echo ""
fi

# Add nr to PATH for this session
export PATH="$PWD/target/release:$PATH"

# Check if nr is accessible
if ! command -v nr &> /dev/null; then
    echo "❌ nr binary not found in PATH"
    echo "Make sure target/release/nr exists"
    exit 1
fi

echo "🎥 Recording demo..."
vhs demo.tape

echo ""
echo "✅ Demo generated at: assets/demo.gif"
echo ""

# Show file size
if command -v du &> /dev/null; then
    SIZE=$(du -h assets/demo.gif | cut -f1)
    echo "📦 File size: $SIZE"
    echo ""
fi

echo "💡 Tips:"
echo "  - Preview: open assets/demo.gif"
echo "  - Optimize: gifsicle -O3 assets/demo.gif -o assets/demo.gif"
echo "  - Edit script: edit demo.tape and run this script again"
echo ""
