#!/bin/bash

find ./artifacts -maxdepth 1 -size +575k | while read file; do
  echo "$file is too large: $(wc -c "$file" | awk '{print $1}') bytes. Maximum: 575000 bytes."
  exit 1
done

exit 0