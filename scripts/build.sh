#!/bin/bash

# EXAMPLE USAGE:
# build.sh andromeda-contract some-category
# Builds "andromeda-contract" contract and "some-category" category

# LOG all the contracts compiled with there compressed file size
FILE_LOG=""

get_version_filename (){
    local CONTRACT=$1
    # Get the version of the contract processed
    local BUILD_VERSION=$(cargo pkgid $CONTRACT | cut -d# -f2 | cut -d: -f2)
    local BUILD_TARGET=${CONTRACT//-/_}

    echo "$BUILD_TARGET@$BUILD_VERSION";
}

build_contract () {
    local CONTRACT_PATH=$1;
    local CONTRACT=`basename $CONTRACT_PATH`;
    echo "Building contract $CONTRACT..."

    # Detect architecture and use appropriate optimizer image
    if [[ $(uname -m) == "arm64" ]]; then
        OPTIMIZER_IMAGE="cosmwasm/optimizer-arm64:0.16.1"
    else
        OPTIMIZER_IMAGE="cosmwasm/optimizer:0.16.1"
    fi

    echo "Building $CONTRACT using $OPTIMIZER_IMAGE"

    # Use Docker optimizer for this specific contract
    docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        $OPTIMIZER_IMAGE $CONTRACT_PATH

    local BUILD_TARGET=${CONTRACT//-/_}
    local OUT_FILE="./artifacts/$BUILD_TARGET.wasm"
    local OUT_FILE_IBC_TEST="./tests/ibc-tests/artifacts/$BUILD_TARGET.wasm"
    local OUT_FILE_PACKAGE="./packages/andromeda-testing-e2e/artifacts/$BUILD_TARGET.wasm"

    # Copy the optimized wasm to required locations
    cp "./artifacts/$BUILD_TARGET.wasm" $OUT_FILE_IBC_TEST
    cp "./artifacts/$BUILD_TARGET.wasm" $OUT_FILE_PACKAGE
    
    # Log file sizes
    local FILESIZE=$(($(wc -c <"$OUT_FILE") +0))
    local LOG="$BUILD_TARGET \t\t: $FILESIZE bytes (optimized)"
    FILE_LOG="$FILE_LOG\n$LOG"
}

build_category () {
     for directory in contracts/*/; do
        if [ "$(basename $directory)" = "$1" ]; then
            echo "Building all contracts in category $(basename $directory)..."
            for contract in $directory/*/; do
                build_contract $contract;
            done
            break
        fi
    done
}

# Helper function to build all contracts with build all command
build_all() {
    for directory in contracts/*/; do
        build_category $(basename $directory)
    done
}

is_contract() {
    for directory in contracts/*/; do
        for contract in $directory/*/; do
            if [ "$(basename $contract)" = "$1" ]; then
                return 0
            fi
        done
    done
    return 1
}

is_category() {
    for directory in contracts/*/; do
        if [ "$(basename $directory)" = "$1" ]; then
            return 0
        fi
    done
    return 1
}

export RUSTFLAGS="-C link-arg=-s"

#Clear current builds
rm -rf ./target
rm -rf ./artifacts
rm -rf ./packages/andromeda-testing-e2e/artifacts
rm -rf ./tests/ibc-tests/artifacts
mkdir artifacts
mkdir packages/andromeda-testing-e2e/artifacts
mkdir tests/ibc-tests/artifacts

set -e
for target in "$@"; do
    if [ "$target" = "all" ]; then
        build_all
    elif is_contract $target; then
        build_contract $target
    elif is_category $target; then
        build_category $target
    else
        echo "$target is not a valid target"
        exit 1
    fi
    echo -e "$FILE_LOG"
done