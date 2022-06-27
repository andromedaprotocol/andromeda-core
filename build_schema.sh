#!/bin/bash

set -e
set -o pipefail

for c in contracts/app/*; do
    (cd $c && cargo schema)
done

for c in contracts/data-storage/*; do
    (cd $c && cargo schema)
done

for c in contracts/ecosystem/*; do
    (cd $c && cargo schema)
done

for c in contracts/finance/*; do
    (cd $c && cargo schema)
done

for c in contracts/fungible-tokens/*; do
    (cd $c && cargo schema)
done

for c in contracts/modules/*; do
    (cd $c && cargo schema)
done

for c in contracts/non-fungible-tokens/*; do
    (cd $c && cargo schema)
done
