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
///
/// The scan fails closed. A line that stops the scan and is not a blank line, a `}`, or the end
/// of a statement can end an attribute that spans lines, such as `))]`. The scan did not read it.
fn is_test_attribute_group(lines: &[&str], index: usize) -> bool {
    let above: Vec<&str> = lines
        .get(..index)
        .unwrap_or_default()
        .iter()
        .rev()
        .map(|line| line.trim())
        .collect();
    let group = above
        .iter()
        .take_while(|line| line.starts_with("#[") || line.starts_with("//"));
    let stop = above
        .iter()
        .find(|line| !(line.starts_with("#[") || line.starts_with("//")));
    if stop.is_some_and(|line| !ends_an_item(line)) {
        return false;
    }
    let (mut has_test, mut has_other_cfg) = (false, false);
    for attribute in group.filter(|line| line.starts_with("#[")) {
        has_test |= *attribute == "#[test]" || *attribute == "#[tokio::test]";
        has_other_cfg |= attribute.starts_with("#[cfg(") && *attribute != "#[cfg(test)]";
        has_other_cfg |= disables_a_test(attribute);
    }
    has_test && !has_other_cfg
}

/// Return true when an attribute line can skip a test: `#[ignore`, or a `#[cfg_attr(` that
/// names `ignore` or does not close on its line. CI runs no ignored test.
fn disables_a_test(attribute: &str) -> bool {
    attribute.starts_with("#[ignore")
        || (attribute.starts_with("#[cfg_attr(")
            && (attribute.contains("ignore") || !attribute.ends_with(']')))
}

/// Return true when a line is blank or closes or opens an item, so no attribute can end on it.
fn ends_an_item(line: &str) -> bool {
    line.is_empty() || line == "}" || line.ends_with('{') || line.ends_with(';')
}

/// Return the test file and the line of the test `fn name(`. The files of this module do not
/// count.
///
/// A test `fn` has `#[test]` or `#[tokio::test]` in its attribute group and no `#[cfg(...)]`
/// other than `#[cfg(test)]`. A plain `fn` or a `fn` under another cfg does not run.
fn find_test_fn<'a>(sources: &'a [Source], name: &str) -> Option<(&'a Source, usize)> {
    let needle = format!("fn {name}(");
    sources
        .iter()
        .filter(|source| is_test_source(source) && !source.relative.starts_with(SELF_DIR))
        .find_map(|source| {
            let lines: Vec<&str> = source.text.lines().collect();
            let line = (0..lines.len()).find(|index| {
                lines
                    .get(*index)
                    .is_some_and(|line| declares_fn(line, &needle))
                    && is_test_attribute_group(&lines, *index)
            })?;
            Some((source, line))
        })
}

/// One `fn`, `const`, or `static` of a test file: its name, its first line, and its code without
/// comments.
struct Item {
    name: String,
    line: usize,
    code: String,
}

/// Return the name that a line declares with `fn`, `const`, or `static`, or `None`.
fn declared_item_name(line: &str) -> Option<&str> {
    let mut rest = code_of(line).trim_start();
    if let Some(after) = rest.strip_prefix("pub(") {
        rest = after
            .split_once(')')
            .map_or("", |(_, tail)| tail)
            .trim_start();
    } else if let Some(after) = rest.strip_prefix("pub ") {
        rest = after.trim_start();
    }
    rest = rest.strip_prefix("async ").unwrap_or(rest);
    rest = ["fn ", "const ", "static "]
        .iter()
        .find_map(|keyword| rest.strip_prefix(keyword))?;
    rest = rest.strip_prefix("fn ").unwrap_or(rest);
    let end = rest.find(|c: char| !is_word(c)).unwrap_or(rest.len());
    rest.get(..end).filter(|name| !name.is_empty())
}

/// Return the index of the last line of the item that starts on line `first`.
///
/// An item ends on its first line when that line ends in `;` or holds the whole body. Else it ends
/// at the next line that closes at the indent of the first line.
fn item_end(lines: &[&str], first: usize) -> usize {
    let head = code_of(lines.get(first).copied().unwrap_or_default()).trim_end();
    if head.ends_with(';') || (head.contains('{') && head.ends_with('}')) {
        return first;
    }
    let indent = indent_of(lines.get(first).copied().unwrap_or_default());
    (first.saturating_add(1)..lines.len())
        .find(|index| {
            let line = lines.get(*index).copied().unwrap_or_default();
            let code = code_of(line).trim();
            indent_of(line) <= indent && (code.starts_with(['}', ')', ']']) || code.ends_with(';'))
        })
        .unwrap_or(lines.len().saturating_sub(1))
}

