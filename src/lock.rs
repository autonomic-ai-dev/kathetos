//! Lockfile and manifest accounting.
//!
//! Cargo.lock (v3/v4) carries the dependency graph, so transitive attribution is
//! exact. go.sum carries no edges at all, so Go attribution is *not* attempted —
//! a missing number costs less trust than a wrong one.

use std::collections::{HashMap, HashSet, VecDeque};

pub type Graph = HashMap<String, Vec<String>>;

pub fn parse_cargo_lock(text: &str) -> Graph {
    let mut graph: Graph = HashMap::new();
    let mut name = String::new();
    let mut deps: Vec<String> = Vec::new();
    let mut in_deps = false;
    let mut in_package = false;

    for raw in text.lines() {
        let line = raw.trim();
        if line == "[[package]]" {
            flush(&mut graph, &mut name, &mut deps);
            in_package = true;
            in_deps = false;
            continue;
        }
        if !in_package {
            continue;
        }
        if in_deps {
            if line == "]" {
                in_deps = false;
                continue;
            }
            let t = line.trim_matches(|c| c == '"' || c == ',');
            if let Some(first) = t.split(' ').next() {
                if !first.is_empty() {
                    deps.push(first.to_string());
                }
            }
            continue;
        }
        if let Some(v) = line.strip_prefix("name = \"") {
            name = v.trim_end_matches('"').to_string();
        } else if line.starts_with("dependencies = [") {
            in_deps = true;
            let tail = line.trim_start_matches("dependencies = [").trim();
            if tail == "]" {
                in_deps = false;
            }
        }
    }
    flush(&mut graph, &mut name, &mut deps);
    graph
}

fn flush(graph: &mut Graph, name: &mut String, deps: &mut Vec<String>) {
    if !name.is_empty() {
        let e = graph.entry(std::mem::take(name)).or_default();
        for d in deps.drain(..) {
            if !e.contains(&d) {
                e.push(d);
            }
        }
    } else {
        deps.clear();
    }
}

pub fn closure(graph: &Graph, root: &str) -> HashSet<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut q: VecDeque<String> = VecDeque::new();
    if let Some(d) = graph.get(root) {
        for x in d {
            q.push_back(x.clone());
        }
    }
    while let Some(n) = q.pop_front() {
        if !seen.insert(n.clone()) {
            continue;
        }
        if let Some(next) = graph.get(&n) {
            for x in next {
                if !seen.contains(x) {
                    q.push_back(x.clone());
                }
            }
        }
    }
    seen
}

pub struct CargoDelta {
    /// direct dependencies added in this diff, with location
    pub direct: Vec<(String, u32, String)>,
    /// every package newly present in Cargo.lock
    pub added: Vec<String>,
    /// packages pulled by the added direct deps that were already in the old lock
    pub already: usize,
    /// distinct packages the added direct deps pull that were not in the old lock
    pub fresh: usize,
    /// per added direct dep: (name, fresh packages it pulls, uses in diff)
    pub per_dep: Vec<(String, usize, usize)>,
}

pub fn cargo_delta(
    old_lock: Option<&str>,
    new_lock: &str,
    direct: &[(String, u32, String)],
    uses: &HashMap<String, usize>,
) -> CargoDelta {
    let new_graph = parse_cargo_lock(new_lock);
    let old_graph = old_lock.map(parse_cargo_lock).unwrap_or_default();
    let old_names: HashSet<String> = old_graph.keys().cloned().collect();

    let mut added: Vec<String> = new_graph
        .keys()
        .filter(|k| !old_names.is_empty() && !old_names.contains(*k))
        .cloned()
        .collect();
    added.sort();

    let mut union: HashSet<String> = HashSet::new();
    let mut already_union: HashSet<String> = HashSet::new();
    let mut per_dep = Vec::new();
    for (dep, _, _) in direct {
        let c = closure(&new_graph, dep);
        let fresh: Vec<String> = c.iter().filter(|n| !old_names.contains(*n)).cloned().collect();
        let have: Vec<String> = c.iter().filter(|n| old_names.contains(*n)).cloned().collect();
        for f in &fresh {
            union.insert(f.clone());
        }
        for h in &have {
            already_union.insert(h.clone());
        }
        per_dep.push((
            dep.clone(),
            fresh.len(),
            *uses.get(dep).unwrap_or(&0),
        ));
    }

    CargoDelta {
        direct: direct.to_vec(),
        added,
        already: already_union.len(),
        fresh: union.len(),
        per_dep,
    }
}

/// Direct dependency names added to a Cargo.toml, found by resolving each added
/// line's enclosing section in the new file content.
pub fn cargo_direct_added(
    path: &str,
    changes: &[crate::diff::Change],
    content: &str,
) -> Vec<(String, u32, String)> {
    use crate::diff::Kind;
    let mut out = Vec::new();
    for ch in changes.iter().filter(|c| c.kind == Kind::Add) {
        let Some(key) = dep_key(&ch.text) else { continue };
        let section = section_at(content, ch.new_no);
        if section.ends_with("dependencies") && !section.contains("dev-dependencies")
            && !section.contains("build-dependencies")
        {
            out.push((key, ch.new_no, path.to_string()));
        }
    }
    out
}

fn dep_key(line: &str) -> Option<String> {
    let t = line.trim();
    if t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    if k.is_empty() || k.contains(' ') || k.contains('"') {
        return None;
    }
    Some(k.to_string())
}

/// The `[section]` header governing 1-based line `n`.
pub fn section_at(content: &str, n: u32) -> String {
    let mut section = String::new();
    for (i, line) in content.lines().enumerate() {
        if i as u32 + 1 > n {
            break;
        }
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            section = t.trim_matches(['[', ']']).to_string();
        }
    }
    section
}

/// Go: direct requires added to go.mod, excluding `// indirect`.
pub fn go_direct_added(changes: &[crate::diff::Change]) -> Vec<String> {
    use crate::diff::Kind;
    changes
        .iter()
        .filter(|c| c.kind == Kind::Add)
        .filter(|c| !c.text.contains("// indirect"))
        .filter_map(|c| {
            let t = c.text.trim();
            let t = t.strip_prefix("require ").unwrap_or(t);
            let t = t.trim_start_matches('(').trim();
            let first = t.split_whitespace().next()?;
            if t.contains(char::is_whitespace) && first.contains('.') {
                Some(first.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// go.sum has no edges: count distinct modules, never attribute.
pub fn go_sum_modules(changes: &[crate::diff::Change]) -> usize {
    use crate::diff::Kind;
    let mut mods: HashSet<String> = HashSet::new();
    for c in changes.iter().filter(|c| c.kind == Kind::Add) {
        let t = c.text.trim();
        if let Some(first) = t.split_whitespace().next() {
            if first.contains('.') && !first.starts_with('#') {
                mods.insert(first.to_string());
            }
        }
    }
    mods.len()
}
