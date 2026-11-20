#!/bin/sh
set -u

BIN=$1
OUT=$2
mkdir -p "$OUT/repos"
cases=0
mismatch=0

build() {
  name=$1
  path=$2
  base=$3
  head=$4
  dir="$OUT/repos/$name"
  rm -rf "$dir"
  mkdir -p "$dir/$(dirname "$path")"
  (
    cd "$dir" || exit 1
    git init -q -b master
    git config user.name verify
    git config user.email verify@example.invalid
    if [ "$base" = "-" ]; then echo placeholder > README.md; else printf '%b' "$base" > "$path"; fi
    git add -A && git commit -qm base
    printf '%b' "$head" > "$path"
    git add -A && git commit -qm head
    "$BIN" check --diff HEAD~1...HEAD --json > "$OUT/$name.json" 2>"$OUT/$name.err"
    echo $? > "$OUT/$name.exit"
  )
}

summarise() {
  python3 -I - "$OUT/$1.json" <<'EOF'
import json, sys
d = json.load(open(sys.argv[1]))
print(";".join(sorted({v["rule"] for v in d["violations"]})) or "-")
print(";".join(f["title"] for f in d["forks"]) or "-")
EOF
}

case_() {
  name=$1
  path=$2
  base=$3
  head=$4
  want_v=$5
  want_f=$6
  cases=$((cases + 1))
  build "$name" "$path" "$base" "$head"
  got=$(summarise "$name")
  got_v=$(printf '%s\n' "$got" | sed -n 1p)
  got_f=$(printf '%s\n' "$got" | sed -n 2p)
  ok=1
  [ "$got_v" = "$want_v" ] || ok=0
  if [ "$want_f" != "-" ]; then
    case "$got_f" in *"$want_f"*) ;; *) ok=0 ;; esac
  fi
  if [ "$ok" -eq 1 ]; then
    echo "ok   $name  v=$got_v"
  else
    mismatch=$((mismatch + 1))
    echo "FAIL $name  want v=$want_v f=$want_f  got v=$got_v f=$got_f"
  fi
  printf '%s\t%s\t%s\t%s\t%s\n' "$name" "$want_v" "$want_f" "$got_v" "$got_f" >> "$OUT/cases.tsv"
}

rm -f "$OUT/cases.tsv"

GO_TEST='package a\n\nfunc TestA(t *testing.T) {\n\tcheck()\n}\n'
RS_TEST='#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n'