/// Return every `fn`, `const`, and `static` of a source text.
fn items_of(text: &str) -> Vec<Item> {
    let lines: Vec<&str> = text.lines().collect();
    lines
        .iter()
        .enumerate()
        .filter_map(|(line, text)| {
            let name = declared_item_name(text)?.to_owned();
            let last = item_end(&lines, line);
            let code = lines
                .get(line..=last)
                .unwrap_or_default()
                .iter()
                .map(|line| code_of(line))
                .collect::<Vec<_>>()
                .join("\n");
            Some(Item { name, line, code })
        })
        .collect()
}

/// Return the code of the item on line `line` and of each item of the file that it names, directly
/// or through another item.
///
/// A test names its request in a helper `fn` or in a `const` table of the same file. An item of
/// another file does not count, because a common name would reach the whole crate.
fn reachable_code(items: &[Item], line: usize) -> String {
    let mut reached: BTreeSet<usize> = BTreeSet::new();
    let mut open: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.line == line)
        .map(|(index, _)| index)
        .collect();
    while let Some(index) = open.pop() {
        if !reached.insert(index) {
            continue;
        }
        let Some(item) = items.get(index) else {
            continue;
        };
        let words: BTreeSet<&str> = item.code.split(|c: char| !is_word(c)).collect();
        open.extend(
            items
                .iter()
                .enumerate()
                .filter(|(other, named)| {
                    !reached.contains(other) && words.contains(named.name.as_str())
                })
                .map(|(other, _)| other),
        );
    }
    reached
        .iter()
        .filter_map(|index| items.get(*index))
        .map(|item| item.code.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Return the index just after the `)` that closes the `(` at `open`, or the text length.
///
/// A parenthesis inside a string literal does not count.
fn after_closing_paren(text: &str, open: usize) -> usize {
    let (mut depth, mut in_string, mut escaped) = (0_usize, false, false);
    for (at, c) in text.char_indices().skip_while(|(at, _)| *at < open) {
        match (in_string, c) {
            (true, _) if escaped => escaped = false,
            (true, '\\') => escaped = true,
            (true, '"') => in_string = false,
            (true, _) => {}
            (false, '"') => in_string = true,
            (false, '(') => depth = depth.saturating_add(1),
            (false, ')') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return at.saturating_add(1);
                }
            }
            (false, _) => {}
        }
    }
    text.len()
}

/// Return the text without the arguments of each call or macro whose name starts with `assert`.
///
/// A name in a message or in an argument of an assertion does not submit a field.
fn without_assertions(code: &str) -> String {
    let mut kept = String::new();
    let mut rest = code;
    while let Some(at) = rest.find("assert") {
        let start = rest
            .get(..at)
            .unwrap_or_default()
            .rfind(|c: char| !is_word(c))
            .map_or(0, |end| end.saturating_add(1));
        let tail = rest.get(start..).unwrap_or_default();
        let word_len = tail.find(|c: char| !is_word(c)).unwrap_or(tail.len());
        let word = tail.get(..word_len).unwrap_or_default();
        let after = tail.get(word_len..).unwrap_or_default().trim_start();
        let call = after.strip_prefix('!').unwrap_or(after).trim_start();
        if (word.starts_with("assert") || word.starts_with("debug_assert")) && call.starts_with('(')
        {
            kept.push_str(rest.get(..start).unwrap_or_default());
            let open = rest.len().saturating_sub(call.len());
            let close = after_closing_paren(rest, open);
            rest = rest.get(close..).unwrap_or_default();
        } else {
            let resume = start.saturating_add(word_len.max(1));
            kept.push_str(rest.get(..resume).unwrap_or(rest));
            rest = rest.get(resume..).unwrap_or_default();
        }
    }
    kept.push_str(rest);
    kept
}

/// Return true when the code submits the field in a request.
///
/// A request holds the name as a quoted text (`("note", ..)`, a list of names), as a form key
/// (`note=`), or as a key of a struct literal (`note: ..`). The name inside a longer word does not
/// count.
fn submits_field(code: &str, field: &str) -> bool {
    let is_alone = |key: &str, accepts: fn(Option<char>) -> bool| {
        code.match_indices(key).any(|(at, _)| {
            let before = code.get(..at).and_then(|head| head.chars().next_back());
            let after = code
                .get(at.saturating_add(key.len())..)
                .and_then(|tail| tail.chars().next());
            !before.is_some_and(is_word) && accepts(after)
        })
    };
    code.contains(&format!("\"{field}\""))
        || is_alone(&format!("{field}="), |_| true)
        || is_alone(&format!("{field}:"), |after| {
            after.is_some_and(char::is_whitespace)
        })
}

