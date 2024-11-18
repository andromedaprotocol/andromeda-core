#!/bin/bash

# Directory containing the contracts
CONTRACTS_DIR="contracts"
OUTPUT_FILE="./artifacts/version_map.json"

# Create artifacts directory if it doesn't exist
mkdir -p "$(dirname "$OUTPUT_FILE")"

# Ensure we can write to the output file
if ! touch "$OUTPUT_FILE" 2>/dev/null; then
    echo "Error: Cannot write to $OUTPUT_FILE" >&2
    exit 1
fi

# Start JSON object
echo "{" > $OUTPUT_FILE

# Create an array to store all entries
declare -a entries=()

# Loop through all contracts and collect their versions
for directory in $CONTRACTS_DIR/*/; do
    for contract_path in $directory/*/; do
        contract_name=`basename $contract_path`
        version=$(cargo pkgid $contract_name | cut -d# -f2 | cut -d: -f2)
        if [ ${#entries[@]} -eq 0 ]; then
            entries+=("  \"$contract_name\": \"$version\"")
        else
            entries+=("\n  \"$contract_name\": \"$version\"")
        fi
    done
done


# Join entries with comma and newline
(IFS=$',\n'; echo -e "${entries[*]}") >> $OUTPUT_FILE

# Close JSON object
echo "}" >> $OUTPUT_FILE

echo "Version map generated in $OUTPUT_FILE"
