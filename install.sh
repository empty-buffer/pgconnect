#!/bin/bash
# Installation script for pgconnect

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default installation directory
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

echo -e "${GREEN}Installing pgconnect...${NC}"

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo is not installed. Please install Rust first.${NC}"
    echo "Visit: https://rustup.rs/"
    exit 1
fi

# Build the release binary
echo -e "${YELLOW}Building release binary...${NC}"
cargo build --release

# Check if build was successful
if [ ! -f "target/release/pgconnect" ]; then
    echo -e "${RED}Error: Build failed or binary not found${NC}"
    exit 1
fi

# Determine installation method
if [ "$EUID" -eq 0 ]; then
    # Running as root - install to system directory
    INSTALL_DIR="/usr/local/bin"
elif [ -w "/usr/local/bin" ]; then
    # User has write access to /usr/local/bin
    INSTALL_DIR="/usr/local/bin"
else
    # Fall back to user's cargo bin directory
    INSTALL_DIR="$HOME/.cargo/bin"
    echo -e "${YELLOW}Note: Installing to $INSTALL_DIR (not in PATH? Add it to your shell config)${NC}"
fi

# Copy binary
echo -e "${YELLOW}Installing binary to $INSTALL_DIR...${NC}"
cp target/release/pgconnect "$INSTALL_DIR/pgconnect"
chmod +x "$INSTALL_DIR/pgconnect"

# Verify installation
if command -v pgconnect &> /dev/null; then
    echo -e "${GREEN}✓ pgconnect installed successfully!${NC}"
    echo -e "Run 'pgconnect --help' to get started."
else
    echo -e "${YELLOW}Installation complete, but pgconnect is not in your PATH.${NC}"
    echo -e "Add $INSTALL_DIR to your PATH, or run: $INSTALL_DIR/pgconnect"
fi
