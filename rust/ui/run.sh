#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

exec cargo run --quiet \
  --manifest-path "$SCRIPT_DIR/Cargo.toml" \
  -- "$SCRIPT_DIR/layout.xml"
