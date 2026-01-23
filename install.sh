#!/bin/bash
set -e

# CWA - Claude Workflow Architect
# Installation script for macOS

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="cwa"

echo "=== CWA Installation Script ==="
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Rust is not installed. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo "Rust version: $(rustc --version)"
echo "Cargo version: $(cargo --version)"
echo ""

# Check optional dependencies
echo "Checking optional dependencies..."

if command -v docker &> /dev/null; then
    echo "  Docker:  $(docker --version | head -1)"
else
    echo "  Docker:  not installed (optional - needed for 'cwa infra' commands)"
fi

if command -v docker compose version &> /dev/null 2>&1 || command -v docker-compose &> /dev/null 2>&1; then
    echo "  Compose: available"
else
    echo "  Compose: not available (optional - needed for 'cwa infra' commands)"
fi

echo ""

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building CWA in release mode..."
cargo build --release

# Check if build succeeded
if [ ! -f "target/release/$BINARY_NAME" ]; then
    echo "Error: Build failed. Binary not found."
    exit 1
fi

# Show binary size
BINARY_SIZE=$(du -h "target/release/$BINARY_NAME" | cut -f1)
echo "Build successful! (binary size: $BINARY_SIZE)"
echo ""

# Install binary
echo "Installing to $INSTALL_DIR..."

# Check if we need sudo
if [ -w "$INSTALL_DIR" ]; then
    cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
else
    echo "Requesting sudo access to install to $INSTALL_DIR"
    sudo cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
fi

# Make executable
chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo ""
echo "=== Installation Complete ==="
echo ""
echo "CWA installed to: $INSTALL_DIR/$BINARY_NAME"
echo "Version: $($INSTALL_DIR/$BINARY_NAME --version 2>/dev/null || echo 'unknown')"
echo ""
echo "Quick start:"
echo "  cwa init my-project          # Initialize a new project"
echo "  cwa --help                   # Show all commands"
echo ""
echo "Optional infrastructure (requires Docker):"
echo "  cwa infra up                 # Start Neo4j, Qdrant, Ollama"
echo "  cwa infra status             # Check service health"
echo ""

# Verify installation
if command -v cwa &> /dev/null; then
    echo "Installation verified successfully!"
else
    echo "Note: Make sure $INSTALL_DIR is in your PATH"
    echo "Add this to your ~/.zshrc or ~/.bash_profile:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
fi
