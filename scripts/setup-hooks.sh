#!/bin/bash
#
# Setup script to install git hooks.
# Run from the repository root: ./scripts/setup-hooks.sh

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the root of the git repository
ROOT=$(git rev-parse --show-toplevel)
HOOKS_DIR="$ROOT/.git/hooks"
SCRIPTS_DIR="$ROOT/scripts"

echo -e "${YELLOW}Setting up git hooks...${NC}"

# Install pre-commit hook
if [ -f "$SCRIPTS_DIR/pre-commit" ]; then
    cp "$SCRIPTS_DIR/pre-commit" "$HOOKS_DIR/pre-commit"
    chmod +x "$HOOKS_DIR/pre-commit"
    echo -e "${GREEN}Installed pre-commit hook${NC}"
else
    echo "Warning: pre-commit script not found at $SCRIPTS_DIR/pre-commit"
fi

echo -e "\n${GREEN}Git hooks setup complete!${NC}"
echo -e "\nThe following hooks are now active:"
echo -e "  - ${YELLOW}pre-commit${NC}: Runs fmt, toml-sort, and cargo check"
echo -e "\nTo skip the pre-commit hook, use: ${YELLOW}git commit --no-verify${NC}"
echo -e "To include tests in pre-commit, use: ${YELLOW}RUN_TESTS=1 git commit${NC}"
