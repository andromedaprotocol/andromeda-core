#!/bin/bash
mkdir -p ./artifacts

if [[ $(uname -m) == "arm64" ]]; then
  OPTIMIZER_IMAGE="cosmwasm/optimizer-arm64:0.16.1"
else
  OPTIMIZER_IMAGE="cosmwasm/optimizer:0.16.1"
fi

docker build --build-arg OPTIMIZER_IMAGE="$OPTIMIZER_IMAGE" -t cosmwasm-optimizer-clang .

echo "Building contracts for $(uname -m) architecture"
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm-optimizer-clang