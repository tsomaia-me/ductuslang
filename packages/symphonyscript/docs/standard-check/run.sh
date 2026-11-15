#!/bin/sh
# The check of STANDARD.md's worked example (3.11). Run it by hand after any edit of the
# standard: `packages/symphonyscript/docs/standard-check/run.sh`, judged by its exit code.
#
# It takes both forms of the example exactly as the document prints them.
# - The good form is compiled alone with warnings as errors. claims.py then reads its text
#   for what "What to see in it" says of it. check_tail.rs runs it: the trees and the faults,
#   agreement with a second reader on every short sequence, 100,000 levels and more on a
#   1 MiB stack, the counted turns, the counted allocations.
# - The bad form is compiled with bad_prelude.rs and bad_tail.rs, which hold what it does not
#   print, and run on the inputs its fault table names. Its three depth runs must die.
#
# STANDARD names another file to check. Everything the run writes goes to a folder of its own
# beside this script, since a system temporary folder may refuse to run a program, and the
# folder is removed when the run ends.
set -e
here="$(cd "$(dirname "$0")" && pwd)"
STANDARD="${STANDARD:-$here/../STANDARD.md}"
RUSTC="${RUSTC:-$HOME/.cargo/bin/rustc}"
out="$(mktemp -d "$here/run.XXXXXX")"
trap 'rm -rf "$out"' EXIT
cd "$here"
block() { awk -v n="$1" '/^```rust$/{c++; if(c==n){on=1; next}} on && /^```$/{exit} on{print}' "$STANDARD"; }
block 1 > "$out/good.rs"
block 2 > "$out/bad.rs"
nice -n 19 "$RUSTC" --edition 2024 --crate-type lib --crate-name example -D warnings "$out/good.rs" --out-dir "$out"
python3 claims.py "$out/good.rs"
cat "$out/good.rs" check_tail.rs > "$out/combined.rs"
nice -n 19 "$RUSTC" --edition 2024 --test -O -D warnings -A dead_code "$out/combined.rs" -o "$out/combined_test"
"$out/combined_test" --nocapture
cat bad_prelude.rs "$out/bad.rs" bad_tail.rs > "$out/bad_combined.rs"
nice -n 19 "$RUSTC" --edition 2024 -O -A warnings "$out/bad_combined.rs" -o "$out/bad_run"
"$out/bad_run"
for shape in neg paren pow; do
    if "$out/bad_run" "$shape" >/dev/null 2>&1; then
        echo "bad form, depth $shape: survived"
        exit 1
    fi
    echo "bad form, depth $shape: died"
done
