#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

if [ "$#" -eq 0 ]; then
  set -- "$SCRIPT_DIR/src" "$SCRIPT_DIR/output"
fi

exec cargo run --quiet \
  --manifest-path "$SCRIPT_DIR/Cargo.toml" \
  -- "$@"