/// Return true when the test and the items of its file that it names submit the field.
fn test_submits_field(source: &Source, line: usize, field: &str) -> bool {
    let code = reachable_code(&items_of(&source.text), line);
    submits_field(&without_assertions(&code), field)
}

/// The integer types of Rust.
const INTEGER_TYPES: [&str; 12] = [
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];

/// The number of lines that a read and its parse call can span: the line of the read and three
/// more.
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

/// Return the code of a source line without its `//` comment.
fn code_of(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code)
}

/// Return true when the line declares a function: the word `fn` and a name follow no identifier
/// character. A `pub(crate)` before it is no obstacle.
fn declares_any_fn(line: &str) -> bool {
    let code = code_of(line);
    code.match_indices("fn ").any(|(at, _)| {
        let before = code.get(..at).and_then(|head| head.chars().next_back());
        let name = code
            .get(at.saturating_add(3)..)
            .and_then(|tail| tail.chars().next());
        !before.is_some_and(is_word) && name.is_some_and(is_word)
    })
}

/// Return the indexes of the lines that sit inside the body of a `struct` declaration.
///
/// The body runs from the `struct` line to the next line that starts with `}` at the same indent.
fn struct_body_lines(lines: &[&str]) -> BTreeSet<usize> {
    let mut inside = BTreeSet::new();
    let mut open_indent: Option<usize> = None;
    for (index, line) in lines.iter().enumerate() {
        let code = code_of(line);
        if let Some(indent) = open_indent {
            inside.insert(index);
            if code.trim_start().starts_with('}') && indent_of(code) <= indent {
                open_indent = None;
            }
        } else if code.trim_end().ends_with('{') && declares_struct(code) {
            inside.insert(index);
            open_indent = Some(indent_of(code));
        }
    }
    inside
}

/// Return true when the code line starts a `struct` item, with or without a visibility.
fn declares_struct(code: &str) -> bool {
    code.match_indices("struct ").any(|(at, _)| {
        let before = code.get(..at).unwrap_or_default();
        !before.chars().next_back().is_some_and(is_word)
    })
}

/// Return true when an even count of `"` comes before the byte `at`. The byte is outside a string.
fn is_outside_string(code: &str, at: usize) -> bool {
    let quotes = code.get(..at).unwrap_or_default().matches('"').count();
    quotes % 2 == 0
}

/// Return true when the code line holds the name as a word outside a string.
fn names_word(code: &str, name: &str) -> bool {
    code.match_indices(name).any(|(at, _)| {
        let before = code.get(..at).and_then(|head| head.chars().next_back());
        let after = code
            .get(at.saturating_add(name.len())..)
            .and_then(|tail| tail.chars().next());
        !before.is_some_and(is_word) && !after.is_some_and(is_word) && is_outside_string(code, at)
    })
}

/// Return true when the module reads the field of an `Unread` entry in any form.
///
/// A read is `.name` (`form._note`, `self._note`) or the bare word `name` in a destructure pattern
/// (`Form { _note: n, .. }`, `Form { _note, .. }`). The declaration in a struct body, a comment,
/// and a string are no read. A word of another struct with the same name counts as a read, so the
/// rule fails closed.
fn reads_unread_field(module: &str, name: &str) -> bool {
    let lines: Vec<&str> = module.lines().collect();
    let declared = struct_body_lines(&lines);
    lines
        .iter()
        .enumerate()
        .filter(|(index, _)| !declared.contains(index))
        .map(|(_, line)| code_of(line))
        .any(|code| reads_field(code, name) || names_word(code, name))
}

/// The struct that a parse window must belong to, and the other structs of the module that have a
/// field of the same name.
pub(super) struct Owner<'a> {
    pub(super) item: &'a str,
    pub(super) rivals: &'a [&'a str],
}

/// Return the number of whitespace characters that start the line.
fn indent_of(line: &str) -> usize {
    line.len().saturating_sub(line.trim_start().len())
}

