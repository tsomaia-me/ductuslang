#!/bin/bash
# Property 7: min of 5 parse runs per size, pinned to core 15, release build.
P=/home/cdagent/cargo-targets/peer-syntax-review/recovery/packages/symphonyscript/target/release/examples/probe
run() { # shape n0
  local sh=$1 n=$2
  for k in 1 2 3 4; do
    taskset -c 15 $P time $sh $n 5
    n=$((n*2))
  done
}
for sh in unclosed-paren unclosed-brace open-lines-paren open-lines-bracket unclosed-lines \
          stray-closers stray-closer-lines faulty-lines faulty-lines-in-bracket faulty-lines-in-call \
          fault-then-valid open-bracket-then-valid open-bracket-then-placements open-group-then-placements \
          open-strings open-strings-one-line interp-unclosed-lines misindented misindented-unexpected tabs \
          long-line long-line-faulty long-line-in-bracket-faulty long-placement-line \
          attr-clause attr-clause-faulty nested-blocks-broken fn-bodies-broken unclosed-bundle unclosed-call-lines; do
  run $sh 10000
done
for sh in open-lines-brace open-lines-mixed nested-map-lines-valid; do run $sh 5000; done
for sh in placement-lines-open-group body-lines-open-group placement-lines-open-type; do run $sh 500; done
for n in 1000 2000 4000 8000; do taskset -c 15 $P time many-dedents-mismatch $n 5; done
