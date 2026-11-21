//! The comparison of the source files with the table.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::registry::{Cover, Entry, Kind};
use super::scan::{ScannedField, scan_source};
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
            for wire in item.fields.iter().map(|field| &field.wire_name) {
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

/// Return true when the line declares `fn name(` and nothing but modifiers comes before it.
fn declares_fn(line: &str, needle: &str) -> bool {
    line.find(needle)
        .and_then(|at| line.get(..at))
        .is_some_and(|head| {
            head.chars()
                .all(|c| c.is_alphanumeric() || c == ' ' || c == '(' || c == ')')
        })
}

/// Return true when the attribute group above line `index` marks a test and holds no other cfg.
fn is_test_attribute_group(lines: &[&str], index: usize) -> bool {
    let above = lines
        .get(..index)
        .unwrap_or_default()
        .iter()
        .rev()
        .map(|line| line.trim())
        .take_while(|line| line.starts_with("#[") || line.starts_with("//"))
        .filter(|line| line.starts_with("#["));
    let (mut has_test, mut has_other_cfg) = (false, false);
    for attribute in above {
        has_test |= attribute == "#[test]" || attribute == "#[tokio::test]";
        has_other_cfg |= attribute.starts_with("#[cfg(") && attribute != "#[cfg(test)]";
    }
    has_test && !has_other_cfg
}

/// Return true when a test file of `src/` has a test `fn name(`. The files of this module do not count.
///
/// A test `fn` has `#[test]` or `#[tokio::test]` in its attribute group and no `#[cfg(...)]`
/// other than `#[cfg(test)]`. A plain `fn` or a `fn` under another cfg does not run.
fn has_test_fn(sources: &[Source], name: &str) -> bool {
    let needle = format!("fn {name}(");
    sources
        .iter()
        .filter(|source| is_test_source(source) && !source.relative.starts_with(SELF_DIR))
        .any(|source| {
            let lines: Vec<&str> = source.text.lines().collect();
            lines.iter().enumerate().any(|(index, line)| {
                declares_fn(line, &needle) && is_test_attribute_group(&lines, index)
            })
        })
}

/// The integer types of Rust.
const INTEGER_TYPES: [&str; 12] = [
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];

/// The number of lines that a read and its parse call can span: the line of the read and three more.
const PARSE_WINDOW_LINES: usize = 4;

/// Remove each `Option<...>` around a type text.
fn without_option(ty: &str) -> &str {
    let mut rest = ty.trim();
    while let Some(inner) = rest
        .strip_prefix("Option<")
        .and_then(|inner| inner.strip_suffix('>'))
    {
        rest = inner.trim();
    }
    rest
}

/// Return true when the character can be part of a Rust identifier.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Return true when `needle` is in `text` and no identifier character follows it.
///
/// With `check_start`, no identifier character comes before it either.
fn has_bounded(text: &str, needle: &str, check_start: bool) -> bool {
    text.match_indices(needle).any(|(at, _)| {
        let before = text.get(..at).and_then(|head| head.chars().next_back());
        let after = text
            .get(at.saturating_add(needle.len())..)
            .and_then(|tail| tail.chars().next());
        !(check_start && before.is_some_and(is_word)) && !after.is_some_and(is_word)
    })
}

/// Return true when the text reads the field with `.field`.
fn reads_field(text: &str, field: &str) -> bool {
    has_bounded(text, &format!(".{field}"), false)
}

/// Return true when a read of `field` and the call of `path` are within the window, and the window
/// reads no field of `others`.
fn parses_field(text: &str, field: &str, others: &[&str], path: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    (0..lines.len()).any(|start| {
        let window: Vec<&str> = lines
            .iter()
            .skip(start)
            .take(PARSE_WINDOW_LINES)
            .copied()
            .collect();
        let first = window.first().copied().unwrap_or_default();
        let window = window.join("\n");
        reads_field(first, field)
            && has_bounded(&window, path, true)
            && !others.iter().any(|other| reads_field(&window, other))
    })
}

/// Return the fault of a kind that the field and its module do not show, or `None`.
fn kind_fault(kind: Kind, field: &ScannedField, siblings: &[&str], module: &str) -> Option<String> {
    let ty = &field.ty;
    match kind {
        Kind::Unread => (!field.rust_name.starts_with('_')).then(|| {
            format!(
                "has the kind Unread, but its Rust name `{}` does not start with `_`",
                field.rust_name
            )
        }),
        Kind::Typed => {
            let inner = without_option(ty);
            (inner != "bool" && !INTEGER_TYPES.contains(&inner))
                .then(|| format!("has the kind Typed, but its type `{ty}` is no integer or bool"))
        }
        Kind::Secret => (without_option(ty) != "SecretField")
            .then(|| format!("has the kind Secret, but its type `{ty}` is no SecretField")),
        Kind::Parsed(path) => (!parses_field(module, &field.rust_name, siblings, path)).then(|| {
            format!(
                "has the kind Parsed, but no read of `.{}` has `{path}` within three lines and no other field",
                field.rust_name
            )
        }),
    }
}

/// Return the fault of the kind of a `NoText` entry, or `None` when the source shows the kind.
///
/// An entry without a field in the sources has no check here. The comparison names it already.
fn no_text_fault(entry: &Entry, kind: Kind, sources: &[Source]) -> Option<String> {
    let (file, item, wire, _cover) = entry;
    let module = &sources.iter().find(|source| source.relative == *file)?.text;
    let scan = scan_source(module);
    let scanned = scan.structs.iter().find(|scanned| scanned.name == *item)?;
    let field = scanned
        .fields
        .iter()
        .find(|field| field.wire_name == *wire)?;
    let siblings: Vec<&str> = scanned
        .fields
        .iter()
        .filter(|other| other.rust_name != field.rust_name)
        .map(|other| other.rust_name.as_str())
        .collect();
    kind_fault(kind, field, &siblings, module)
        .map(|fault| format!("the entry {} {fault}", describe(&entry_key(entry))))
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
        Cover::NoText(kind) => no_text_fault(entry, kind, sources),
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
