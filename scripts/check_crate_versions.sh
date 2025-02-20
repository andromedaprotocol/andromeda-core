#!/bin/bash

# Initialize arrays for different categories
MACROS_CRATE=""
STD_CRATE=""
PACKAGE_CRATES=()
CONTRACT_CRATES=()

check_crate_version() {
    local cargo_toml=$1
    local crate_name=$(grep '^name = ' $cargo_toml | cut -d'"' -f2)
    local local_version=$(grep '^version = ' $cargo_toml | cut -d'"' -f2)
    
    # Get latest version from crates.io
    local remote_version=$(cargo search $crate_name --limit 1 | grep "^$crate_name = " | cut -d'"' -f2 || echo "0.0.0")
    
    if [ "$local_version" != "$remote_version" ]; then
        echo "Version change detected in $crate_name: crates.io($remote_version) -> local($local_version)"
        return 0
    fi
    return 1
}

# Check macros crate
if [ -f "packages/std/macros/Cargo.toml" ] && check_crate_version "packages/std/macros/Cargo.toml"; then
    MACROS_CRATE="packages/std/macros"
fi

# Check std crate
if [ -f "packages/std/Cargo.toml" ] && check_crate_version "packages/std/Cargo.toml"; then
    STD_CRATE="packages/std"
fi

# Check other packages
for toml in packages/*/Cargo.toml; do
    if [[ "$toml" != "packages/std/Cargo.toml" && "$toml" != "packages/std/macros/Cargo.toml" ]]; then
        if check_crate_version "$toml"; then
            PACKAGE_CRATES+=($(dirname "$toml"))
        fi
    fi
done

# Check contracts
for toml in contracts/*/*/Cargo.toml; do
    if check_crate_version "$toml"; then
        CONTRACT_CRATES+=($(dirname "$toml"))
    fi
done

# Set outputs for GitHub Actions
echo "macros_crate=$MACROS_CRATE" >> $GITHUB_OUTPUT
echo "std_crate=$STD_CRATE" >> $GITHUB_OUTPUT
echo "package_crates=${PACKAGE_CRATES[@]}" >> $GITHUB_OUTPUT
echo "contract_crates=${CONTRACT_CRATES[@]}" >> $GITHUB_OUTPUT 