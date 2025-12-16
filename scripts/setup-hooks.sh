#!/bin/bash

# Setup git hooks for the repository
# Run this once after cloning the repository

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Setting up git hooks..."

# Configure git to use .githooks directory
git config core.hooksPath .githooks

echo "Git hooks configured successfully!"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Checks formatting (cargo fmt) and TOML sorting"
echo ""
echo "To skip hooks temporarily, use: git commit --no-verify"
