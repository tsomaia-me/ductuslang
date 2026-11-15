#!/bin/bash
# usage: run.sh <profile-dir> <out-file> <round>
bin="packages/symphonyscript/target/$1/examples/floor"
out="$2"
round="$3"
cd "$(dirname "$0")"
{
  echo "# $(date -u +%FT%TZ) profile=$1 round=$round load=$(cut -d' ' -f1-3 /proc/loadavg)"
  if [ "$round" = "1" ]; then
    for pack in inputs/gen_*.pack; do
      taskset -c 6 "$bin" "$pack" all 700
    done
  else
    for pack in inputs/gen_*_80000.pack; do
      taskset -c 6 "$bin" "$pack" all 700
    done
  fi
  taskset -c 6 "$bin" inputs/large.pack all 1500
  taskset -c 6 "$bin" inputs/large_one_file.pack all 1500
  taskset -c 6 "$bin" inputs/corpus.pack clean 1500
  taskset -c 6 "$bin" inputs/corpus.pack faulty 1500
  echo "# $(date -u +%FT%TZ) end load=$(cut -d' ' -f1-3 /proc/loadavg)"
} > "$out" 2>&1
