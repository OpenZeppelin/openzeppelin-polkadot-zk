#!/bin/bash

# Setup git hooks for the repository
# Run this once after cloning the repository

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

echo "Setting up git hooks..."

# Ensure scripts are executable
chmod +x "$SCRIPT_DIR/pre-commit"
chmod +x "$ROOT_DIR/.githooks/pre-commit"

# Configure git to use .githooks directory
git config core.hooksPath .githooks

echo ""
echo "Git hooks configured successfully!"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Runs CI-aligned checks (toml-sort, cargo fmt, cargo check)"
echo ""
echo "Hook options:"
echo "  - Skip hooks:        git commit --no-verify"
echo "  - Include tests:     RUN_TESTS=1 git commit"
echo "  - Include doc check: RUN_DOC=1 git commit"
echo ""
echo "You can also run checks manually:"
echo "  ./scripts/pre-commit"
