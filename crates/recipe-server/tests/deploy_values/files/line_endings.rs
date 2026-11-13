use std::collections::BTreeSet;

use super::docker::{DB_CONTEXT_LIST, PGTOOLS_CONTEXT_LIST, copy_sources};
use crate::fault::Report;
use crate::lines::code_lines;

/// The path of the attribute file, as the faults of `check_line_endings` show it.
const ATTRIBUTES_FILE: &str = ".gitattributes";

/// True when the glob `pattern` matches the whole `text`. `*` matches any run of characters but `/`.
/// `?` matches one character but `/`. Every other character matches itself.
fn glob_matches(pattern: &[char], text: &[char]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some(('*', rest)) => {
            for skip in 0..=text.len() {
                if glob_matches(rest, text.get(skip..).unwrap_or_default()) {
                    return true;
                }
                if text.get(skip) == Some(&'/') {
                    return false;
                }
            }
            false
        }
        Some((c, rest)) => text.split_first().is_some_and(|(first, tail)| {
            ((*c == '?' && *first != '/') || c == first) && glob_matches(rest, tail)
        }),
    }
}

/// True when some line of `.gitattributes` sets `text` and `eol=lf` for the path.
/// Supported subset: a comment, an empty line, and `pattern attribute...`.
/// A pattern is a path from the repository root, or a name without `/` that matches at any depth.
/// A pattern may use `*` and `?`. It never uses `**`, a trailing `/`, `!`, or a character class.
/// An attribute is `text` or `eol=lf` as a plain word. The check does not model `-text` or the
/// override order of Git, so a later line `-text` for the same path is outside its scope.
fn has_lf_rule(attributes: &str, path: &str) -> bool {
    let chars = |text: &str| text.chars().collect::<Vec<char>>();
    let name = path.rsplit('/').next().unwrap_or(path);
    code_lines(attributes).iter().any(|line| {
        let mut words = line.split_whitespace();
        let Some(pattern) = words.next() else {
            return false;
        };
        let pattern = pattern.trim_start_matches('/');
        let rest: Vec<&str> = words.collect();
        let target = if pattern.contains('/') { path } else { name };
        glob_matches(&chars(pattern), &chars(target))
            && rest.contains(&"text")
            && rest.contains(&"eol=lf")
    })
}

/// The files of the repository that a `COPY` source names. A file is its own path.
/// A directory source is each file under it. An empty answer means that the source does not exist.
fn files_of_source(listing: &[String], path: &str) -> Vec<String> {
    let prefix = format!("{path}/");
    listing
        .iter()
        .filter(|file| file.as_str() == path || file.starts_with(&prefix))
        .cloned()
        .collect()
}

/// Each file that a `COPY` line of the Dockerfile names has a `text eol=lf` rule in `.gitattributes`.
/// The two allow lists have one too. A CRLF line ending of a Windows checkout then never reaches the
/// build context (ADR-0064 (g), design D2). `listing` holds the repository paths under `deploy/db`
/// and `deploy/pgtools`. A source that the listing lacks is a fault. A `COPY` from a stage names no file.
/// The pattern matching follows `has_lf_rule`.
pub fn check_line_endings(dockerfile: &str, attributes: &str, listing: &[String]) -> Report {
    let mut report = Report::default();
    let (db, pgtools) = copy_sources(&mut report, dockerfile);
    let mut wanted: BTreeSet<String> = [DB_CONTEXT_LIST, PGTOOLS_CONTEXT_LIST]
        .map(String::from)
        .into();
    for (directory, sources) in [("deploy/db", db), ("deploy/pgtools", pgtools)] {
        for source in sources {
            let path = format!("{directory}/{source}");
            let files = files_of_source(listing, &path);
            if files.is_empty() {
                report.fail(
                    &path,
                    &source,
                    "a file or a directory of the repository",
                    "none",
                );
            }
            wanted.extend(files);
        }
    }
    for path in wanted {
        if !has_lf_rule(attributes, &path) {
            report.fail(
                ATTRIBUTES_FILE,
                &path,
                "a rule with text and eol=lf",
                "no rule",
            );
        }
    }
    report
}
