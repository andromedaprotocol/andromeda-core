#!/bin/bash
set -euo pipefail

mkdir -p ./artifacts

# Determine architecture and set custom image tag
if [[ $(uname -m) == "arm64" ]]; then
  OPTIMIZER_TAG="cosmwasm/optimizer-arm64:main"
  PLATFORM="linux/arm64"
else
  OPTIMIZER_TAG="cosmwasm/optimizer:main"
  PLATFORM="linux/amd64"
fi

# Check if the image already exists
if ! docker image inspect "$OPTIMIZER_TAG" > /dev/null 2>&1; then
  echo "🔧 Building $OPTIMIZER_TAG from GitHub repo..."
  docker buildx build --platform "$PLATFORM" \
    --pull \
    --tag "$OPTIMIZER_TAG" \
    "https://github.com/CosmWasm/optimizer.git#main"
fi

# Build the custom image that adds clang
docker build --build-arg OPTIMIZER_IMAGE="$OPTIMIZER_TAG" -t cosmwasm-optimizer-clang .

echo "📦 Building contracts for $(uname -m) architecture..."
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm-optimizer-clang
