#!/bin/bash

# Script to detect crates with changes in dependencies

# Get the list of changed files
CHANGED_FILES=$(git diff --name-only HEAD^ HEAD)

# Initialize an array for crates needing a version bump
CRATES_TO_BUMP=()

# Check each changed file
for file in $CHANGED_FILES; do
    if [[ "$file" == */Cargo.toml || "$file" == */Cargo.lock ]]; then
        CRATE_DIR=$(dirname "$file")
        if [[ ! " ${CRATES_TO_BUMP[@]} " =~ " ${CRATE_DIR} " ]]; then
            CRATES_TO_BUMP+=("$CRATE_DIR")
        fi
    fi
done

# Output the crates that need a version bump
echo "${CRATES_TO_BUMP[@]}"