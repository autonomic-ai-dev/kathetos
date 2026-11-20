mod cards;
mod diff;
mod lock;
mod rules;

use std::collections::HashMap;
use std::fs;
use std::process::exit;
use std::time::Instant;

use rules::{Fork, Violation};

const USAGE: &str = "\
kathetos — the plumb line for coding agents

USAGE
  kathetos check [--diff <range>] [--json] [--verbose]

  --diff <range>   git rev range, e.g. origin/main...HEAD
                   default: origin/HEAD...HEAD, then main/master, then HEAD~1
  --json           machine-readable output
  --verbose        per-file list on stderr

EXIT
  0  nothing fired
  1  a hard rule fired (test integrity)
  2  usage error, or the base revision could not be resolved
";

struct Args {
    range: Option<String>,
    json: bool,
    verbose: bool,
}

fn main() {
    let mut args = Args {
        range: None,
        json: false,
        verbose: false,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.is_empty() || argv.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return;
    }
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "check" => {}
            "--diff" => {
                i += 1;
                match argv.get(i) {
                    Some(v) => args.range = Some(v.clone()),
                    None => die("--diff needs a revision range"),
                }
            }
            "--json" => args.json = true,
            "--verbose" | "-v" => args.verbose = true,
            other => die(&format!("unknown argument `{other}`")),
        }
        i += 1;
    }

    let started = Instant::now();
    let range = match args.range.clone().or_else(diff::resolve_default_range) {
        Some(r) => r,
        None => die("cannot resolve a base revision (no origin/HEAD, main, master or HEAD~1)"),
    };

    let d = match diff::load(&range) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("kathetos · {range}");
            eprintln!("  0 files · {e}");
            exit(2);
        }
    };

    if d.files.is_empty() {
        eprintln!("kathetos · {range}");
        eprintln!("  0 files · empty diff, nothing to check");
        exit(2);
    }

    if args.verbose {
        for f in &d.files {
            eprintln!("  {:<48} +{} -{}", f.path, f.added().count(), f.removed().count());
        }
    }

    let mut language = "none";
    if d.files.iter().any(|f| f.path.ends_with(".rs")) {
        language = "rust";
    }
    if d.files.iter().any(|f| f.path.ends_with(".go")) {
        language = if language == "rust" { "mixed" } else { "go" };
    }

    // ---- blobs: one subprocess for everything the rules need --------------
    let mut specs: Vec<String> = Vec::new();
    let want = |s: String, v: &mut Vec<String>| {
        if !v.contains(&s) {
            v.push(s);
        }
    };
    want(format!("{}:Cargo.lock", d.head), &mut specs);
    want(format!("{}:Cargo.lock", d.base), &mut specs);
    for f in &d.files {
        let (want_new, want_old) = rules::contents_needed(f);
        if f.path.ends_with("Cargo.toml") || want_new {
            want(format!("{}:{}", d.head, f.path), &mut specs);
        }
        if want_old {
            want(format!("{}:{}", d.base, f.old_path), &mut specs);
        }
    }
    let blobs = diff::read_many(&specs);

    let new_of = |path: &str| -> Option<String> {
        if d.head == "HEAD" || d.head.is_empty() {
            if let Ok(t) = fs::read_to_string(path) {
                return Some(t);
            }
        }
        blobs
            .get(&format!("{}:{}", d.head, path))
            .cloned()
            .or_else(|| fs::read_to_string(path).ok())
    };

    // ---- dependency accounting -------------------------------------------
    let mut direct: Vec<(String, u32, String)> = Vec::new();
    let mut go_direct: Vec<String> = Vec::new();
    let mut go_modules = 0usize;

    for f in &d.files {
        if f.path.ends_with("Cargo.toml") {
            if let Some(content) = new_of(&f.path) {
                direct.extend(lock::cargo_direct_added(&f.path, &f.changes, &content));
            }
        }
        if f.path == "go.mod" {
            go_direct = lock::go_direct_added(&f.changes);
        }
        if f.path == "go.sum" {
            go_modules = lock::go_sum_modules(&f.changes);
        }
    }

    let mut seen: Vec<String> = Vec::new();
    direct.retain(|(d, _, _)| {
        if seen.contains(d) {
            false
        } else {
            seen.push(d.clone());
            true
        }
    });

    let new_lock = new_of("Cargo.lock");
    let old_lock = blobs.get(&format!("{}:Cargo.lock", d.base)).cloned();

    let mut uses: HashMap<String, usize> = HashMap::new();
    for (dep, _, _) in &direct {
        uses.insert(dep.clone(), rules::uses_of(dep, &d.files));
    }

    let delta = new_lock.as_ref().map(|nl| {
        lock::cargo_delta(old_lock.as_deref(), nl, &direct, &uses)
    });

    // ---- rules ------------------------------------------------------------
    let mut forks: Vec<Fork> = Vec::new();
    let mut violations: Vec<Violation> = Vec::new();

    for f in &d.files {
        let (want_new, want_old) = rules::contents_needed(f);
        let new_content = if want_new { new_of(&f.path) } else { None };
        let old_content = if want_old {
            blobs.get(&format!("{}:{}", d.base, f.old_path)).cloned()
        } else {
            None
        };

        let (v, mut fs_forks) =
            rules::tests(f, new_content.as_deref(), old_content.as_deref());
        violations.extend(v);
        forks.append(&mut fs_forks);
        forks.extend(rules::hygiene(f));
        forks.extend(rules::infra(f));
    }

    // ---- forks from the dependency delta ---------------------------------
    const UNMEASURED: &str = "transitive cost unmeasured (no Cargo.lock in this repo)";
    let per_dep: Vec<(String, Option<usize>, usize)> = match &delta {
        Some(dl) => dl
            .per_dep
            .iter()
            .map(|(d, fresh, nuses)| (d.clone(), Some(*fresh), *nuses))
            .collect(),
        None => direct
            .iter()
            .map(|(d, _, _)| (d.clone(), None, uses.get(d).copied().unwrap_or(0)))
            .collect(),
    };

    if direct.len() >= 4 {
        let cost = match &delta {
            Some(dl) => format!("+{} packages, {} already in the tree", dl.fresh, dl.already),
            None => UNMEASURED.to_string(),
        };
        forks.push(Fork {
            file: direct[0].2.clone(),
            count: 1,
            line: direct[0].1,
            title: format!("{} direct deps for one change", direct.len()),
            native: "ship the part of the feature that needs none of them".to_string(),
            cost,
            ask: "which of these does the diff call directly?".to_string(),
            weight: 4,
        });
    }

    for (dep, fresh, nuses) in &per_dep {
        let loc = direct
            .iter()
            .find(|(d, _, _)| d == dep)
            .map(|(_, l, f)| (f.clone(), *l))
            .unwrap_or((String::from("Cargo.toml"), 0));
        let card = cards::card(dep);

        if *nuses == 0 {
            let cost = match (&card, fresh) {
                (Some(c), Some(_)) => c.cost.to_string(),
                (Some(c), None) => format!("{}; {UNMEASURED}", c.cost),
                (None, Some(n)) => format!("+{n} package{} with no caller yet", if *n == 1 { "" } else { "s" }),
                (None, None) => UNMEASURED.to_string(),
            };
            forks.push(Fork {
                file: loc.0,
                line: loc.1,
                count: 1,
                title: format!("{dep} added, 0 references in this diff"),
                native: card
                    .as_ref()
                    .map(|c| c.native.to_string())
                    .unwrap_or_else(|| "remove it until the change that uses it".to_string()),
                cost,
                ask: card
                    .as_ref()
                    .map(|c| c.ask.to_string())
                    .unwrap_or_else(|| "was this added for a later change?".to_string()),
                weight: 3,
            });
        } else if let Some(c) = card {
            let cost = match fresh {
                Some(0) => "+0 packages: already in the tree".to_string(),
                Some(n) => format!("+{n} package{}", if *n == 1 { "" } else { "s" }),
                None => UNMEASURED.to_string(),
            };
            forks.push(Fork {
                file: loc.0,
                line: loc.1,
                count: 1,
                title: format!("{dep} added ({nuses} ref{})", if *nuses == 1 { "" } else { "s" }),
                native: c.native.to_string(),
                cost,
                ask: c.ask.to_string(),
                weight: 3,
            });
        }
    }

    forks.sort_by(|a, b| {
        b.weight
            .cmp(&a.weight)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    // Collapse repeats within one file: four identical skips in a file are one
    // finding with a count. Keyed by file too, so the location shown is real.
    let mut deduped: Vec<Fork> = Vec::new();
    for f in forks {
        match deduped
            .iter_mut()
            .find(|e| e.title == f.title && e.file == f.file)
        {
            Some(e) => e.count += 1,
            None => deduped.push(f),
        }
    }
    let mut forks = deduped;
    let total_forks = forks.len();
    let cap = match (args.verbose, violations.is_empty()) {
        (true, _) => usize::MAX,
        (false, true) => 3,
        (false, false) => 2,
    };
    forks.truncate(cap);
    let omitted = total_forks - forks.len();

    let ms = started.elapsed().as_millis();
    let blocked = !violations.is_empty();

    let summary = summary_line(&delta, &go_direct, go_modules, violations.len());

    if args.json {
        println!(
            "{}",
            to_json(
                language,
                d.files.len(),
                ms,
                &summary,
                &delta,
                &violations,
                &forks,
                omitted,
                blocked
            )
        );
    } else {
        println!(
            "kathetos · {language} · {} file{} · {ms}ms{summary}",
            d.files.len(),
            if d.files.len() == 1 { "" } else { "s" }
        );
        for v in &violations {
            println!("violation: {} ({}:{}) [{}]", v.rule, v.file, v.line, rules::BASIS);
        }
        if !forks.is_empty() {
            println!("forks: {total_forks}");
            for (i, f) in forks.iter().enumerate() {
                let more = if f.count > 1 {
                    format!(" (x{})", f.count)
                } else {
                    String::new()
                };
                println!("{}. {}:{}  {}{}", i + 1, f.file, f.line, f.title, more);
                println!("   native: {}", f.native);
                println!("   6mo:    {}", f.cost);
                println!("   ask:    {}", f.ask);
            }
            if omitted > 0 {
                println!("   … {omitted} more not shown (--verbose lists all)");
            }
        }
    }

    exit(if blocked { 1 } else { 0 });
}

fn summary_line(
    delta: &Option<lock::CargoDelta>,
    go_direct: &[String],
    go_modules: usize,
    violations: usize,
) -> String {
    let mut parts = Vec::new();
    if let Some(d) = delta {
        parts.push(format!(
            "deps +{} -> +{} pkg ({} in tree)",
            d.direct.len(),
            d.fresh,
            d.already
        ));
    } else if !go_direct.is_empty() || go_modules > 0 {
        parts.push(format!(
            "go +{} direct -> +{} modules (attribution unavailable)",
            go_direct.len(),
            go_modules
        ));
    }
    parts.push(format!("tests {violations}"));
    format!(" · {}", parts.join(" · "))
}

fn die(msg: &str) -> ! {
    eprintln!("kathetos: {msg}");
    eprintln!("  try: kathetos check --diff origin/main...HEAD");
    exit(2)
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push(' '),
            c => o.push(c),
        }
    }
    o
}

