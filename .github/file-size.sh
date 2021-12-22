#!/bin/bash
for FILENAME in ./artifacts/*

    do
    FILESIZE=$(wc -c "$FILENAME" | awk '{print $1}')
        FILESIZE=$(stat -printf='%s' "$FILENAME")
        echo "Size of $FILENAME = $FILESIZE bytes."

    done

exit 0