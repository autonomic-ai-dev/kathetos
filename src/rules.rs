//! Deterministic rules.
//!
//! Four rules may block, and each is narrowed until it survives a real history:
//! a skip *with a stated reason* is a decision, not a cheat, so it warns. A test
//! deleted for requiring the outside world is maintenance, not coverage loss, so
//! it warns. Everything else in this file is a fork item and never an exit code.

use crate::diff::{Change, FileDiff, Kind};

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

pub fn tests(
    file: &FileDiff,
    new_content: Option<&str>,
    old_content: Option<&str>,
    is_test: bool,
) -> (Vec<Violation>, Vec<Fork>) {
    let mut vs = Vec::new();
    let mut forks = Vec::new();
    if !is_test {
        return (vs, forks);
    }

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
            vs.push(Violation {
                file: file.path.clone(),
                line: removed[0].old_no,
                rule: "test removed with no replacement",
            });
        }
    }

    for c in file.added() {
        let t = c.text.trim();

        // Rule 2 — a skip added. Only a skip with no stated reason blocks: a
        // reason is a decision someone can review; silence is not.
        if t.starts_with("#[ignore") {
            if t.contains('=') {
                forks.push(mk(
                    file,
                    c.new_no,
                    "test ignored (reason given)".to_string(),
                    "gate it behind a feature/CFG flag",
                    "the reason is buried in an attribute nobody reads",
                    "is this ignore temporary?",
                    3,
                ));
            } else {
                vs.push(Violation {
                    file: file.path.clone(),
                    line: c.new_no,
                    rule: "#[ignore] with no reason",
                });
            }
        } else if let Some(pos) = t.find("t.Skip(") {
            let rest = &t[pos + 7..];
            let reasoned = rest.trim_start().starts_with('"') || rest.trim_start().starts_with('`');
            if reasoned {
                forks.push(mk(
                    file,
                    c.new_no,
                    "test skipped (reason given)".to_string(),
                    "skip only when the dependency is absent, and log it",
                    "a skip that outlives its cause is invisible coverage loss",
                    "what has to be true for this test to run again?",
                    3,
                ));
            } else {
                vs.push(Violation {
                    file: file.path.clone(),
                    line: c.new_no,
                    rule: "t.Skip with no reason",
                });
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
    if let Some(text) = new_content {
        for c in &added {
            if let Some(body) = body_text(text, c.new_no) {
                if body.trim().is_empty() {
                    vs.push(Violation {
                        file: file.path.clone(),
                        line: c.new_no,
                        rule: "test body emptied",
                    });
                }
            }
        }
    }

    // Assertion accounting is a fork, never a block. A count can move in either
    // direction without the test getting weaker.
    let rem = file.removed().filter(|c| assertion(&c.text)).count();
    let add = file.added().filter(|c| assertion(&c.text)).count();
    // A count drop caused by a deleted test is already reported above; saying it
    // twice is noise.
    if rem > add && vs.is_empty() {
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