#[allow(clippy::too_many_arguments)]
fn to_json(
    language: &str,
    files: usize,
    ms: u128,
    summary: &str,
    delta: &Option<lock::CargoDelta>,
    violations: &[Violation],
    forks: &[Fork],
    omitted: usize,
    blocked: bool,
) -> String {
    let deps = match delta {
        Some(d) => format!(
            "{{\"direct\":{},\"transitive\":{},\"packages_added\":{},\"already_in_tree\":{}}}",
            d.direct.len(),
            d.fresh,
            d.added.len(),
            d.already
        ),
        None => "null".to_string(),
    };
    let vs: Vec<String> = violations
        .iter()
        .map(|v| {
            format!(
                "{{\"file\":\"{}\",\"line\":{},\"rule\":\"{}\",\"basis\":\"{}\"}}",
                esc(&v.file),
                v.line,
                esc(v.rule),
                rules::BASIS
            )
        })
        .collect();
    let fs: Vec<String> = forks
        .iter()
        .map(|f| {
            format!(
                "{{\"file\":\"{}\",\"line\":{},\"title\":\"{}\",\"native\":\"{}\",\"cost\":\"{}\",\"ask\":\"{}\",\"weight\":{},\"count\":{}}}",
                esc(&f.file),
                f.line,
                esc(&f.title),
                esc(&f.native),
                esc(&f.cost),
                esc(&f.ask),
                f.weight,
                f.count
            )
        })
        .collect();

    format!(
        "{{\"v\":1,\"lang\":\"{}\",\"files\":{},\"ms\":{},\"summary\":\"{}\",\"deps\":{},\"violations\":[{}],\"forks\":[{}],\"forks_omitted\":{},\"blocked\":{}}}",
        language,
        files,
        ms,
        esc(summary),
        deps,
        vs.join(","),
        fs.join(","),
        omitted,
        blocked
    )
}
