#!/bin/bash

# Create hooks directory if it doesn't exist
mkdir -p .git/hooks

# Create symlink to pre-push hook
ln -sf ../../scripts/pre-push.sh .git/hooks/pre-push

# Make both scripts executable
chmod +x scripts/pre-push.sh
chmod +x .git/hooks/pre-push

echo "Git hooks installed successfully!" 