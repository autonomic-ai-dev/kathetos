//! Deterministic rules.
//!
//! Three rules may block: a bare skip or ignore, a constant-true assertion, and an
//! emptied test body. Each is a syntax check on the diff. Admission status lives in
//! docs/rule-admission.md; every other finding in this file is a fork item.

use crate::cards::{self, Infra};
use crate::diff::{Change, FileDiff, Kind, Status};

pub const BASIS: &str = "provisional";

pub struct Violation {
    pub file: String,
    pub line: u32,
    pub rule: &'static str,
}

pub struct Fork {
    pub file: String,
    pub count: usize,
    pub line: u32,
    pub title: String,
    pub native: String,
    pub cost: String,
    pub ask: String,
    pub weight: u8,
}

fn marker_rust(t: &str) -> bool {
    let t = t.trim();
    t.starts_with("#[test]") || t.starts_with("#[tokio::test") || t.starts_with("#[rstest]")
}

fn marker_go(t: &str) -> bool {
    t.trim_start().starts_with("func Test")
}

fn is_marker(path: &str, t: &str) -> bool {
    if path.ends_with(".go") {
        marker_go(t)
    } else {
        marker_rust(t)
    }
}

pub fn any_marker(path: &str, changes: &[Change]) -> bool {
    changes.iter().any(|c| is_marker(path, &c.text))
}

pub fn added_marker(path: &str, changes: &[Change]) -> bool {
    changes
        .iter()
        .any(|c| c.kind == Kind::Add && is_marker(path, &c.text))
}

pub fn removed_marker(path: &str, changes: &[Change]) -> bool {
    changes
        .iter()
        .any(|c| c.kind == Kind::Del && is_marker(path, &c.text))
}

fn assertion(t: &str) -> bool {
    let t = t.trim();
    t.contains("assert")
        || t.contains("t.Error")
        || t.contains("t.Fatal")
        || t.contains("require.")
        || t.contains("if got != want")
}

/// True when a test touched the outside world. Such a test is deleted because it
/// cannot run in CI, which is a different claim from "coverage was bought back".
fn external(name: &str, body: Option<&str>) -> bool {
    let mut hay = name.to_string();
    if let Some(b) = body {
        hay.push_str(b);
    }
    [
        "os.Getenv",
        "exec.Command",
        "http.Get",
        "http.Post",
        "net.Dial",
        "DialContext",
        "Live",
        "Integration",
        "requires docker",
        "PROJECT_X_HOME",
    ]
    .iter()
    .any(|m| hay.contains(m))
}

fn mk(
    file: &FileDiff,
    line: u32,
    title: String,
    native: &str,
    cost: &str,
    ask: &str,
    weight: u8,
) -> Fork {
    Fork {
        file: file.path.clone(),
        line,
        count: 1,
        title,
        native: native.to_string(),
        cost: cost.to_string(),
        ask: ask.to_string(),
        weight,
    }
}

pub fn is_test_file(file: &FileDiff) -> bool {
    file.path.ends_with("_test.go")
        || file.path.ends_with("_test.rs")
        || file.path.starts_with("tests/")
        || file.path.contains("/tests/")
        || any_marker(&file.path, &file.changes)
}

