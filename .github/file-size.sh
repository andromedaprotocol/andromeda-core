#!/bin/bash

find ./artifacts -maxdepth 1 -size +810k | while read file; do
  echo "$file is too large: $(wc -c "$file" | awk '{print $1}') bytes. Maximum: 810000 bytes."
  exit 1
done

exit 0