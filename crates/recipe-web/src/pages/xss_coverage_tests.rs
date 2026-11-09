//! The guard of the XSS corpus coverage of the request structs (task 3.1, design D4).
//!
//! Each field of a `Deserialize` struct in `src/` is a wire name that a client can fill with any
//! text. A field needs a corpus test that sends the payloads of `tests/web/xss-corpus.txt` through
//! it, or a reason that it carries no free text. The guard reads the source files as text and
//! compares each field with the table `TABLE`. A new request struct without an entry fails the
//! test.
//!
//! The guard reads structs only. Two pair readers take keys that no struct names, and the corpus
//! tests of `recipe_xss_tests.rs`, `approval_text_xss_tests.rs`, and `plant_xss_tests.rs` cover
//! them. A new pair reader is outside the guard, so the review of its handler checks its corpus
//! test.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::test_support::source_walk::{files_below, is_test_path};

/// The path of this file below `src/`. The search for a corpus test skips it.
const SELF_FILE: &str = "pages/xss_coverage_tests.rs";

/// What covers one field of a request struct.
#[derive(Clone, Copy, Debug)]
enum Cover {
    /// The name of the corpus test that sends the payloads through the field.
    Corpus(&'static str),
    /// The reason that the field carries no free text.
    NoText(&'static str),
}

/// One entry of the table: the file below `src/`, the struct, the wire name, and the cover.
type Entry = (&'static str, &'static str, &'static str, Cover);

/// The file, the struct, and the wire name of one field.
type Key = (String, String, String);

/// One item of a `#[serde(...)]` attribute: the key and the value of `key = "value"`.
type SerdeItem = (String, Option<String>);

const CSRF: Cover = Cover::NoText(
    "The CSRF layer compares the token before the handler runs. No handler reads or shows it.",
);
const NUMBER: Cover =
    Cover::NoText("A number or a flag. A bad text gives a form error without the sent text.");
const LANG: Cover = Cover::NoText(
    "A closed set of two language codes. The error line does not show the sent text.",
);
const ROLE: Cover =
    Cover::NoText("A closed set of role names. The form never shows the sent text of this field.");
const FILTERS: Cover =
    Cover::Corpus("approval_xss_filter_values_that_no_payload_can_satisfy_give_e_web_008");
const PLANT_GETS: Cover =
    Cover::Corpus("plant_xss_query_values_and_path_values_of_get_pages_give_their_error_code");
const RECIPE_GETS: Cover =
    Cover::Corpus("recipe_xss_query_values_of_get_pages_give_their_error_code");
const RECIPE_REFUSED: Cover =
    Cover::Corpus("recipe_xss_fields_that_the_server_refuses_give_their_error_code");
const RECIPE_TEXTS: Cover =
    Cover::Corpus("recipe_xss_text_fields_accept_a_valid_text_and_refuse_a_control_character");
const PLANT_REFUSED: Cover =
    Cover::Corpus("plant_xss_machine_fields_that_the_server_refuses_give_their_error_code");
const PLANT_MACHINE_NAMES: Cover =
    Cover::Corpus("plant_xss_machine_names_are_stored_and_shown_escaped");
const PLANT_RENAME: Cover =
    Cover::Corpus("plant_xss_rename_name_is_shown_escaped_in_the_answer_and_on_the_page");
const PLANT_UNIT_REFUSED: Cover =
    Cover::Corpus("plant_xss_unit_fields_that_the_server_refuses_give_their_error_code");

/// Each field of each `Deserialize` struct of `src/`, with the test or the reason that covers it.
const TABLE: &[Entry] = &[
    ("lang.rs", "LangForm", "lang", LANG),
    (
        "lang.rs",
        "LangForm",
        "next",
        Cover::Corpus("xss_corpus_next_field_of_post_lang_is_escaped"),
    ),
    ("lang.rs", "LangForm", "csrf_pre", CSRF),
    ("lang.rs", "LangForm", "csrf_token", CSRF),
    ("pages/diff.rs", "DiffQuery", "kind", FILTERS),
    ("pages/diff.rs", "DiffQuery", "page", FILTERS),
    ("pages/history.rs", "HistoryQuery", "before", FILTERS),
    (
        "pages/own.rs",
        "LoginFields",
        "login",
        Cover::Corpus("xss_corpus_login_field_of_post_login_is_escaped"),
    ),
    (
        "pages/own.rs",
        "LoginFields",
        "password",
        Cover::Corpus("xss_corpus_password_field_of_post_login_is_escaped"),
    ),
    ("pages/own.rs", "LoginFields", "csrf_pre", CSRF),
    ("pages/own.rs", "LoginFields", "csrf_token", CSRF),
    (
        "pages/own.rs",
        "PasswordFields",
        "current_password",
        Cover::Corpus("xss_corpus_current_password_field_of_post_me_password_is_escaped"),
    ),
    (
        "pages/own.rs",
        "PasswordFields",
        "password",
        Cover::Corpus("xss_corpus_password_field_of_post_me_password_is_escaped"),
    ),
    ("pages/own.rs", "PasswordFields", "csrf_token", CSRF),
    ("pages/security_events.rs", "EventsQuery", "kind", FILTERS),
    (
        "pages/security_events.rs",
        "EventsQuery",
        "login",
        Cover::Corpus(
            "approval_xss_login_filter_of_the_security_events_is_echoed_escaped_or_refused",
        ),
    ),
    (
        "pages/security_events.rs",
        "EventsQuery",
        "client_ip",
        FILTERS,
    ),
    ("pages/security_events.rs", "EventsQuery", "from", FILTERS),
    ("pages/security_events.rs", "EventsQuery", "to", FILTERS),
    ("pages/security_events.rs", "EventsQuery", "before", FILTERS),
    (
        "pages/users.rs",
        "CreateFields",
        "login",
        Cover::Corpus("xss_corpus_login_field_of_post_users_is_escaped"),
    ),
    ("pages/users.rs", "CreateFields", "role", ROLE),
    ("pages/users.rs", "CreateFields", "csrf_token", CSRF),
    ("pages/users.rs", "RoleFields", "role", ROLE),
    ("pages/users.rs", "RoleFields", "csrf_token", CSRF),
    ("pages/users.rs", "TokenFields", "csrf_token", CSRF),
    (
        "pages/approval/approve.rs",
        "ApproveFields",
        "row_version",
        NUMBER,
    ),
    (
        "pages/approval/approve.rs",
        "ApproveFields",
        "allow_outdated",
        NUMBER,
    ),
    (
        "pages/approval/approve.rs",
        "ApproveFields",
        "csrf_token",
        CSRF,
    ),
    ("pages/approval/preview.rs", "PreviewQuery", "kind", FILTERS),
    ("pages/approval/preview.rs", "PreviewQuery", "page", FILTERS),
    ("pages/approval/preview.rs", "PreviewQuery", "rv", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "aggregate", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "entity_type", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "entity_id", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "actor", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "action", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "from", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "to", FILTERS),
    ("pages/audit/list.rs", "AuditQuery", "before", FILTERS),
    (
        "pages/machines/commands.rs",
        "NameFields",
        "name",
        PLANT_RENAME,
    ),
    (
        "pages/machines/commands.rs",
        "NameFields",
        "csrf_token",
        CSRF,
    ),
    (
        "pages/machines/form.rs",
        "CreateFields",
        "code",
        PLANT_REFUSED,
    ),
    (
        "pages/machines/form.rs",
        "CreateFields",
        "name",
        PLANT_MACHINE_NAMES,
    ),
    (
        "pages/machines/form.rs",
        "CreateFields",
        "plc_platform",
        PLANT_REFUSED,
    ),
    (
        "pages/machines/form.rs",
        "CreateFields",
        "main_unit_name",
        PLANT_MACHINE_NAMES,
    ),
    ("pages/machines/form.rs", "CreateFields", "csrf_token", CSRF),
    (
        "pages/machines/list.rs",
        "ListQuery",
        "q",
        Cover::Corpus("plant_xss_search_text_of_the_machine_list_is_echoed_escaped"),
    ),
    ("pages/machines/list.rs", "ListQuery", "page", PLANT_GETS),
    (
        "pages/machines/list.rs",
        "ListQuery",
        "show_archived",
        PLANT_GETS,
    ),
    (
        "pages/machines/list.rs",
        "ListQuery",
        "archived",
        PLANT_GETS,
    ),
    (
        "pages/machines/page.rs",
        "CodeQuery",
        "archived",
        PLANT_GETS,
    ),
    ("pages/parameters/new.rs", "NewQuery", "unit", RECIPE_GETS),
    (
        "pages/parameters/table.rs",
        "TableQuery",
        "unit",
        RECIPE_GETS,
    ),
    (
        "pages/parameters/table.rs",
        "TableQuery",
        "page",
        RECIPE_GETS,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "TranslationFields",
        "row_version",
        RECIPE_REFUSED,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "TranslationFields",
        "name",
        RECIPE_TEXTS,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "TranslationFields",
        "description",
        RECIPE_TEXTS,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "TranslationFields",
        "csrf_token",
        CSRF,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "RowFields",
        "row_version",
        RECIPE_REFUSED,
    ),
    (
        "pages/parameters/translation_routes.rs",
        "RowFields",
        "csrf_token",
        CSRF,
    ),
    (
        "pages/recipes/form.rs",
        "CreateFields",
        "machine_code",
        RECIPE_REFUSED,
    ),
    (
        "pages/recipes/form.rs",
        "CreateFields",
        "reference_number",
        RECIPE_REFUSED,
    ),
    (
        "pages/recipes/form.rs",
        "CreateFields",
        "name",
        RECIPE_TEXTS,
    ),
    (
        "pages/recipes/form.rs",
        "CreateFields",
        "change_note",
        RECIPE_TEXTS,
    ),
    ("pages/recipes/form.rs", "CreateFields", "csrf_token", CSRF),
    (
        "pages/recipes/list.rs",
        "ListQuery",
        "q",
        Cover::Corpus("recipe_xss_search_text_of_the_recipe_list_is_echoed_escaped"),
    ),
    ("pages/recipes/list.rs", "ListQuery", "machine", RECIPE_GETS),
    ("pages/recipes/list.rs", "ListQuery", "status", RECIPE_GETS),
    ("pages/recipes/list.rs", "ListQuery", "page", RECIPE_GETS),
    (
        "pages/recipes/list.rs",
        "ListQuery",
        "show_deleted",
        RECIPE_GETS,
    ),
    ("pages/recipes/page.rs", "PageQuery", "page", RECIPE_GETS),
    (
        "pages/recipes/rename.rs",
        "NameFields",
        "name",
        RECIPE_TEXTS,
    ),
    ("pages/recipes/rename.rs", "NameFields", "csrf_token", CSRF),
    ("pages/types/form.rs", "FormQuery", "from", PLANT_GETS),
    ("pages/types/tree.rs", "TypeQuery", "rev", PLANT_GETS),
    ("pages/types/usage.rs", "UsageQuery", "rev", PLANT_GETS),
    ("pages/types/usage.rs", "UsageQuery", "page", PLANT_GETS),
    (
        "pages/units/add.rs",
        "AddFields",
        "parent_path",
        PLANT_UNIT_REFUSED,
    ),
    (
        "pages/units/add.rs",
        "AddFields",
        "segment",
        PLANT_UNIT_REFUSED,
    ),
    (
        "pages/units/add.rs",
        "AddFields",
        "name",
        Cover::Corpus("plant_xss_unit_name_is_shown_escaped_in_the_new_tree"),
    ),
    ("pages/units/add.rs", "AddFields", "csrf_token", CSRF),
    (
        "pages/units/commands.rs",
        "NameFields",
        "name",
        PLANT_RENAME,
    ),
    ("pages/units/commands.rs", "NameFields", "csrf_token", CSRF),
    ("pages/units/form.rs", "NewQuery", "parent_path", PLANT_GETS),
    (
        "pages/users/query.rs",
        "ListQuery",
        "q",
        Cover::Corpus("xss_corpus_search_field_of_get_users_is_escaped"),
    ),
    ("pages/users/query.rs", "ListQuery", "page", NUMBER),
    (
        "pages/versions/actions.rs",
        "VersionedFields",
        "row_version",
        NUMBER,
    ),
    (
        "pages/versions/actions.rs",
        "VersionedFields",
        "csrf_token",
        CSRF,
    ),
    (
        "pages/versions/compare.rs",
        "CompareQuery",
        "n",
        RECIPE_GETS,
    ),
    (
        "pages/versions/compare.rs",
        "CompareQuery",
        "m",
        RECIPE_GETS,
    ),
    (
        "pages/versions/form.rs",
        "CreateFields",
        "based_on",
        RECIPE_REFUSED,
    ),
    (
        "pages/versions/form.rs",
        "CreateFields",
        "change_note",
        RECIPE_TEXTS,
    ),
    ("pages/versions/form.rs", "CreateFields", "csrf_token", CSRF),
    (
        "pages/versions/page.rs",
        "VersionQuery",
        "unit",
        RECIPE_GETS,
    ),
    (
        "pages/versions/page.rs",
        "VersionQuery",
        "page",
        RECIPE_GETS,
    ),
];

// --- The scan of one source file ---------------------------------------------------------------------

/// A struct with a `Deserialize` derive that the scan read.
#[derive(Debug, PartialEq, Eq)]
struct ScannedStruct {
    name: String,
    /// The wire names of the fields in the order of the source.
    wire_names: Vec<String>,
}

/// The result of the scan of one source text.
#[derive(Debug, Default, PartialEq, Eq)]
struct Scan {
    /// The count of the `Deserialize` derives in the text, counted without the parser.
    derives: usize,
    structs: Vec<ScannedStruct>,
    faults: Vec<String>,
}

/// Pair each char of `text` with a flag that is true inside a string literal, quotes included.
fn mark_quotes(text: &str) -> Vec<(char, bool)> {
    let (mut quoted, mut escaped) = (false, false);
    text.chars()
        .map(|c| {
            let inside = quoted || c == '"';
            match c {
                _ if escaped => escaped = false,
                '\\' if quoted => escaped = true,
                '"' => quoted = !quoted,
                _ => {}
            }
            (c, inside)
        })
        .collect()
}

/// Return true when the attribute text has a `[` that no `]` closes.
fn is_open(text: &str) -> bool {
    let mut depth = 0_usize;
    for (c, quoted) in mark_quotes(text) {
        match c {
            '[' if !quoted => depth = depth.saturating_add(1),
            ']' if !quoted => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth > 0
}

/// Read an attribute that starts with `first` and can go over several lines.
fn read_attribute<'a>(first: &str, rest: &mut impl Iterator<Item = &'a str>) -> String {
    let mut text = first.to_owned();
    while is_open(&text) {
        let Some(next) = rest.next() else { break };
        text.push(' ');
        text.push_str(next);
    }
    text
}

/// Return the identifier words of a text.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
}

/// Return true when the attribute is a `derive` that names `Deserialize`.
fn is_deserialize_derive(attr: &str) -> bool {
    attr.starts_with("#[derive(") && words(attr).any(|word| word == "Deserialize")
}

/// Count the `derive(...)` lists of the text that name `Deserialize`. The count needs no parser.
fn count_derives(source: &str) -> usize {
    source
        .match_indices("derive(")
        .filter_map(|(at, _)| source.get(at..)?.split_once(')'))
        .filter(|(list, _)| words(list).any(|word| word == "Deserialize"))
        .count()
}

/// Remove `pub`, `pub(crate)`, `pub(super)`, or `pub(in path)` from the start of a line.
fn strip_visibility(line: &str) -> &str {
    let Some(rest) = line.strip_prefix("pub") else {
        return line;
    };
    if let Some(inner) = rest.strip_prefix('(') {
        return inner
            .split_once(')')
            .map_or(line, |(_, after)| after.trim_start());
    }
    if rest.starts_with(char::is_whitespace) {
        return rest.trim_start();
    }
    line
}

/// Split the content of a `#[serde(...)]` list at its top-level commas.
fn split_top_level(text: &str) -> Vec<String> {
    let (mut parts, mut current, mut depth) = (Vec::new(), String::new(), 0_usize);
    for (c, quoted) in mark_quotes(text) {
        match c {
            '(' if !quoted => depth = depth.saturating_add(1),
            ')' if !quoted => depth = depth.saturating_sub(1),
            ',' if !quoted && depth == 0 => {
                parts.push(std::mem::take(&mut current).trim().to_owned());
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    parts.push(current.trim().to_owned());
    parts
}

/// Read one item of a `#[serde(...)]` list. A list item such as `rename(...)` has no value.
fn parse_item(item: &str) -> SerdeItem {
    match item.split_once('=') {
        Some((key, value)) if !key.contains('(') => (
            key.trim().to_owned(),
            Some(value.trim().trim_matches('"').to_owned()),
        ),
        _list_or_flag => (
            item.split('(').next().unwrap_or_default().trim().to_owned(),
            None,
        ),
    }
}

/// Return the items of every `#[serde(...)]` attribute in the list.
fn serde_items(attrs: &[String]) -> Vec<SerdeItem> {
    attrs
        .iter()
        .filter_map(|attr| {
            attr.trim()
                .strip_prefix("#[serde(")?
                .strip_suffix(")]")
                .map(str::to_owned)
        })
        .flat_map(|list| split_top_level(&list))
        .filter(|item| !item.is_empty())
        .map(|item| parse_item(&item))
        .collect()
}

/// Return one fault for each key that changes the wire names of `what`.
///
/// `rename_all`, `flatten`, and `alias` change them in a way that the table does not list. A
/// `rename` without a plain value has the list form, which the guard does not read.
fn key_faults(items: &[SerdeItem], what: &str) -> Vec<String> {
    items
        .iter()
        .filter(|(key, value)| {
            key == "flatten"
                || key == "alias"
                || key.starts_with("rename_all")
                || (key == "rename" && value.is_none())
        })
        .map(|(key, _value)| format!("{what} uses the serde key `{key}`, which changes wire names"))
        .collect()
}

/// Return the name of a struct and the text after its `{`, or `None` for another item.
fn struct_head(line: &str) -> Option<(String, &str)> {
    let rest = strip_visibility(line).strip_prefix("struct ")?;
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    let (name, tail) = rest.split_at(end);
    let (before, after) = tail.split_once('{')?;
    (!before.contains([';', '('])).then(|| (name.to_owned(), after))
}

/// Return the name of the field on the line, or `None` when the line is no field.
fn field_name(line: &str) -> Option<String> {
    let (name, tail) = strip_visibility(line).split_once(':')?;
    let name = name.trim().trim_start_matches("r#");
    let is_name = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_');
    (is_name && !tail.starts_with(':')).then(|| name.to_owned())
}

/// Read the fields up to the closing brace. Each field comes with its `serde` items.
fn read_fields<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Vec<(String, Vec<SerdeItem>)> {
    let mut fields = Vec::new();
    let mut attrs: Vec<String> = Vec::new();
    while let Some(line) = lines.next() {
        if line.starts_with('}') {
            break;
        }
        if line.starts_with("#[") {
            attrs.push(read_attribute(line, lines));
        } else if let Some(name) = field_name(line) {
            fields.push((name, serde_items(&attrs)));
            attrs.clear();
        }
    }
    fields
}

/// Return the wire name of a field: the value of `rename`, else the name of the field.
fn wire_name(field: &str, items: &[SerdeItem]) -> String {
    items
        .iter()
        .find(|(key, _value)| key == "rename")
        .and_then(|(_key, value)| value.clone())
        .unwrap_or_else(|| field.to_owned())
}

/// Read the item after a `Deserialize` derive into the scan.
fn scan_item<'a>(
    line: &str,
    attrs: &[String],
    lines: &mut impl Iterator<Item = &'a str>,
    scan: &mut Scan,
) {
    let Some((name, after)) = struct_head(line) else {
        let fault =
            format!("a Deserialize derive on `{line}`, which is no struct with named fields");
        scan.faults.push(fault);
        return;
    };
    let items = serde_items(attrs);
    if !items
        .iter()
        .any(|(key, _value)| key == "deny_unknown_fields")
    {
        scan.faults
            .push(format!("{name} has no deny_unknown_fields"));
    }
    scan.faults.extend(key_faults(&items, &name));
    let fields = match after.trim() {
        "}" => Vec::new(),
        "" => read_fields(lines),
        other => {
            scan.faults.push(format!(
                "{name} has fields on the line of the item: `{other}`"
            ));
            return;
        }
    };
    let mut wire_names = Vec::new();
    for (field, field_items) in &fields {
        scan.faults
            .extend(key_faults(field_items, &format!("{name}.{field}")));
        wire_names.push(wire_name(field, field_items));
    }
    scan.structs.push(ScannedStruct { name, wire_names });
}

/// Scan one source text for the structs with a `Deserialize` derive.
///
/// The function solves one problem: the guard needs the wire names of each request struct, and a
/// trait or a macro on each struct would need a change of the production code (design D4). So the
/// scan reads the lines. An attribute or a doc comment before the item can come in any order.
fn scan_source(source: &str) -> Scan {
    let mut scan = Scan {
        derives: count_derives(source),
        ..Scan::default()
    };
    let mut lines = source.lines().map(str::trim);
    let mut attrs: Vec<String> = Vec::new();
    while let Some(line) = lines.next() {
        if line.starts_with("#[") {
            attrs.push(read_attribute(line, &mut lines));
        } else if !line.is_empty() && !line.starts_with("//") {
            if attrs.iter().any(|attr| is_deserialize_derive(attr)) {
                scan_item(line, &attrs, &mut lines, &mut scan);
            }
            attrs.clear();
        }
    }
    scan
}

// --- The comparison with the table -------------------------------------------------------------------

/// One source file of the crate: its path below `src/` with `/`, and its text.
struct Source {
    relative: String,
    text: String,
}

/// Read every `.rs` file below the folder.
fn read_sources(folder: &Path) -> Vec<Source> {
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

/// Return true when a test file of `src/` has `fn name(`. This file does not count.
fn has_test_fn(sources: &[Source], name: &str) -> bool {
    let needle = format!("fn {name}(");
    sources
        .iter()
        .filter(|source| is_test_source(source) && source.relative != SELF_FILE)
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
fn coverage_faults(sources: &[Source], table: &[Entry]) -> Vec<String> {
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

#[test]
fn xss_coverage_every_deserialized_field_has_a_corpus_test_or_a_reason() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let sources = read_sources(&folder);

    let faults = coverage_faults(&sources, TABLE);

    assert!(
        faults.is_empty(),
        "the XSS corpus does not cover every field ({} faults):\n{}",
        faults.len(),
        faults.join("\n")
    );
}

// --- The tests of the scan and of the comparison ----------------------------------------------------

/// A source with each shape that the scan must read: doc comments, blank lines, a derive with
/// other traits, an attribute with more items, an attribute over three lines, and a `rename` whose
/// value holds a comma and a bracket.
const SAMPLE: &str = r#"
use serde::Deserialize;

/// The fields of the form.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename = "x")]
pub(crate) struct Sample {
    /// The login.
    login: String,

    /// The session token.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
    #[serde(
        default,
        rename = "a,b)]"
    )]
    pub(super) odd: String,
}
"#;

#[test]
fn xss_coverage_scan_reads_the_wire_names_of_a_struct() {
    let scan = scan_source(SAMPLE);

    assert_eq!(scan.derives, 1);
    assert_eq!(scan.faults, Vec::<String>::new());
    let wanted = ScannedStruct {
        name: "Sample".to_owned(),
        wire_names: vec![
            "login".to_owned(),
            "csrf_token".to_owned(),
            "a,b)]".to_owned(),
        ],
    };
    assert_eq!(scan.structs, [wanted]);
}

#[test]
fn xss_coverage_scan_reads_a_serde_attribute_before_the_derive_and_a_path_derive() {
    let source =
        "#[serde(deny_unknown_fields)]\n#[derive(serde::Deserialize)]\nstruct A {\n    a: u8,\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan.derives, 1);
    assert_eq!(scan.faults, Vec::<String>::new());
    assert_eq!(scan.structs.len(), 1);
}

#[test]
fn xss_coverage_scan_reads_a_struct_with_empty_braces() {
    let source =
        "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\npub(crate) struct NoQuery {}\n";

    let scan = scan_source(source);

    assert_eq!(scan.faults, Vec::<String>::new());
    assert_eq!(scan.structs.len(), 1);
    assert_eq!(
        scan.structs.first().unwrap().wire_names,
        Vec::<String>::new()
    );
}

#[test]
fn xss_coverage_scan_ignores_a_struct_without_a_deserialize_derive() {
    let source =
        "use serde::Deserialize;\n#[derive(Debug, Serialize)]\nstruct A {\n    a: u8,\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan, Scan::default());
}

#[test]
fn xss_coverage_scan_refuses_each_key_that_changes_a_wire_name() {
    let keys = [
        ("rename_all = \"camelCase\"", "rename_all"),
        ("flatten", "flatten"),
        ("alias = \"b\"", "alias"),
        ("rename(deserialize = \"b\")", "rename"),
    ];
    for (item, key) in keys {
        let head = "#[derive(Deserialize)]\n";
        let on_struct =
            format!("{head}#[serde(deny_unknown_fields, {item})]\nstruct A {{\n    a: u8,\n}}\n");
        let on_field = format!(
            "{head}#[serde(deny_unknown_fields)]\nstruct A {{\n    #[serde(default, {item})]\n    a: u8,\n}}\n"
        );
        for source in [on_struct, on_field] {
            let scan = scan_source(&source);

            assert_eq!(scan.faults.len(), 1, "{item}: {:?}", scan.faults);
            let fault = scan.faults.first().unwrap();
            assert!(fault.contains(&format!("`{key}`")), "{item}: {fault}");
        }
    }
}

#[test]
fn xss_coverage_scan_refuses_a_struct_without_deny_unknown_fields() {
    let source = "#[derive(Deserialize)]\n#[serde(default)]\nstruct A {\n    a: u8,\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan.faults, ["A has no deny_unknown_fields"]);
}

#[test]
fn xss_coverage_scan_refuses_an_item_that_is_no_struct_with_named_fields() {
    let items = [
        "enum A {\n    B,\n}\n",
        "struct A(String);\n",
        "struct A;\n",
        "pub(crate) struct A { a: u8 }\n",
    ];
    for item in items {
        let source = format!("#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\n{item}");

        let scan = scan_source(&source);

        assert_eq!(scan.faults.len(), 1, "{item}: {:?}", scan.faults);
        assert_eq!(scan.derives, 1, "{item}");
        assert_eq!(scan.structs.len(), 0, "{item}");
    }
}

/// Build a source file for the comparison tests.
fn source(relative: &str, text: &str) -> Source {
    Source {
        relative: relative.to_owned(),
        text: text.to_owned(),
    }
}

/// A production file with one request struct of one field.
const REQUEST: &str =
    "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    text: String,\n}\n";

#[test]
fn xss_coverage_comparison_passes_when_a_corpus_test_covers_the_field() {
    let sources = [
        source("pages/form.rs", REQUEST),
        source("pages/form_tests.rs", "async fn xss_covers_text() {}"),
    ];
    let table: &[Entry] = &[(
        "pages/form.rs",
        "Form",
        "text",
        Cover::Corpus("xss_covers_text"),
    )];

    assert_eq!(coverage_faults(&sources, table), Vec::<String>::new());
}

#[test]
fn xss_coverage_comparison_names_a_field_without_an_entry() {
    let sources = [source("pages/form.rs", REQUEST)];

    let faults = coverage_faults(&sources, &[]);

    assert_eq!(faults, ["field without an entry: pages/form.rs Form.text"]);
}

#[test]
fn xss_coverage_comparison_names_an_entry_without_a_field() {
    let sources = [source("pages/form.rs", REQUEST)];
    let table: &[Entry] = &[
        ("pages/form.rs", "Form", "text", Cover::NoText("A reason.")),
        ("pages/form.rs", "Form", "gone", Cover::NoText("A reason.")),
    ];

    let faults = coverage_faults(&sources, table);

    assert_eq!(faults, ["entry without a field: pages/form.rs Form.gone"]);
}

#[test]
fn xss_coverage_comparison_names_a_table_that_lists_a_field_twice() {
    let sources = [source("pages/form.rs", REQUEST)];
    let entry: Entry = ("pages/form.rs", "Form", "text", Cover::NoText("A reason."));

    let faults = coverage_faults(&sources, &[entry, entry]);

    assert_eq!(faults, ["the table lists a field twice"]);
}

#[test]
fn xss_coverage_comparison_names_a_corpus_test_that_has_no_fn() {
    let sources = [
        source("pages/form.rs", REQUEST),
        source("pages/form_tests.rs", "async fn xss_other_name() {}"),
        source(SELF_FILE, "fn xss_covers_text() {}"),
    ];
    let table: &[Entry] = &[(
        "pages/form.rs",
        "Form",
        "text",
        Cover::Corpus("xss_covers_text"),
    )];

    let faults = coverage_faults(&sources, table);

    assert_eq!(
        faults,
        ["the corpus test of pages/form.rs Form.text has no fn: xss_covers_text"]
    );
}

#[test]
fn xss_coverage_comparison_names_a_reason_entry_without_a_reason() {
    let sources = [source("pages/form.rs", REQUEST)];
    let table: &[Entry] = &[("pages/form.rs", "Form", "text", Cover::NoText("  "))];

    let faults = coverage_faults(&sources, table);

    assert_eq!(
        faults,
        ["the entry pages/form.rs Form.text has an empty reason"]
    );
}

#[test]
fn xss_coverage_comparison_skips_the_structs_of_test_files() {
    let sources = [source("pages/form_tests.rs", REQUEST)];

    assert_eq!(coverage_faults(&sources, &[]), Vec::<String>::new());
}

#[test]
fn xss_coverage_comparison_names_a_count_mismatch() {
    let sources = [source(
        "pages/form.rs",
        "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nenum Form {\n    A,\n}\n",
    )];

    let faults = coverage_faults(&sources, &[]);

    assert!(
        faults
            .iter()
            .any(|fault| fault.contains("1 Deserialize derives, but 0 parsed structs")),
        "{faults:?}"
    );
}
