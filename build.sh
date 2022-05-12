#!/bin/bash

# EXAMPLE USAGE:
# build.sh andromeda-contract some-category
# Builds "andromeda-contract" contract and "some-category" category

build_contract () {
    local CONTRACT=$1
    echo "Building contract $CONTRACT..."
    cargo wasm -p $CONTRACT
    local BUILD_TARGET=${CONTRACT//-/_}
    wasm-opt -Os ./target/wasm32-unknown-unknown/release/$BUILD_TARGET.wasm -o ./artifacts/$BUILD_TARGET.wasm
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
    if is_contract $target; then
        build_contract $target
    elif is_category $target; then
        build_category $target
    else
        echo "$target is not a valid target"
    fi
done