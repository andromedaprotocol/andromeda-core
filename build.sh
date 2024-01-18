#!/bin/bash

# EXAMPLE USAGE:
# build.sh andromeda-contract some-category
# Builds "andromeda-contract" contract and "some-category" category

# LOG all the contracts compiled with there compressed file size
local FILE_LOG=""

build_contract () {
    local CONTRACT=$1
    echo "Building contract $CONTRACT..."
    cargo wasm -p $CONTRACT -q

    # Get the version of the contract processed
    local BUILD_VERSION=$(cargo pkgid $CONTRACT | cut -d# -f2 | cut -d: -f2)
    local BUILD_TARGET=${CONTRACT//-/_}
    local IN_FILE="./target/wasm32-unknown-unknown/release/$BUILD_TARGET.wasm"
    local OUT_FILE="./artifacts/$BUILD_TARGET@$BUILD_VERSION.wasm"
    wasm-opt -Os $IN_FILE -o $OUT_FILE
    
    # NOT SO IMPORTANT STEPS
    # Log wasm file sizes at the end of build process
    local IN_FILESIZE=$(($(wc -c <"$IN_FILE") +0))
    local OUT_FILESIZE=$(($(wc -c <"$OUT_FILE") +0))
    local LOG="$BUILD_TARGET \t\t: $IN_FILESIZE \t- $OUT_FILESIZE bytes"
    FILE_LOG="$FILE_LOG\n$LOG"
}

build_category () {
     for directory in contracts/*/; do
        if [[ "$(basename $directory)" = "$1" ]]; then
            echo "Building all contracts in category $(basename $directory)..."
            for contract in $directory/*/; do
                build_contract $(basename $contract)
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
            if [[ "$(basename $contract)" = "$1" ]]; then
                return 0
            fi
        done
    done
    return 1
}

is_category() {
    for directory in contracts/*/; do
        if [[ "$(basename $directory)" = "$1" ]]; then
            return 0
        fi
    done
    return 1
}

export RUSTFLAGS="-C link-arg=-s"

#Clear current builds
rm -rf ./target
rm -rf ./artifacts
mkdir artifacts

for target in "$@"; do
    if [[ "$target" = "all" ]]; then
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