/// Return the `impl` header line above an indented function, or an empty text.
///
/// The header is the nearest line above that starts with a letter in column 0. A method sits in the
/// block of that line.
fn impl_header<'a>(lines: &[&'a str], declared: usize) -> &'a str {
    let indented = lines
        .get(declared)
        .is_some_and(|line| line.starts_with(char::is_whitespace));
    let header = lines
        .get(..declared)
        .unwrap_or_default()
        .iter()
        .rev()
        .find(|line| line.starts_with(char::is_alphabetic));
    header
        .filter(|line| indented && line.starts_with("impl"))
        .map_or("", |line| *line)
}

/// Return true when the function that holds the window names the struct of the entry and no rival.
///
/// The function text runs from its `fn` line to the last line of the window. It stops before the
/// next `fn` line and at the `}` that closes the function. A method also has the `impl` header of
/// its block in the text. The window of a function that names a rival belongs to no struct, so it
/// covers no entry.
fn window_belongs_to(lines: &[&str], start: usize, owner: &Owner<'_>) -> bool {
    let window_end = start.saturating_add(PARSE_WINDOW_LINES).min(lines.len());
    let head = lines.get(..start).unwrap_or_default();
    let Some(declared) = head.iter().rposition(|line| declares_any_fn(line)) else {
        return false;
    };
    let indent = indent_of(lines.get(declared).copied().unwrap_or_default());
    let function_end = (start.saturating_add(1)..window_end)
        .find(|index| {
            lines.get(*index).is_some_and(|line| {
                declares_any_fn(line)
                    || (line.trim_start().starts_with('}') && indent_of(line) <= indent)
            })
        })
        .unwrap_or(window_end);
    let body = lines
        .get(declared..function_end)
        .unwrap_or_default()
        .iter()
        .map(|line| code_of(line))
        .collect::<Vec<_>>()
        .join("\n");
    let function = format!("{}\n{body}", impl_header(lines, declared));
    has_bounded(&function, owner.item, true)
        && !owner
            .rivals
            .iter()
            .any(|rival| has_bounded(&function, rival, true))
}

/// Return true when a read of `field` and the call of `path` are within the window, the window
/// reads no field of `others`, and the window belongs to the struct of `owner`.
fn parses_field(text: &str, field: &str, others: &[&str], path: &str, owner: &Owner<'_>) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    (0..lines.len()).any(|start| {
        let window: Vec<&str> = lines
            .iter()
            .skip(start)
            .take(PARSE_WINDOW_LINES)
            .map(|line| code_of(line))
            .collect();
        let first = window.first().copied().unwrap_or_default();
        let window = window.join("\n");
        reads_field(first, field)
            && has_bounded(&window, path, true)
            && !others.iter().any(|other| reads_field(&window, other))
            && window_belongs_to(&lines, start, owner)
    })
}

/// Return the fault of a kind that the field and its module do not show, or `None`.
fn kind_fault(
    kind: Kind,
    field: &ScannedField,
    siblings: &[&str],
    module: &str,
    owner: &Owner<'_>,
) -> Option<String> {
    let ty = &field.ty;
    match kind {
        Kind::Unread if !field.rust_name.starts_with('_') => Some(format!(
            "has the kind Unread, but its Rust name `{}` does not start with `_`",
            field.rust_name
        )),
        Kind::Unread => reads_unread_field(module, &field.rust_name).then(|| {
            format!(
                "has the kind Unread, but the module reads `{}`",
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
        Kind::Parsed(path) => (!parses_field(module, &field.rust_name, siblings, path, owner)).then(|| {
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
    let rivals: Vec<&str> = scan
        .structs
        .iter()
        .filter(|other| {
            other.name != scanned.name
                && other
                    .fields
                    .iter()
                    .any(|rival| rival.rust_name == field.rust_name)
        })
        .map(|other| other.name.as_str())
        .collect();
    let owner = Owner {
        item: &scanned.name,
        rivals: &rivals,
    };
    kind_fault(kind, field, &siblings, module, &owner)
        .map(|fault| format!("the entry {} {fault}", describe(&entry_key(entry))))
}

/// Return the fault of the cover of an entry, or `None` when the cover is sound.
fn cover_fault(entry: &Entry, sources: &[Source]) -> Option<String> {
    match entry.3 {
        Cover::Corpus(name) => {
            let key = describe(&entry_key(entry));
            let Some((source, line)) = find_test_fn(sources, name) else {
                return Some(format!("the corpus test of {key} has no fn: {name}"));
            };
            (!test_submits_field(source, line, entry.2))
                .then(|| format!("the corpus test of {key} does not submit the field: {name}"))
        }
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
