#!/bin/sh
set -eu
cd "$(dirname "$0")"
FUZZ4ALL_COMMIT=6d1169382e5470e8d83e86f327ce4215f307a12b
"${PYTHON:-python3.11}" -m venv .venv
.venv/bin/pip install -q -r requirements.txt
rm -rf .venv/src/fuzz4all
git clone -q https://github.com/fuzz4all/fuzz4all .venv/src/fuzz4all
git -C .venv/src/fuzz4all checkout -q "$FUZZ4ALL_COMMIT"
echo "$PWD/.venv/src/fuzz4all" > "$(.venv/bin/python -c 'import sysconfig; print(sysconfig.get_paths()["purelib"])')/fuzz4all.pth"
cargo build --release --manifest-path ../Cargo.toml