pub fn tests(
    file: &FileDiff,
    new_content: Option<&str>,
    old_content: Option<&str>,
) -> (Vec<Violation>, Vec<Fork>) {
    let mut vs = Vec::new();
    let mut forks = Vec::new();

    let added: Vec<&Change> = file
        .added()
        .filter(|c| is_marker(&file.path, &c.text))
        .collect();
    let removed: Vec<&Change> = file
        .removed()
        .filter(|c| is_marker(&file.path, &c.text))
        .collect();

    // Rule 1 — a hermetic test deleted with no replacement in the same file.
    if !removed.is_empty() && added.is_empty() {
        let touched_outside = removed.iter().any(|c| {
            let body = old_content.and_then(|x| body_text(x, c.old_no));
            external(&c.text, body.as_deref())
        });
        if touched_outside {
            forks.push(mk(
                file,
                removed[0].old_no,
                "live/integration test removed".to_string(),
                "keep it behind a build tag (//go:build integration)",
                "the suite no longer proves the real path still works",
                "was this deleted because it cannot run, or because it fails?",
                4,
            ));
        } else {
            forks.push(mk(
                file,
                removed[0].old_no,
                "test removed, no test added in this file".to_string(),
                "move the test to the file that now covers this behaviour",
                "the behaviour it guarded may have no test anywhere",
                "which test now covers what this one checked?",
                3,
            ));
        }
    }

    for c in file.added() {
        let t = c.text.trim();

        let is_ignore = t.starts_with("#[ignore");
        let skip_rest = t.find("t.Skip(").map(|pos| &t[pos + 7..]);
        if is_ignore || skip_rest.is_some() {
            let bare = match skip_rest {
                Some(rest) if !is_ignore => rest.trim_start().starts_with(')'),
                _ => !t.contains('='),
            };
            let documented = !bare
                || trailing_reason(&c.text)
                || new_content.is_some_and(|src| reason_above(src, c.new_no, !is_ignore));
            if bare && !documented {
                vs.push(Violation {
                    file: file.path.clone(),
                    line: c.new_no,
                    rule: if is_ignore {
                        "#[ignore] with no reason"
                    } else {
                        "t.Skip with no reason"
                    },
                });
            } else {
                let (kind, native, cost, ask) = if is_ignore {
                    (
                        "ignored",
                        "gate it behind a feature/CFG flag",
                        "the reason is buried in an attribute nobody reads",
                        "is this ignore temporary?",
                    )
                } else {
                    (
                        "skipped",
                        "skip only when the dependency is absent, and log it",
                        "a skip that outlives its cause is invisible coverage loss",
                        "what has to be true for this test to run again?",
                    )
                };
                let title = if bare {
                    format!("test {kind} (reason in comment)")
                } else {
                    format!("test {kind} (reason given)")
                };
                forks.push(mk(file, c.new_no, title, native, cost, ask, 3));
            }
        }

        // Rule 4 — a constant-true assertion.
        if t.contains("assert!(true)") {
            vs.push(Violation {
                file: file.path.clone(),
                line: c.new_no,
                rule: "constant-true assertion",
            });
        }
    }

    // Rule 3 — a test body emptied.
    if let (Some(new), Some(old)) = (new_content, old_content) {
        for (i, line) in new.lines().enumerate() {
            if !is_marker(&file.path, line) {
                continue;
            }
            let at = i as u32 + 1;
            let Some(name) = test_name(new, at) else {
                continue;
            };
            let Some(now) = body_text(new, at) else {
                continue;
            };
            if !now.trim().is_empty() {
                continue;
            }
            if named_body(old, &name).is_some_and(|b| !b.trim().is_empty()) {
                vs.push(Violation {
                    file: file.path.clone(),
                    line: at,
                    rule: "test body emptied",
                });
            }
        }
    }

    // Assertion accounting is a fork, never a block. A count can move in either
    // direction without the test getting weaker.
    let rem = file.removed().filter(|c| assertion(&c.text)).count();
    let add = file.added().filter(|c| assertion(&c.text)).count();
    // A count drop caused by a deleted test is already reported above; saying it
    // twice is noise.
    if is_test_file(file) && rem > add && vs.is_empty() {
        forks.push(mk(
            file,
            file.removed()
                .find(|c| assertion(&c.text))
                .map(|c| c.old_no)
                .unwrap_or(0),
            format!("assertions {rem} -> {add}"),
            "move the assertion, do not delete it",
            "the test now passes for any behaviour in that path",
            "which behaviour lost its assertion?",
            3,
        ));
    }

    (vs, forks)
}

pub fn contents_needed(file: &FileDiff) -> (bool, bool) {
    let marked = file.status == Status::Modified && is_test_file(file);
    let new = marked || added_marker(&file.path, &file.changes) || adds_skip(file);
    let old = !file.old_path.is_empty() && (marked || removed_marker(&file.path, &file.changes));
    (new, old)
}

fn adds_skip(file: &FileDiff) -> bool {
    file.added().any(|c| {
        let t = c.text.trim();
        t.starts_with("#[ignore") || t.contains("t.Skip(")
    })
}

