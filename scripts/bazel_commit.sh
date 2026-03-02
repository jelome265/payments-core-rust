#!/bin/bash

# Bazel-driven commit script for multi-repo layout.
# This script is called via 'bazelisk run //:commit -- -m "message"'

if [ -z "$1" ] || [ "$1" != "-m" ] || [ -z "$2" ]; then
    echo "Usage: bazelisk run //:commit -- -m \"commit message\""
    exit 1
fi

MESSAGE="$2"

echo "[BAZEL-COMMIT] Staging all changes in $(pwd)..."
git add .

echo "[BAZEL-COMMIT] Committing with message: $MESSAGE"
git commit -m "$MESSAGE"

if [ $? -eq 0 ]; then
    echo "[BAZEL-COMMIT] Success."
else
    echo "[BAZEL-COMMIT] Commit failed (possibly no changes to commit)."
fi
