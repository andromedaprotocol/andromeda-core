#!/bin/bash

# Check if directory and version bump type are provided
if [ $# -ne 2 ]; then
  echo "Usage: $0 <directory> <version_bump>"
  echo "version_bump should be one of: patch, minor, major"
  exit 1
fi

DIRECTORY=$1
VERSION_BUMP=$2

# Function to bump the version in a Cargo.toml file
bump_version() {
  FILE=$1
  VERSION_BUMP=$2

  # Extract the current version
  VERSION=$(grep '^version =' "$FILE" | sed -E 's/version = "(.*)"/\1/')
  
  if [[ ! $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "No valid version found in $FILE"
    return 1
  fi

  IFS='.' read -r -a VERSION_PARTS <<< "$VERSION"

  MAJOR=${VERSION_PARTS[0]}
  MINOR=${VERSION_PARTS[1]}
  PATCH=${VERSION_PARTS[2]}

  # Bump the version based on the input
  case $VERSION_BUMP in
    major)
      MAJOR=$((MAJOR + 1))
      MINOR=0
      PATCH=0
      ;;
    minor)
      MINOR=$((MINOR + 1))
      PATCH=0
      ;;
    patch)
      PATCH=$((PATCH + 1))
      ;;
    *)
      echo "Invalid version bump type. Use 'patch', 'minor', or 'major'."
      return 1
      ;;
  esac

  NEW_VERSION="$MAJOR.$MINOR.$PATCH"
  
  # Update the version in the Cargo.toml file
  sed -i.bak "s/version = \"$VERSION\"/version = \"$NEW_VERSION\"/" "$FILE"
  rm "$FILE.bak"

  echo "Updated $FILE to version $NEW_VERSION"
}

export -f bump_version

# Find all Cargo.toml files and bump their versions
find "$DIRECTORY" -name "Cargo.toml" -exec bash -c 'bump_version "$0" "$1"' {} "$VERSION_BUMP" \;

