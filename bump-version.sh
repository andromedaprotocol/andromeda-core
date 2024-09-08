#!/bin/bash

# Check if directory and version bump type are provided
if [ $# -lt 2 ] || [ $# -gt 4 ]; then
  echo "Usage: $0 <directory> <version_bump|--prerelease-only> [prerelease_tag]"
  echo "version_bump should be one of: patch, minor, major, or use '--prerelease-only' to add a prerelease tag without bumping"
  exit 1
fi

DIRECTORY=$1
VERSION_BUMP=$2
PRERELEASE_TAG=$3

# Function to bump the version or add a prerelease tag in a Cargo.toml file
bump_or_add_prerelease() {
  FILE=$1
  VERSION_BUMP=$2
  PRERELEASE_TAG=$3

  # Extract the current version
  VERSION=$(grep '^version =' "$FILE" | sed -E 's/version = "(.*)"/\1/')
  
  if [[ ! $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
    echo "No valid version found in $FILE"
    return 1
  fi

  # Remove any existing prerelease tag
  BASE_VERSION=$(echo "$VERSION" | sed -E 's/-[a-zA-Z0-9]+//')

  IFS='.' read -r -a VERSION_PARTS <<< "$BASE_VERSION"

  MAJOR=${VERSION_PARTS[0]}
  MINOR=${VERSION_PARTS[1]}
  PATCH=${VERSION_PARTS[2]}

  if [[ "$VERSION_BUMP" == "--prerelease-only" ]]; then
    # Only add the prerelease tag without bumping the version
    NEW_VERSION="$BASE_VERSION"
  else
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
        echo "Invalid version bump type. Use 'patch', 'minor', 'major', or '--prerelease-only'."
        return 1
        ;;
    esac
    NEW_VERSION="$MAJOR.$MINOR.$PATCH"
  fi

  # Add prerelease tag if provided
  if [ -n "$PRERELEASE_TAG" ]; then
    NEW_VERSION="$NEW_VERSION-$PRERELEASE_TAG"
  fi

  # Update the version in the Cargo.toml file
  sed -i.bak "s/version = \"$VERSION\"/version = \"$NEW_VERSION\"/" "$FILE"
  rm "$FILE.bak"

  echo "Updated $FILE to version $NEW_VERSION"
}

export -f bump_or_add_prerelease

# Find all Cargo.toml files and bump their versions or add prerelease tags
find "$DIRECTORY" -name "Cargo.toml" -exec bash -c 'bump_or_add_prerelease "$0" "$1" "$2"' {} "$VERSION_BUMP" "$PRERELEASE_TAG" \;
