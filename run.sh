#!/usr/bin/env sh
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
if [ ! -x "target/release/api" ]; then
    echo "API binary not found. Run ./build.sh first." >&2
    exit 1
fi
exec ./target/release/api "$@"