case_ bare-skip-go pkg/a_test.go "$GO_TEST" 'package a\n\nfunc TestA(t *testing.T) {\n\tt.Skip()\n}\n' "t.Skip with no reason" -
case_ reasoned-skip-go pkg/a_test.go "$GO_TEST" 'package a\n\nfunc TestA(t *testing.T) {\n\tt.Skip("needs db")\n}\n' - "test skipped (reason given)"
case_ helper-skip-var-go pkg/a_test.go "$GO_TEST" 'package a\n\nfunc TestA(t *testing.T) {\n\tt.Skip(msg)\n}\n' - "test skipped (reason given)"
case_ bare-ignore-rust src/lib.rs "$RS_TEST" '#[test]\n#[ignore]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n' "#[ignore] with no reason" -
case_ reasoned-ignore-rust src/lib.rs "$RS_TEST" '#[test]\n#[ignore = "needs docker"]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n' - "test ignored (reason given)"
case_ assert-true-rust src/lib.rs "$RS_TEST" '#[test]\nfn adds() {\n    assert!(true);\n}\n' "constant-true assertion" -
case_ emptied-go pkg/a_test.go "$GO_TEST" 'package a\n\nfunc TestA(t *testing.T) {\n}\n' "test body emptied" -
case_ emptied-tests-dir tests/api.rs '#[test]\nfn api() {\n    call();\n}\n' '#[test]\nfn api() {\n}\n' "test body emptied" -
case_ stub-go pkg/a_test.go 'package a\n' 'package a\n\nfunc TestA(t *testing.T) {\n}\n' - -
case_ removed-hermetic-rust src/lib.rs '#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n#[test]\nfn hermetic() {\n    assert_eq!(add(2, 2), 4);\n}\n' '#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n' - "test removed, no test added"
case_ removed-live-go pkg/live_test.go 'package a\n\nfunc TestLive(t *testing.T) {\n\tos.Getenv("KEY")\n}\n\nfunc TestB(t *testing.T) {\n\tcheck()\n}\n' 'package a\n\nfunc TestB(t *testing.T) {\n\tcheck()\n}\n' - "live/integration test removed"
case_ comment-only-emptied-go pkg/a_test.go "$GO_TEST" 'package a\n\nfunc TestA(t *testing.T) {\n\t// todo\n}\n' "test body emptied" -
case_ renamed-go-test pkg/a_test.go "$GO_TEST" 'package a\n\nfunc TestRenamed(t *testing.T) {\n\tcheck()\n}\n' - -
case_ tokio-no-refs-cargo Cargo.toml '[package]\nname = "demo"\nversion = "0.1.0"\n\n[dependencies]\n' '[package]\nname = "demo"\nversion = "0.1.0"\n\n[dependencies]\ntokio = "1"\n' - "tokio added"
case_ four-deps-cargo Cargo.toml '[package]\nname = "demo"\nversion = "0.1.0"\n\n[dependencies]\n' '[package]\nname = "demo"\nversion = "0.1.0"\n\n[dependencies]\nserde = "1"\nclap = "4"\nlog = "0.4"\nrand = "0.8"\n' - "direct deps for one change"
case_ http-no-timeout-go client.go '-' 'package a\n\nimport "net/http"\n\nvar c = &http.Client{}\n' - "http.Client without a Timeout"
case_ compose-added docker-compose.yml '-' 'services:\n  web:\n    image: nginx\n' - "docker-compose file added"
case_ dockerfile-added Dockerfile '-' 'FROM debian:12-slim\nCOPY api /usr/local/bin/api\n' - "Dockerfile added"
case_ clean-rust-rename src/lib.rs 'pub fn add(a: i32) -> i32 {\n    a\n}\n' 'pub fn add(value: i32) -> i32 {\n    value\n}\n' - -
case_ clean-go-timeout client.go 'package a\n' 'package a\n\nimport "net/http"\n\nvar c = &http.Client{Timeout: 5}\n' - -

python3 -I - "$OUT/cases.tsv" > "$OUT/metrics.txt" <<'EOF'
import sys
rows = [l.rstrip("\n").split("\t") for l in open(sys.argv[1])]
rules = ["t.Skip with no reason", "#[ignore] with no reason", "constant-true assertion", "test body emptied"]
for r in rules:
    tp = fp = fn = 0
    for _, want_v, _, got_v, _ in rows:
        w = r in want_v.split(";")
        g = r in got_v.split(";")
        tp += w and g
        fp += g and not w
        fn += w and not g
    prec = f"{tp / (tp + fp):.2f}" if tp + fp else "n/a"
    rec = f"{tp / (tp + fn):.2f}" if tp + fn else "n/a"
    print(f"{r}: tp={tp} fp={fp} fn={fn} precision={prec} recall={rec}")
want_f = [r for r in rows if r[2] != "-"]
hit_f = [r for r in want_f if r[4] != "-" and r[2] in r[4]]
noise = [r for r in rows if r[2] == "-" and r[4] != "-"]
print(f"forks: expected={len(want_f)} found={len(hit_f)} recall={len(hit_f) / len(want_f):.2f}")
print(f"clean cases with a fork (noise): {len(noise)} of {sum(1 for r in rows if r[2] == '-')}")
EOF
cat "$OUT/metrics.txt"
echo "corpus: $cases cases, $mismatch mismatched"
[ "$mismatch" -eq 0 ]
