use std::collections::BTreeSet;

use crate::fault::Report;
use crate::lines::code_lines;

/// The `Dockerfile` of the image `recipe-pgtools` and the allow lists of its two build contexts.
pub const PGTOOLS_DOCKERFILE: &str = "deploy/pgtools/Dockerfile";

pub const DB_CONTEXT_LIST: &str = "deploy/db/.dockerignore";

pub const PGTOOLS_CONTEXT_LIST: &str = "deploy/pgtools/.dockerignore";

/// The name of the second build context. It is `deploy/db` (`compose.build.yaml`, design D2).
const DB_CONTEXT: &str = "db";

/// Remove a leading `./` and a trailing `/`. A directory source `bin/` and the entry `!bin` then match.
fn context_name(path: &str) -> &str {
    path.trim_start_matches("./").trim_end_matches('/')
}

/// The names of the build stages: `FROM image AS name`. A `COPY --from=name` of a stage reads no context.
fn stage_names(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let (first, second_last) = (words.first()?, words.iter().rev().nth(1)?);
            (first.eq_ignore_ascii_case("FROM") && second_last.eq_ignore_ascii_case("AS"))
                .then(|| words.last().map(|name| (*name).to_owned()))
                .flatten()
        })
        .collect()
}

/// The sources of the `COPY` lines of the Dockerfile for each context: `(db, pgtools)`.
/// A source with `--from=db` belongs to the db context. A source without `--from` belongs to the
/// pgtools context. A wildcard or an unknown `--from` is a fault, because no list can name it.
pub(super) fn copy_sources(
    report: &mut Report,
    dockerfile: &str,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let joined = dockerfile.replace("\\\r\n", " ").replace("\\\n", " ");
    let lines = code_lines(&joined);
    let stages = stage_names(&lines);
    let (mut db, mut pgtools) = (BTreeSet::new(), BTreeSet::new());
    for line in lines {
        let mut words = line.split_whitespace();
        if !words
            .next()
            .is_some_and(|word| word.eq_ignore_ascii_case("COPY"))
        {
            continue;
        }
        let (flags, mut paths): (Vec<&str>, Vec<&str>) =
            words.partition(|word| word.starts_with("--"));
        paths.pop();
        let from = flags.iter().find_map(|flag| flag.strip_prefix("--from="));
        let target = match from {
            None => &mut pgtools,
            Some(DB_CONTEXT) => &mut db,
            Some(name) if stages.iter().any(|stage| stage == name) => continue,
            Some(name) => {
                report.fail(
                    PGTOOLS_DOCKERFILE,
                    line,
                    "--from of the context db or of a stage",
                    name,
                );
                continue;
            }
        };
        for path in paths {
            if path.contains(['*', '?', '[']) {
                report.fail(
                    PGTOOLS_DOCKERFILE,
                    path,
                    "a source without a wildcard",
                    line,
                );
            }
            target.insert(context_name(path).to_owned());
        }
    }
    (db, pgtools)
}

/// The names that an allow list admits. The first line of code is `*`. Every other line is `!name`.
fn admitted_names(report: &mut Report, file: &str, text: &str) -> Vec<String> {
    let lines = code_lines(text);
    if lines.first().map(|line| line.trim()) != Some("*") {
        report.fail(
            file,
            "first line",
            "*",
            lines.first().copied().unwrap_or("no line"),
        );
    }
    let mut names = Vec::new();
    for line in lines.iter().skip(1) {
        match line.trim().strip_prefix('!') {
            Some(name) => names.push(context_name(name).to_owned()),
            None => report.fail(file, line.trim(), "a line !name", "another line"),
        }
    }
    names.sort();
    names
}

/// Each allow list admits exactly the sources of the `COPY` lines for its context (ADR-0064 (f), ADR-0103 (l)).
/// A `COPY` source is a path of the Dockerfile. A directory source `bin/` is admitted as `!bin`.
/// A file in a directory is admitted by its own path, so `!lib` would admit too much.
/// The list of the other context never admits the same source. So `deploy/pgtools/tests/` stays out.
pub fn check_context_lists(dockerfile: &str, db_list: &str, pgtools_list: &str) -> Report {
    let mut report = Report::default();
    let (db, pgtools) = copy_sources(&mut report, dockerfile);
    for (file, list, sources) in [
        (DB_CONTEXT_LIST, db_list, db),
        (PGTOOLS_CONTEXT_LIST, pgtools_list, pgtools),
    ] {
        let admitted = admitted_names(&mut report, file, list);
        let want: Vec<String> = sources.into_iter().collect();
        report.compare_list(file, "the names that !name admits", &want, Some(admitted));
    }
    report
}

/// The directory of the library files in the repository, and the directory that holds them in the image.
const LIBRARY_PREFIX: &str = "deploy/pgtools/lib/";

const LIBRARY_DESTINATION: &str = "/etc/pgtools/";

/// The pairs `(source, destination)` of the `COPY` lines that read the pgtools context.
/// A `COPY` with `--from` reads another context or a stage, so it is not in the answer.
fn pgtools_copies(dockerfile: &str) -> Vec<(String, String)> {
    let joined = dockerfile.replace("\\\r\n", " ").replace("\\\n", " ");
    let mut copies = Vec::new();
    for line in code_lines(&joined) {
        let mut words = line.split_whitespace();
        if !words
            .next()
            .is_some_and(|word| word.eq_ignore_ascii_case("COPY"))
        {
            continue;
        }
        let (flags, mut paths): (Vec<&str>, Vec<&str>) =
            words.partition(|word| word.starts_with("--"));
        if flags.iter().any(|flag| flag.starts_with("--from=")) {
            continue;
        }
        let Some(destination) = paths.pop() else {
            continue;
        };
        copies.extend(
            paths
                .into_iter()
                .map(|path| (context_name(path).to_owned(), destination.to_owned())),
        );
    }
    copies
}

/// Each file of `deploy/pgtools/lib/` reaches the image `recipe-pgtools`. The Dockerfile copies it to
/// `/etc/pgtools/` with a `COPY` line, and the allow list of the pgtools context admits its own path.
/// A script of `bin/` sources the file from there, so a missing copy breaks the script in the image.
pub fn check_library_copies(dockerfile: &str, pgtools_list: &str, listing: &[String]) -> Report {
    let mut report = Report::default();
    let copies = pgtools_copies(dockerfile);
    let admitted = admitted_names(&mut Report::default(), PGTOOLS_CONTEXT_LIST, pgtools_list);
    for path in listing
        .iter()
        .filter(|path| path.starts_with(LIBRARY_PREFIX))
    {
        let source = path.strip_prefix("deploy/pgtools/").unwrap_or(path);
        match copies.iter().find(|(name, _)| name == source) {
            None => report.fail(
                PGTOOLS_DOCKERFILE,
                source,
                "a COPY line from the pgtools context",
                "no COPY line",
            ),
            Some((_, destination)) if destination != LIBRARY_DESTINATION => report.fail(
                PGTOOLS_DOCKERFILE,
                source,
                format!("the destination {LIBRARY_DESTINATION}"),
                destination.as_str(),
            ),
            Some(_) => {}
        }
        if !admitted.iter().any(|name| name == source) {
            report.fail(
                PGTOOLS_CONTEXT_LIST,
                source,
                "an entry !name with its own path",
                "no entry",
            );
        }
    }
    report
}
