#!/bin/bash

# Directory containing the contracts
CONTRACTS_DIR="contracts"
OUTPUT_FILE="./artifacts/version_map.json"

# Create artifacts directory if it doesn't exist
mkdir -p $(dirname "$OUTPUT_FILE")

# Start JSON object
echo "{" > $OUTPUT_FILE

# Create an array to store all entries
declare -a entries=()

# Collect all entries first
while read -r file; do
    contract_dir=$(dirname "$file")
    
    # Extract version and crate name from Cargo.toml
    version=$(grep -m 1 "^version = " "$contract_dir/Cargo.toml" | cut -d '"' -f 2)
    crate_name=$(grep -m 1 "^name = " "$contract_dir/Cargo.toml" | cut -d '"' -f 2)
    
    entries+=("  \"$crate_name\": \"$version\"")
done < <(find $CONTRACTS_DIR -type f -name "Cargo.toml")

# Join entries with comma and newline
(IFS=$',\n'; echo "${entries[*]}") >> $OUTPUT_FILE

# Close JSON object
echo "}" >> $OUTPUT_FILE

echo "Version map generated in $OUTPUT_FILE"
