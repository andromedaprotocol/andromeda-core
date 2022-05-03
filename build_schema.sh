#!/bin/bash

set -e
set -o pipefail

for c in contracts/*; do
    (cd $c && cargo schema)
done
