//! The comparison of the source files with the table.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::registry::{Cover, Entry};
use super::scan::scan_source;
use crate::test_support::source_walk::{files_below, is_test_path};

/// The folder of this module below `src/`. The search for a corpus test skips its files.
pub(super) const SELF_DIR: &str = "pages/xss_coverage_tests/";

/// The file, the struct, and the wire name of one field.
type Key = (String, String, String);

/// One source file of the crate: its path below `src/` with `/`, and its text.
pub(super) struct Source {
    pub(super) relative: String,
    pub(super) text: String,
}

/// Read every `.rs` file below the folder.
pub(super) fn read_sources(folder: &Path) -> Vec<Source> {
    files_below(folder)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| {
            let relative = path.strip_prefix(folder).unwrap();
            let parts: Vec<String> = relative
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect();
            Source {
                relative: parts.join("/"),
                text: fs::read_to_string(&path).unwrap(),
            }
        })
        .collect()
}

/// Return true when the file is test code.
fn is_test_source(source: &Source) -> bool {
    is_test_path(Path::new(&source.relative))
}

/// Return the key of every field of the production files. Add the faults of the scan to `faults`.
fn scanned_keys(sources: &[Source], faults: &mut Vec<String>) -> BTreeSet<Key> {
    let mut keys = BTreeSet::new();
    for source in sources.iter().filter(|source| !is_test_source(source)) {
        let file = &source.relative;
        let scan = scan_source(&source.text);
        faults.extend(scan.faults.iter().map(|fault| format!("{file}: {fault}")));
        if scan.derives != scan.structs.len() {
            faults.push(format!(
                "{file}: {} Deserialize derives, but {} parsed structs",
                scan.derives,
                scan.structs.len()
            ));
        }
        for item in &scan.structs {
            for wire in &item.wire_names {
                let key = (file.clone(), item.name.clone(), wire.clone());
                if !keys.insert(key) {
                    faults.push(format!(
                        "{file}: the wire name {wire} of {} repeats",
                        item.name
                    ));
                }
            }
        }
    }
    keys
}

/// Return the key of an entry.
fn entry_key(entry: &Entry) -> Key {
    let (file, item, wire, _cover) = entry;
    ((*file).to_owned(), (*item).to_owned(), (*wire).to_owned())
}

/// Return the text of a key for a fault.
fn describe(key: &Key) -> String {
    let (file, item, wire) = key;
    format!("{file} {item}.{wire}")
}

/// Return true when a test file of `src/` has `fn name(`. The files of this module do not count.
fn has_test_fn(sources: &[Source], name: &str) -> bool {
    let needle = format!("fn {name}(");
    sources
        .iter()
        .filter(|source| is_test_source(source) && !source.relative.starts_with(SELF_DIR))
        .any(|source| source.text.contains(&needle))
}

/// Return the fault of the cover of an entry, or `None` when the cover is sound.
fn cover_fault(entry: &Entry, sources: &[Source]) -> Option<String> {
    match entry.3 {
        Cover::Corpus(name) => (!has_test_fn(sources, name)).then(|| {
            format!(
                "the corpus test of {} has no fn: {name}",
                describe(&entry_key(entry))
            )
        }),
        Cover::NoText(reason) => reason.trim().is_empty().then(|| {
            format!(
                "the entry {} has an empty reason",
                describe(&entry_key(entry))
            )
        }),
    }
}

/// Return the faults of the comparison of the source files with the table. No fault is a pass.
pub(super) fn coverage_faults(sources: &[Source], table: &[Entry]) -> Vec<String> {
    let mut faults = Vec::new();
    let scanned = scanned_keys(sources, &mut faults);
    let listed: BTreeSet<Key> = table.iter().map(entry_key).collect();
    if listed.len() != table.len() {
        faults.push("the table lists a field twice".to_owned());
    }
    let missing = scanned.difference(&listed);
    faults.extend(missing.map(|key| format!("field without an entry: {}", describe(key))));
    let stale = listed.difference(&scanned);
    faults.extend(stale.map(|key| format!("entry without a field: {}", describe(key))));
    let cover_faults: BTreeSet<String> = table
        .iter()
        .filter_map(|entry| cover_fault(entry, sources))
        .collect();
    faults.extend(cover_faults);
    faults
}
