#!/bin/bash

set -e
set -o pipefail

for directory in contracts/*/; do
    for contract in $directory/*/; do
        ( cd $contract && cargo schema )
    done
done