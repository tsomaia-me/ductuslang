#!/bin/bash
# Runs a cases file; prints every case whose outcome contradicts its (ok)/(refuse) mark, and all TREEFAULT lines.
P=/home/cdagent/cargo-targets/peer-syntax-review/conformance/packages/symphonyscript/target/debug/examples/probe
taskset -c 8-11,16,17 $P "$1" > "$1.out"
echo "exit=$?"
awk '
/^#### / { show = 0;
  if ($0 ~ /\(ok\)/ && $0 !~ /parse-diags=0$/) show = 1;
  if ($0 ~ /\(refuse\)/ && $0 ~ /parse-diags=0$/) show = 1;
  if (show) print "MISMATCH " $0; next }
/TREEFAULT/ { print }
show && /DIAG/ { print substr($0, 1, 260) }
/^CASES/ { print }
' "$1.out"
echo "ok-marked: $(grep -c '^#### .*(ok)' "$1.out")  refuse-marked: $(grep -c '^#### .*(refuse)' "$1.out")"