/// A comment is a reason only when it states a cause or a condition. A comment that
/// only describes what the test checks ("Verifies that…") is not a reason. Tuned on
/// the labeled findings in docs/labels; a lexical check, so it can miss a reason
/// phrased differently, and that miss is a fork, never a block.
fn states_reason(comment: &str) -> bool {
    const CUES: &[&str] = &[
        "because",
        "need",
        "requires",
        "flaky",
        "fail",
        "fixme",
        "todo",
        "issue",
        "unavailable",
        "until",
        "not yet",
        "doesn't",
        "does not",
        "cannot",
        "can't",
        "subprocess",
        "re-enable",
        "ignored while",
        "for now",
        "temporar",
        "only",
    ];
    let c = comment.to_lowercase();
    CUES.iter().any(|cue| c.contains(cue))
}

fn trailing_reason(text: &str) -> bool {
    text.find("//")
        .is_some_and(|p| p > 0 && states_reason(&text[p..]))
}

fn is_commented_attribute(comment: &str) -> bool {
    comment.trim_start_matches('/').trim_start().starts_with("#[")
}

fn comment_block_above(lines: &[&str], at: usize) -> bool {
    lines[..at]
        .iter()
        .rev()
        .map(|l| l.trim())
        .take_while(|t| t.starts_with("//") || t.starts_with("#["))
        .any(|t| t.starts_with("//") && !is_commented_attribute(t) && states_reason(t))
}

fn reason_above(src: &str, line: u32, enclosing_fn: bool) -> bool {
    let lines: Vec<&str> = src.lines().collect();
    let at = (line as usize).saturating_sub(1).min(lines.len());
    if comment_block_above(&lines, at) {
        return true;
    }
    if enclosing_fn {
        if let Some(decl) = (0..at).rev().find(|&k| lines[k].starts_with("func ")) {
            return comment_block_above(&lines, decl);
        }
    }
    false
}

fn fn_name(line: &str) -> Option<&str> {
    let t = line.trim_start();
    let t = t.strip_prefix("pub ").unwrap_or(t);
    let rest = t.strip_prefix("func ").or_else(|| t.strip_prefix("fn "))?;
    let rest = if rest.starts_with('(') {
        rest.split_once(") ")?.1
    } else {
        rest
    };
    let end = rest.find(|c: char| c == '(' || c == '<' || c.is_whitespace())?;
    let name = &rest[..end];
    (!name.is_empty()).then_some(name)
}

fn test_name(content: &str, marker_line: u32) -> Option<String> {
    content
        .lines()
        .skip(marker_line.saturating_sub(1) as usize)
        .take(3)
        .find_map(fn_name)
        .map(str::to_string)
}

fn named_body(content: &str, name: &str) -> Option<String> {
    let line = content.lines().position(|l| fn_name(l) == Some(name))? as u32 + 1;
    body_text(content, line)
}

/// The brace body that follows (or starts on) `start_line`, comments stripped.
fn body_text(content: &str, start_line: u32) -> Option<String> {
    let mut depth: i32 = 0;
    let mut opened = false;
    let mut body = String::new();

    for line in content.lines().skip(start_line.saturating_sub(1) as usize) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => {
                    depth -= 1;
                    if opened && depth == 0 {
                        return Some(strip_comments(&body));
                    }
                }
                _ if opened && depth > 0 => body.push(ch),
                _ => {}
            }
        }
        if opened && depth > 0 {
            body.push('\n');
        }
    }
    None
}

