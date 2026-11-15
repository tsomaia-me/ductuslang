#!/bin/sh
# Repeats the whole run.  Writes everything under ./out next to this script.
#   sh run.sh            # N=6 for the full enumeration and the reader comparison,
#                        # N=7 for the intersection listing (about 8 minutes)
#   sh run.sh 5 6        # other lengths: <N full> <N intersections>
set -e
cd "$(dirname "$0")"
N="${1:-6}"
NI="${2:-7}"
PY="nice -n 19 /usr/bin/python3 -I"
mkdir -p out
echo "== 0. grammar: alphabet and rule automata" | tee out/run.log
$PY grammar.py | tee -a out/run.log
echo "== 1. recogniser sanity tests against spec examples" | tee -a out/run.log
$PY tests.py | tee -a out/run.log
echo "== 2. enumeration and counts up to N=$N (out/counts.txt, out/intersection_*.txt)" | tee -a out/run.log
$PY counts.py "$N" out | tee -a out/run.log
echo "== 2b. intersections by shape up to N=$NI (out/intersections.txt)" | tee -a out/run.log
$PY inter.py "$NI" out 2>&1 | tee -a out/run.log
echo "== 3. single-pass reader against the recognisers up to N=$N, four positions (out/compare.txt)" | tee -a out/run.log
$PY compare.py "$N" out | tee -a out/run.log
echo "== done" | tee -a out/run.log
