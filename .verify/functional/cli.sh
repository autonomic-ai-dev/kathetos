#!/bin/sh
set -u

BIN=$1
OUT=$2
pass=0
fails=0

mkdir -p "$OUT/repos"

run_case() {
  name=$1
  path=$2
  base=$3
  head=$4
  shift 4
  dir="$OUT/repos/$name"
  rm -rf "$dir"
  mkdir -p "$dir/$(dirname "$path")"
  (
    cd "$dir" || exit 1
    git init -q -b master
    git config user.name verify
    git config user.email verify@example.invalid
    printf '%b' "$base" > "$path"
    git add -A && git commit -qm base
    printf '%b' "$head" > "$path"
    git add -A && git commit -qm head
    "$BIN" check --diff "${RANGE:-HEAD~1...HEAD}" "$@" > "$OUT/$name.txt" 2>&1
    echo $? > "$OUT/$name.exit"
  )
}

record() {
  if [ "$1" = ok ]; then
    pass=$((pass + 1))
    echo "ok   $2"
  else
    fails=$((fails + 1))
    echo "FAIL $2"
  fi
}

check_exit() {
  got=$(cat "$OUT/$1.exit")
  if [ "$got" = "$2" ]; then record ok "$1 exit $2"; else record fail "$1 exit want $2 got $got"; fi
}

check_has() {
  if grep -qF -- "$2" "$OUT/$1.txt"; then record ok "$1 has '$2'"; else record fail "$1 missing '$2'"; fi
}

check_lacks() {
  if grep -qF -- "$2" "$OUT/$1.txt"; then record fail "$1 unexpectedly has '$2'"; else record ok "$1 lacks '$2'"; fi
}

run_case bare-skip pkg/x_test.go \
  'package x\n\nfunc TestA(t *testing.T) {\n\tcheck()\n}\n' \
  'package x\n\nfunc TestA(t *testing.T) {\n\tt.Skip()\n}\n'
check_exit bare-skip 1
check_has bare-skip 'violation: t.Skip with no reason'
check_has bare-skip '[provisional]'

run_case removed-test src/lib.rs \
  '#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n#[test]\nfn hermetic() {\n    assert_eq!(add(2, 2), 4);\n}\n' \
  '#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n'
check_exit removed-test 0
check_lacks removed-test 'violation:'
check_has removed-test 'test removed, no test added in this file'

run_case ignore-with-reason src/lib.rs \
  '#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n' \
  '#[test]\n#[ignore = "needs docker"]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n'
check_exit ignore-with-reason 0
check_lacks ignore-with-reason 'violation:'
check_has ignore-with-reason 'test ignored (reason given)'

run_case bare-ignore-on-existing-test src/lib.rs \
  '#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n' \
  '#[test]\n#[ignore]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n'
check_exit bare-ignore-on-existing-test 1
check_has bare-ignore-on-existing-test 'violation: #[ignore] with no reason'

run_case emptied-body pkg/x_test.go \
  'package x\n\nfunc TestA(t *testing.T) {\n\tcheck()\n}\n' \
  'package x\n\nfunc TestA(t *testing.T) {\n}\n'
check_exit emptied-body 1
check_has emptied-body 'violation: test body emptied'

run_case stub-test pkg/x_test.go \
  'package x\n' \
  'package x\n\nfunc TestA(t *testing.T) {\n}\n'
check_exit stub-test 0
check_lacks stub-test 'violation:'

run_case clean-change src/main.rs \
  'fn main() {\n    run();\n}\n' \
  'fn main() {\n    run();\n    done();\n}\n'
check_exit clean-change 0
check_lacks clean-change 'violation:'

RANGE=HEAD...HEAD
run_case empty-range src/main.rs \
  'fn main() {\n}\n' \
  'fn main() {\n    run();\n}\n'
check_exit empty-range 2
check_has empty-range 'empty diff, nothing to check'

RANGE=nope...HEAD
run_case unknown-revision src/main.rs \
  'fn main() {\n}\n' \
  'fn main() {\n    run();\n}\n'
check_exit unknown-revision 2
check_has unknown-revision 'unknown revision `nope`'
check_lacks unknown-revision 'deepen'
RANGE=

run_case json-basis pkg/x_test.go \
  'package x\n\nfunc TestA(t *testing.T) {\n\tcheck()\n}\n' \
  'package x\n\nfunc TestA(t *testing.T) {\n\tt.Skip()\n}\n' --json
check_exit json-basis 1
check_has json-basis '"basis":"provisional"'

echo "functional: $pass passed, $fails failed"
[ "$fails" -eq 0 ]
