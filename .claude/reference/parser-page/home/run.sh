#!/bin/sh
# The peer's trial for the parser's page, question 1. Compiles home.rs alone with warnings as
# errors, then with tail.rs, and runs the trees, the faults and 100,000 levels on a 1 MiB stack.
set -e
cd "$(dirname "$0")"
RUSTC="${RUSTC:-$HOME/.cargo/bin/rustc}"
nice -n 19 "$RUSTC" --edition 2024 --crate-type lib --crate-name home -D warnings home.rs --out-dir .
cat home.rs tail.rs > combined.rs
nice -n 19 "$RUSTC" --edition 2024 --test -O -D warnings -A dead_code combined.rs -o combined_test
./combined_test
rm -f combined.rs combined_test libhome.rlib
