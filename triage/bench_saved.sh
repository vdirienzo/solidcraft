#!/usr/bin/env bash
# What does opening a saved design cost, and where does the time go?
#   1. the design is saved with its model cache;
#   2. it is opened again: the cache is taken, nothing is evaluated;
#   3. it is opened with the cache turned off (as after a kernel upgrade): the timeline is
#      evaluated again.
set -u
CLI=/home/user/solidcraft-wt/issue-14310/target/ci/solidcraft-cli
DESIGN=${1:-/home/user/solidcraft-wt/issue-14310/apps/solidcraft-cli/tests/corpus/m0-box-fillet-hole.solidcraft}
OUT=/tmp/opencode/saved.sc

echo "=== load: $(cut -d' ' -f1-3 /proc/loadavg)  free: $(free -m | awk 'NR==2 {print $7}') MB"
echo "=== design: $DESIGN"

echo
echo "--- 1. save it with its model cache"
"$CLI" bench-open "$DESIGN" --save "$OUT" 2>&1 | grep -E '"step":"(save|evaluate|parse|mesh)"'
ls -la "$OUT" | awk '{print "   saved:", $5, "bytes"}'

echo
echo "--- 2. open it again (the cache is taken)"
"$CLI" bench-open "$OUT" 2>&1 | grep -E '"step":"(evaluate|mesh|parse)"'

echo
echo "--- 3. open it with the cache turned off (as after a kernel upgrade)"
SOLIDCRAFT_NO_MODEL_CACHE=1 "$CLI" bench-open "$OUT" 2>&1 | grep -E '"step":"(evaluate|mesh|parse)"'

echo
echo "--- 4. the same with the profile (the table goes to stderr)"
"$CLI" bench-open "$OUT" --profile 2>&1 >/dev/null | head -25
