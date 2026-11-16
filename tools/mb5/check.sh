#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
RUST_DIR="$ROOT/runner/rust"
TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/mb5-check.XXXXXX")
trap 'rm -rf "$TMP_DIR"' EXIT HUP INT TERM

printf '%s\n' '[1/4] MB5 project lint'
python3 "$ROOT/tools/mb5/lint.py" "$ROOT/example/components"

printf '%s\n' '[2/4] Script VM tests'
cc -std=c99 -O2 -Wall -Wextra -Werror \
  "$RUST_DIR/tests/script_vm_test.c" "$RUST_DIR/src/scriptvm/mb5_vm.c" -lm \
  -o "$TMP_DIR/script_vm_test"
"$TMP_DIR/script_vm_test"

printf '%s\n' '[3/4] Script syntax check'
cc -std=c99 -O2 -Wall -Wextra -Werror \
  "$RUST_DIR/tests/script_vm_check.c" "$RUST_DIR/src/scriptvm/mb5_vm.c" -lm \
  -o "$TMP_DIR/script_vm_check"
"$TMP_DIR/script_vm_check" "$ROOT/example/components/scripts/main.mb5"

printf '%s\n' '[4/4] Rust build availability'
if command -v cargo >/dev/null 2>&1; then
  cargo check --manifest-path "$RUST_DIR/Cargo.toml"
else
  printf '%s\n' 'SKIP: cargo is not installed; Rust integration was not compiled.'
fi