fn strip_comments(body: &str) -> String {
    body.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Diff-local systems hygiene.
///
/// One pattern only. Text-level error-masking detection (`.unwrap()`, `_ = fn()`)
/// was removed after a 50-commit sweep: precision was 0/11, because whether an
/// ignored error is actionable depends on what the callee does, not on how the
/// line is spelled. That judgement belongs in the skill card, not in a grep.
pub fn hygiene(file: &FileDiff) -> Vec<Fork> {
    let mut out = Vec::new();

    if file.path.ends_with(".go") {
        let adds: Vec<&Change> = file.added().collect();
        let has_timeout = adds.iter().any(|c| c.text.contains("Timeout:"));
        for c in adds {
            let t = c.text.as_str();
            if (t.contains("http.Client{") || t.contains("&http.Client{"))
                && !t.contains("Timeout:")
                && !has_timeout
            {
                out.push(mk(
                    file,
                    c.new_no,
                    "http.Client without a Timeout".to_string(),
                    "Client{Timeout: 10 * time.Second} + ctx deadline",
                    "a wedged upstream holds the goroutine until the kernel gives up",
                    "is this client for a streaming body?",
                    3,
                ));
            }

        }
    }

    out
}

/// How many added lines in the diff reference a crate by name. Whole-token
/// matching only: `age` must not match `message`.
pub fn uses_of(dep: &str, files: &[FileDiff]) -> usize {
    let snake = dep.replace('-', "_");
    files
        .iter()
        .filter(|f| !f.path.ends_with("Cargo.toml") && !f.path.ends_with("Cargo.lock"))
        .flat_map(|f| f.added())
        .filter(|c| mentions(&c.text, dep) || mentions(&c.text, &snake))
        .count()
}

fn mentions(text: &str, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let bytes = text.as_bytes();
    let mut from = 0usize;
    while let Some(i) = text[from..].find(name) {
        let start = from + i;
        let end = start + name.len();
        let before_ok = start == 0 || !ident(bytes[start - 1] as char);
        let after_ok = end >= bytes.len() || !ident(bytes[end] as char);
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

fn ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// A new file that declares a runtime process, an image, or a workload. Only
/// files the diff adds: editing an existing compose file is a change to a
/// decision already made, not a new one.
pub fn infra(file: &FileDiff) -> Vec<Fork> {
    if file.status != Status::Added {
        return Vec::new();
    }
    let Some(kind) = infra_kind(file) else {
        return Vec::new();
    };
    let card = cards::infra(kind);
    let (title, weight) = match kind {
        Infra::Compose => ("docker-compose file added", 4),
        Infra::Dockerfile => ("Dockerfile added", 2),
        Infra::KubeWorkload => ("Kubernetes workload manifest added", 4),
        Infra::Helm => ("Helm chart added", 2),
    };
    vec![Fork {
        file: file.path.clone(),
        line: 1,
        count: 1,
        title: title.to_string(),
        native: card.native.to_string(),
        cost: card.cost.to_string(),
        ask: card.ask.to_string(),
        weight,
    }]
}

fn infra_kind(file: &FileDiff) -> Option<Infra> {
    let name = file.path.rsplit('/').next().unwrap_or(&file.path);
    if matches!(
        name,
        "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml"
    ) {
        return Some(Infra::Compose);
    }
    if name == "Dockerfile" || name.starts_with("Dockerfile.") || name.ends_with(".Dockerfile") {
        return Some(Infra::Dockerfile);
    }
    if name == "Chart.yaml" {
        return Some(Infra::Helm);
    }
    if (name.ends_with(".yaml") || name.ends_with(".yml")) && is_kube_workload(file) {
        return Some(Infra::KubeWorkload);
    }
    None
}

/// Top-level keys only: an unindented `apiVersion:` and `kind:` pair naming a
/// workload controller. A nested `kind:` belongs to some other object.
fn is_kube_workload(file: &FileDiff) -> bool {
    let has_api = file.added().any(|c| c.text.starts_with("apiVersion:"));
    let has_workload = file.added().any(|c| {
        c.text
            .strip_prefix("kind:")
            .map(|k| matches!(k.trim(), "Deployment" | "StatefulSet" | "DaemonSet"))
            .unwrap_or(false)
    });
    has_api && has_workload
}

#[cfg(test)]
mod tests {
    use super::*;

    fn added(path: &str, status: Status, lines: &[&str]) -> FileDiff {
        FileDiff {
            path: path.to_string(),
            old_path: path.to_string(),
            status,
            changes: lines
                .iter()
                .enumerate()
                .map(|(i, t)| Change {
                    kind: Kind::Add,
                    text: t.to_string(),
                    new_no: i as u32 + 1,
                    old_no: 0,
                })
                .collect(),
        }
    }

    fn titles(f: &FileDiff) -> Vec<String> {
        infra(f).into_iter().map(|x| x.title).collect()
    }

    const DEPLOY: &[&str] = &["apiVersion: apps/v1", "kind: Deployment", "metadata:", "  name: api"];

    #[test]
    fn new_compose_file_fires() {
        let f = added("docker-compose.yml", Status::Added, &["services:", "  db:"]);
        assert_eq!(titles(&f), ["docker-compose file added"]);
    }

    #[test]
    fn compose_v2_name_fires_in_subdirectory() {
        let f = added("deploy/compose.yaml", Status::Added, &["services:"]);
        assert_eq!(titles(&f), ["docker-compose file added"]);
    }

    #[test]
    fn existing_compose_file_is_silent() {
        let f = added("docker-compose.yml", Status::Modified, &["  cache:"]);
        assert!(titles(&f).is_empty());
    }

    #[test]
    fn dockerfile_variants_fire() {
        for path in ["Dockerfile", "services/api/Dockerfile", "Dockerfile.worker", "api.Dockerfile"] {
            let f = added(path, Status::Added, &["FROM scratch"]);
            assert_eq!(titles(&f), ["Dockerfile added"], "{path}");
        }
    }

    #[test]
    fn helm_chart_fires() {
        let f = added("chart/Chart.yaml", Status::Added, &["name: api", "version: 0.1.0"]);
        assert_eq!(titles(&f), ["Helm chart added"]);
    }

    #[test]
    fn deployment_manifest_fires() {
        let f = added("k8s/api.yaml", Status::Added, DEPLOY);
        assert_eq!(titles(&f), ["Kubernetes workload manifest added"]);
    }

    #[test]
    fn statefulset_manifest_fires_in_yml() {
        let f = added("k8s/db.yml", Status::Added, &["apiVersion: apps/v1", "kind: StatefulSet"]);
        assert_eq!(titles(&f), ["Kubernetes workload manifest added"]);
    }

    #[test]
    fn service_only_manifest_is_silent() {
        let f = added("k8s/svc.yaml", Status::Added, &["apiVersion: v1", "kind: Service"]);
        assert!(titles(&f).is_empty());
    }

    #[test]
    fn nested_kind_is_not_a_workload() {
        let f = added(
            "k8s/cfg.yaml",
            Status::Added,
            &["apiVersion: v1", "kind: ConfigMap", "spec:", "  kind: Deployment"],
        );
        assert!(titles(&f).is_empty());
    }

    #[test]
    fn ci_workflow_yaml_is_silent() {
        let f = added(".github/workflows/ci.yml", Status::Added, &["on: push", "jobs:"]);
        assert!(titles(&f).is_empty());
    }

    #[test]
    fn source_file_is_silent() {
        let f = added("src/main.rs", Status::Added, DEPLOY);
        assert!(titles(&f).is_empty());
    }

    fn go_file(lines: &[&str]) -> FileDiff {
        added("pkg/x_test.go", Status::Modified, lines)
    }

    fn rule_names(vs: &[Violation]) -> Vec<&'static str> {
        vs.iter().map(|v| v.rule).collect()
    }

    fn removed(path: &str, lines: &[&str]) -> FileDiff {
        FileDiff {
            path: path.to_string(),
            old_path: path.to_string(),
            status: Status::Modified,
            changes: lines
                .iter()
                .enumerate()
                .map(|(i, t)| Change {
                    kind: Kind::Del,
                    text: t.to_string(),
                    new_no: 0,
                    old_no: i as u32 + 1,
                })
                .collect(),
        }
    }

    #[test]
    fn seeded_constant_true_assertion_blocks() {
        let f = added("src/lib.rs", Status::Modified, &["    assert!(true);"]);
        let (vs, _) = tests(&f, None, None);
        assert_eq!(rule_names(&vs), ["constant-true assertion"]);
    }

    #[test]
    fn seeded_bare_ignore_blocks() {
        let f = added("src/lib.rs", Status::Modified, &["#[ignore]"]);
        let (vs, _) = tests(&f, None, None);
        assert_eq!(rule_names(&vs), ["#[ignore] with no reason"]);
    }

    #[test]
    fn seeded_ignore_with_reason_is_a_fork() {
        let f = added("src/lib.rs", Status::Modified, &["#[ignore = \"needs docker\"]"]);
        let (vs, forks) = tests(&f, None, None);
        assert!(vs.is_empty());
        assert!(forks.iter().any(|x| x.title == "test ignored (reason given)"));
    }

    #[test]
    fn seeded_removed_test_is_a_fork_not_a_block() {
        let f = removed("src/lib.rs", &["#[test]", "fn adds() {", "    assert_eq!(add(1, 2), 3);", "}"]);
        let (vs, forks) = tests(&f, None, None);
        assert!(vs.is_empty());
        assert!(forks
            .iter()
            .any(|x| x.title == "test removed, no test added in this file"));
    }

    #[test]
    fn clean_non_test_change_is_silent() {
        let f = added("src/lib.rs", Status::Modified, &["    let x = add(1, 2);"]);
        let (vs, forks) = tests(&f, None, None);
        assert!(vs.is_empty());
        assert!(forks.is_empty());
    }

    #[test]
    fn skip_with_a_variable_reason_is_not_a_violation() {
        let f = go_file(&["\tt.Skip(msg)"]);
        let (vs, forks) = tests(&f, None, None);
        assert!(vs.is_empty());
        assert!(forks.iter().any(|x| x.title == "test skipped (reason given)"));
    }

    #[test]
    fn bare_skip_is_still_a_violation() {
        let f = go_file(&["\tt.Skip()"]);
        let (vs, _) = tests(&f, None, None);
        assert_eq!(rule_names(&vs), ["t.Skip with no reason"]);
    }

    const BASE_WITH_BODY: &str = "package x\n\nfunc TestA(t *testing.T) {\n\tcheck()\n}\n";

    #[test]
    fn test_body_deleted_from_existing_test_is_a_violation() {
        let new = "package x\n\nfunc TestA(t *testing.T) {\n}\n";
        let f = go_file(&["-\tcheck()"]);
        let (vs, _) = tests(&f, Some(new), Some(BASE_WITH_BODY));
        assert_eq!(rule_names(&vs), ["test body emptied"]);
        assert_eq!(vs[0].line, 3);
    }

    #[test]
    fn comment_only_body_counts_as_emptied() {
        let new = "package x\n\nfunc TestA(t *testing.T) {\n\t// todo\n}\n";
        let f = go_file(&["-\tcheck()"]);
        let (vs, _) = tests(&f, Some(new), Some(BASE_WITH_BODY));
        assert_eq!(rule_names(&vs), ["test body emptied"]);
    }

    #[test]
    fn new_empty_test_is_a_stub_not_an_emptying() {
        let new = "package x\n\nfunc TestA(t *testing.T) {\n}\n";
        let f = go_file(&["+func TestA(t *testing.T) {", "+}"]);
        let (vs, _) = tests(&f, Some(new), None);
        assert!(vs.is_empty());
    }

    #[test]
    fn test_that_was_already_empty_is_not_flagged() {
        let new = "package x\n\nfunc TestA(t *testing.T) {\n}\n";
        let old = "package x\n\nfunc TestA(t *testing.T) {\n}\n";
        let f = go_file(&["+\tcheck()"]);
        let (vs, _) = tests(&f, Some(new), Some(old));
        assert!(vs.is_empty());
    }

    #[test]
    fn renamed_test_is_not_an_emptying() {
        let new = "package x\n\nfunc TestRenamed(t *testing.T) {\n}\n";
        let f = go_file(&["-func TestA(t *testing.T) {", "+func TestRenamed(t *testing.T) {"]);
        let (vs, _) = tests(&f, Some(new), Some(BASE_WITH_BODY));
        assert!(vs.is_empty());
    }

    #[test]
    fn rust_test_body_deleted_is_a_violation() {
        let old = "#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n";
        let new = "#[test]\nfn adds() {\n}\n";
        let f = added("src/lib.rs", Status::Modified, &["-    assert_eq!(add(1, 2), 3);"]);
        let (vs, _) = tests(&f, Some(new), Some(old));
        assert_eq!(rule_names(&vs), ["test body emptied"]);
    }

    #[test]
    fn contents_are_loaded_both_sides_for_modified_test_files() {
        let f = go_file(&["func TestA(t *testing.T) {"]);
        assert_eq!(contents_needed(&f), (true, true));
    }

    fn added_at(path: &str, text: &str, new_no: u32) -> FileDiff {
        FileDiff {
            path: path.to_string(),
            old_path: path.to_string(),
            status: Status::Modified,
            changes: vec![Change {
                kind: Kind::Add,
                text: text.to_string(),
                new_no,
                old_no: 0,
            }],
        }
    }

    #[test]
    fn ignore_with_comment_above_is_a_fork() {
        let src = "// Needs docker.\n#[test]\n#[ignore]\nfn adds() {}\n";
        let f = added_at("src/lib.rs", "#[ignore]", 3);
        let (vs, forks) = tests(&f, Some(src), None);
        assert!(vs.is_empty());
        assert!(forks.iter().any(|x| x.title == "test ignored (reason in comment)"));
    }

    #[test]
    fn ignore_without_comment_still_blocks() {
        let src = "#[test]\n#[ignore]\nfn adds() {}\n";
        let f = added_at("src/lib.rs", "#[ignore]", 2);
        let (vs, _) = tests(&f, Some(src), None);
        assert_eq!(rule_names(&vs), ["#[ignore] with no reason"]);
    }

    #[test]
    fn descriptive_doc_comment_is_not_a_reason() {
        // Says what the test checks, not why it is ignored: still a violation.
        let src = "/// Verifies that the stream emits events in order.\n#[test]\n#[ignore]\nfn adds() {}\n";
        let f = added_at("src/lib.rs", "#[ignore]", 3);
        let (vs, _) = tests(&f, Some(src), None);
        assert_eq!(rule_names(&vs), ["#[ignore] with no reason"]);
    }

    #[test]
    fn doc_comment_stating_a_cause_is_a_reason() {
        let src = "/// Ignored because it needs themes.bin up to date.\n#[test]\n#[ignore]\nfn adds() {}\n";
        let f = added_at("src/lib.rs", "#[ignore]", 3);
        let (vs, forks) = tests(&f, Some(src), None);
        assert!(vs.is_empty());
        assert!(forks.iter().any(|x| x.title == "test ignored (reason in comment)"));
    }

    #[test]
    fn commented_out_attribute_is_not_a_reason() {
        let src = "// #[cfg_attr(not(windows), ignore)]\n#[ignore]\nfn adds() {}\n";
        let f = added_at("src/lib.rs", "#[ignore]", 2);
        let (vs, _) = tests(&f, Some(src), None);
        assert_eq!(rule_names(&vs), ["#[ignore] with no reason"]);
    }

    #[test]
    fn trailing_comment_is_a_reason() {
        let f = added_at("src/lib.rs", "#[ignore] // flaky under load", 2);
        let (vs, forks) = tests(&f, None, None);
        assert!(vs.is_empty());
        assert!(forks.iter().any(|x| x.title == "test ignored (reason in comment)"));
    }

    #[test]
    fn go_skip_under_documented_function_is_a_fork() {
        let src = "// Needs the dart-sass binary.\nfunc TestX(t *testing.T) {\n\tif !ok() {\n\t\tt.Skip()\n\t}\n}\n";
        let f = added_at("pkg/x_test.go", "\t\tt.Skip()", 4);
        let (vs, forks) = tests(&f, Some(src), None);
        assert!(vs.is_empty());
        assert!(forks.iter().any(|x| x.title == "test skipped (reason in comment)"));
    }

    #[test]
    fn go_skip_guarded_only_still_blocks() {
        let src = "func TestX(t *testing.T) {\n\tif !ok() {\n\t\tt.Skip()\n\t}\n}\n";
        let f = added_at("pkg/x_test.go", "\t\tt.Skip()", 3);
        let (vs, _) = tests(&f, Some(src), None);
        assert_eq!(rule_names(&vs), ["t.Skip with no reason"]);
    }

    #[test]
    fn new_test_file_has_no_base_side() {
        let f = added("pkg/x_test.go", Status::Added, &["func TestA(t *testing.T) {"]);
        assert_eq!(contents_needed(&f), (true, false));
    }
}
