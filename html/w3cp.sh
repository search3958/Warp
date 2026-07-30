#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
output_dir=${1:-"$project_dir/dist"}

cd "$project_dir"
cargo build --release --quiet
"$project_dir/target/release/w3cp" "$project_dir" "$output_dir"

printf 'Generated: %s/index.html, app.js, style.css\n' "$output_dir"

