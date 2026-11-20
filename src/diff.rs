//! git access + unified-diff parsing. No external crates.

use std::process::Command;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Added,
    Deleted,
    Modified,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Add,
    Del,
}

#[derive(Clone, Debug)]
pub struct Change {
    pub kind: Kind,
    pub text: String,
    pub new_no: u32,
    pub old_no: u32,
}

#[derive(Clone, Debug)]
pub struct FileDiff {
    pub path: String,
    pub old_path: String,
    pub status: Status,
    pub changes: Vec<Change>,
}

impl FileDiff {
    pub fn added(&self) -> impl Iterator<Item = &Change> {
        self.changes.iter().filter(|c| c.kind == Kind::Add)
    }
    pub fn removed(&self) -> impl Iterator<Item = &Change> {
        self.changes.iter().filter(|c| c.kind == Kind::Del)
    }
}

pub struct Diff {
    pub files: Vec<FileDiff>,
    pub base: String,
    pub head: String,
}

fn git(args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(err.trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Read many blobs in one subprocess. Specs are `rev:path`.
pub fn read_many(specs: &[String]) -> std::collections::HashMap<String, String> {
    use std::io::Write;
    let mut out = std::collections::HashMap::new();
    if specs.is_empty() {
        return out;
    }
    let child = Command::new("git")
        .args(["cat-file", "--batch"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();
    let Ok(mut child) = child else { return out };
    if let Some(stdin) = child.stdin.as_mut() {
        for s in specs {
            let _ = writeln!(stdin, "{s}");
        }
    }
    drop(child.stdin.take());
    let Ok(proc_out) = child.wait_with_output() else {
        return out;
    };

    let buf = proc_out.stdout;
    let mut i = 0usize;
    let mut cursor = 0usize;
    while i < buf.len() && cursor < specs.len() {
        let Some(nl) = buf[i..].iter().position(|b| *b == b'\n') else {
            break;
        };
        let header = String::from_utf8_lossy(&buf[i..i + nl]).to_string();
        i += nl + 1;
        if header.ends_with("missing") || header.ends_with("ambiguous") {
            cursor += 1;
            continue;
        }
        let size: usize = match header.rsplit(' ').next().and_then(|s| s.parse().ok()) {
            Some(n) => n,
            None => break,
        };
        if i + size > buf.len() {
            break;
        }
        let body = String::from_utf8_lossy(&buf[i..i + size]).to_string();
        out.insert(specs[cursor].clone(), body);
        cursor += 1;
        i += size + 1;
    }
    out
}

pub fn resolve_default_range() -> Option<String> {
    if git(&["rev-parse", "--verify", "origin/HEAD"]).is_ok() {
        return Some("origin/HEAD...HEAD".to_string());
    }
    for b in ["main", "master"] {
        if git(&["rev-parse", "--verify", b]).is_ok() {
            return Some(format!("{b}...HEAD"));
        }
    }
    if git(&["rev-parse", "--verify", "HEAD~1"]).is_ok() {
        return Some("HEAD~1...HEAD".to_string());
    }
    None
}

/// Split a rev range into (left, right) and find the merge base.
fn sides(range: &str) -> (String, String) {
    let sep = if range.contains("...") { "..." } else { ".." };
    match range.split_once(sep) {
        Some((l, r)) => (l.to_string(), r.to_string()),
        None => (range.to_string(), "HEAD".to_string()),
    }
}

pub fn load(range: &str) -> Result<Diff, String> {
    let (left, right) = sides(range);
    for side in [&left, &right] {
        if git(&["rev-parse", "--verify", "--quiet", &format!("{side}^{{commit}}")]).is_err() {
            let hint = if side.starts_with("origin/") {
                "\n  hint: a shallow clone may lack it: git fetch --deepen=50 origin"
            } else {
                ""
            };
            return Err(format!("unknown revision `{side}`{hint}"));
        }
    }
    let base = git(&["merge-base", &left, &right])
        .or_else(|_| git(&["rev-parse", &left]))
        .map_err(|_| format!("cannot resolve merge base for `{range}`"))?
        .trim()
        .to_string();

    let text = git(&[
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--unified=0",
        range,
    ])
    .map_err(|e| format!("{e}"))?;

    Ok(Diff {
        files: parse(&text),
        base,
        head: right,
    })
}

fn parse(text: &str) -> Vec<FileDiff> {
    let mut out: Vec<FileDiff> = Vec::new();
    let mut cur: Option<FileDiff> = None;
    let mut old_no: u32 = 0;
    let mut new_no: u32 = 0;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            if let Some(f) = cur.take() {
                out.push(f);
            }
            let (a, b) = split_git_paths(rest);
            cur = Some(FileDiff {
                path: b,
                old_path: a,
                status: Status::Modified,
                changes: Vec::new(),
            });
            continue;
        }

        let Some(f) = cur.as_mut() else { continue };

        if let Some(rest) = line.strip_prefix("--- ") {
            let p = unquote(rest);
            if p == "/dev/null" {
                f.status = Status::Added;
            } else {
                f.old_path = strip_prefix(&p, "a/");
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            let p = unquote(rest);
            if p == "/dev/null" {
                f.status = Status::Deleted;
            } else {
                f.path = strip_prefix(&p, "b/");
            }
            continue;
        }
        if line.starts_with("@@") {
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() >= 3 {
                old_no = hunk_start(parts[1]).unwrap_or(0);
                new_no = hunk_start(parts[2]).unwrap_or(0);
            }
            continue;
        }
        if let Some(t) = line.strip_prefix('+') {
            f.changes.push(Change {
                kind: Kind::Add,
                text: t.to_string(),
                new_no,
                old_no: 0,
            });
            new_no += 1;
        } else if let Some(t) = line.strip_prefix('-') {
            f.changes.push(Change {
                kind: Kind::Del,
                text: t.to_string(),
                old_no,
                new_no: 0,
            });
            old_no += 1;
        }
    }
    if let Some(f) = cur.take() {
        out.push(f);
    }
    out
}

fn hunk_start(tok: &str) -> Option<u32> {
    let t = tok.trim_start_matches(['-', '+']);
    t.split(',').next()?.parse().ok()
}

fn split_git_paths(rest: &str) -> (String, String) {
    let rest = unquote(rest);
    match rest.split_once(" b/") {
        Some((a, b)) => (strip_prefix(a, "a/"), b.to_string()),
        None => (String::new(), rest.to_string()),
    }
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].replace("\\\"", "\"")
    } else {
        s.to_string()
    }
}

fn strip_prefix(s: &str, p: &str) -> String {
    s.strip_prefix(p).unwrap_or(s).to_string()
}
