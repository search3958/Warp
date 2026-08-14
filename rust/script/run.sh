#!/usr/bin/env bash

set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
BUILD_DIR="$(mktemp -d)"
trap 'rm -rf "$BUILD_DIR"' EXIT

rustc --edition=2021 \
  "$SCRIPT_DIR/main.rs" \
  -o "$BUILD_DIR/warp4-script"

exec "$BUILD_DIR/warp4-script" "$SCRIPT_DIR/script-tester.w4s"
