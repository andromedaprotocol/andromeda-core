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

  # Split base version and suffix
  BASE_VERSION=${VERSION%%-*}
  SUFFIX=${VERSION#"$BASE_VERSION"}
  if [[ "$SUFFIX" == "$VERSION" ]]; then
    SUFFIX=""
  fi

  IFS='.' read -r -a VERSION_PARTS <<< "$BASE_VERSION"

  MAJOR=${VERSION_PARTS[0]}
  MINOR=${VERSION_PARTS[1]}
  PATCH=${VERSION_PARTS[2]}

  # Extract suffix letter and number if present (e.g. "-a.4" or "-b.7")
  SUFFIX_LETTER=""
  SUFFIX_NUM=""
  if [[ $SUFFIX =~ ^-([a-zA-Z]+)\.([0-9]+)$ ]]; then
    SUFFIX_LETTER="${BASH_REMATCH[1]}"
    SUFFIX_NUM="${BASH_REMATCH[2]}"
  fi

  case $VERSION_BUMP in
    major)
      MAJOR=$((MAJOR + 1))
      MINOR=0
      PATCH=0
      SUFFIX="-a.1"
      ;;
    minor)
      MINOR=$((MINOR + 1))
      PATCH=0
      SUFFIX="-a.1"
      ;;
    patch)
      if [[ -n $SUFFIX_LETTER && -n $SUFFIX_NUM ]]; then
        # bump suffix number
        SUFFIX_NUM=$((SUFFIX_NUM + 1))
        SUFFIX="-$SUFFIX_LETTER.$SUFFIX_NUM"
      else
        PATCH=$((PATCH + 1))
        SUFFIX="" # no suffix originally, keep clean patch bump
      fi
      ;;
    *)
      echo "Invalid version bump type. Use 'patch', 'minor', or 'major'."
      return 1
      ;;
  esac

  NEW_VERSION="$MAJOR.$MINOR.$PATCH$SUFFIX"

  # Update the version in the Cargo.toml file
  sed -i.bak "s/version = \"$VERSION\"/version = \"$NEW_VERSION\"/" "$FILE"
  rm "$FILE.bak"

  echo "Updated $FILE to version $NEW_VERSION"
}

export -f bump_version

# Find all Cargo.toml files and bump their versions
find "$DIRECTORY" -name "Cargo.toml" -exec bash -c 'bump_version "$0" "$1"' {} "$VERSION_BUMP" \;
