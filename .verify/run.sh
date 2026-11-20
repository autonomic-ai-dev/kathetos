#!/bin/sh
set -u

ROOT=$(cd "$(dirname "$0")/.." && pwd)
FEATURE=${1:-all}
RUN_ID=$(date +%Y%m%dT%H%M%S)
OUT="$ROOT/.verify/artifacts/$RUN_ID"
mkdir -p "$OUT"
cd "$ROOT" || exit 2

phase() { echo "== $1"; }
fail_phase() { echo "FAILED: $1 (exit $2)"; echo "run: $OUT"; exit "$2"; }

phase doctor
sh "$ROOT/.verify/doctor.sh" > "$OUT/doctor.txt" 2>&1 || fail_phase doctor 3
cat "$OUT/doctor.txt"

phase unit
cargo test > "$OUT/unit.txt" 2>&1 || fail_phase unit 4
grep "test result" "$OUT/unit.txt"

cargo build --release > "$OUT/build.txt" 2>&1 || fail_phase build 4

phase static
CONTRACT="$ROOT/.verify/features/rule-admission.md"
missing=0
for case_name in $(grep '^run_case ' "$ROOT/.verify/functional/cli.sh" | awk '{print $2}'); do
  if ! grep -q "$case_name" "$CONTRACT"; then
    echo "static: case $case_name has no step in $CONTRACT"
    missing=1
  fi
done
[ "$missing" -eq 0 ] || fail_phase static 5
echo "static: every functional case has a contract step"

phase functional
sh "$ROOT/.verify/functional/cli.sh" "$ROOT/target/release/kathetos" "$OUT/functional" > "$OUT/functional.txt" 2>&1
functional_status=$?
cat "$OUT/functional.txt"
[ "$functional_status" -eq 0 ] || fail_phase functional 6

phase corpus
sh "$ROOT/.verify/corpus/run.sh" "$ROOT/target/release/kathetos" "$OUT/corpus" > "$OUT/corpus.txt" 2>&1
corpus_status=$?
cat "$OUT/corpus.txt"
[ "$corpus_status" -eq 0 ] || fail_phase corpus 7

echo "PASS: all phases (run $RUN_ID)"
echo "artifacts: $OUT"
