#!/bin/bash
# runs on wb2, inside ~/cargo-targets/peer-floor
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")"
CPU=4
T="glibc.malloc.mmap_threshold=4294967296:glibc.malloc.trim_threshold=4294967296:glibc.malloc.top_pad=268435456"
mkdir -p results-wb2
( cd packages/symphonyscript && nice -n 19 cargo build --release -p symphonyscript --example floor -j 6 ) > build-release-wb2.log 2>&1 || exit 1
round() {
  bin="packages/symphonyscript/target/$1/examples/floor"; out="results-wb2/$1-$2.tsv"
  {
    echo "# $(date -u +%FT%TZ) host=$(hostname) profile=$1 round=$2 cpu=$CPU load=$(cut -d' ' -f1-3 /proc/loadavg) mhz=$(grep -m1 MHz /proc/cpuinfo | cut -d: -f2)"
    for pack in inputs/gen_*_80000.pack; do taskset -c $CPU "$bin" "$pack" all 700; done
    taskset -c $CPU "$bin" inputs/large.pack all 1500
    taskset -c $CPU "$bin" inputs/large_one_file.pack all 1500
    taskset -c $CPU "$bin" inputs/corpus.pack clean 1500
    taskset -c $CPU "$bin" inputs/corpus.pack faulty 1500
    taskset -c $CPU "$bin" inputs/calls_1000000.pack all 1500
    echo "# tunables"
    GLIBC_TUNABLES="$T" taskset -c $CPU "$bin" inputs/large.pack all 1500
    GLIBC_TUNABLES="$T" taskset -c $CPU "$bin" inputs/calls_1000000.pack all 1500
    echo "# $(date -u +%FT%TZ) end load=$(cut -d' ' -f1-3 /proc/loadavg) mhz_cpu$CPU=$(awk -v c=$CPU '$1=="processor"&&$3==c{f=1} f&&/MHz/{print $4; exit}' /proc/cpuinfo)"
  } > "$out" 2>&1
}
round release 1; round release 2; round release 3
if [ "$1" = "linear" ]; then
  { echo "# linear"; for pack in inputs/gen_*_10000.pack inputs/gen_*_20000.pack inputs/gen_*_40000.pack; do taskset -c $CPU packages/symphonyscript/target/release/examples/floor "$pack" all 500; done; } > results-wb2/release-sizes.tsv 2>&1
fi
( cd packages/symphonyscript && nice -n 19 cargo build -p symphonyscript --example floor -j 6 ) > build-dev-wb2.log 2>&1 || exit 2
round debug 1
rm -rf packages/symphonyscript/target
echo done > results-wb2/DONE
