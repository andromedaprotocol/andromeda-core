#!/bin/sh

# Pre-push hook to run tests before pushing
# Called by "git push" after it has checked the remote status, but before anything has been pushed.
# If this script exits with a non-zero status nothing will be pushed.

echo "Running lints and tests before push..."

echo "Running lints..."
make lint
LINT_RESULT=$?

if [ $LINT_RESULT -ne 0 ]; then
    echo "Linting failed. Push aborted."
    exit 1
fi

# Check for WIP commits (optional, from the example)
remote="$1"
url="$2"

z40=0000000000000000000000000000000000000000

while read local_ref local_sha remote_ref remote_sha
do
    if [ "$local_sha" = $z40 ]
    then
        # Handle delete
        :
    else
        if [ "$remote_sha" = $z40 ]
        then
            # New branch, examine all commits
            range="$local_sha"
        else
            # Update to existing branch, examine new commits
            range="$remote_sha..$local_sha"
        fi
    fi
done

exit 0 