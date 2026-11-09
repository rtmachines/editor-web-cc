//! The scan of one source file: the wire names of each `Deserialize` struct.

/// One item of a `#[serde(...)]` attribute: the key and the value of `key = "value"`.
type SerdeItem = (String, Option<String>);

/// A struct with a `Deserialize` derive that the scan read.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ScannedStruct {
    pub(super) name: String,
    /// The wire names of the fields in the order of the source.
    pub(super) wire_names: Vec<String>,
}

/// The result of the scan of one source text.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Scan {
    /// The count of the `Deserialize` derives in the text, counted without the parser.
    pub(super) derives: usize,
    pub(super) structs: Vec<ScannedStruct>,
    pub(super) faults: Vec<String>,
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
fn read_attribute<S: AsRef<str>>(first: &str, rest: &mut impl Iterator<Item = S>) -> String {
    let mut text = first.to_owned();
    while is_open(&text) {
        let Some(next) = rest.next() else { break };
        text.push(' ');
        text.push_str(next.as_ref());
    }
    text
}

/// Return the text after the `]` that closes the first `[` of the attribute. A `]` inside a string
/// closes nothing. The result is empty when the attribute ends the text or never closes.
fn after_attribute(attr: &str) -> &str {
    let (mut depth, mut end) = (0_usize, 0_usize);
    for (c, quoted) in mark_quotes(attr) {
        end = end.saturating_add(c.len_utf8());
        match c {
            '[' if !quoted => depth = depth.saturating_add(1),
            ']' if !quoted => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return attr.get(end..).unwrap_or_default().trim();
                }
            }
            _ => {}
        }
    }
    ""
}

/// Return a fault when text other than a `//` comment follows the attribute on its line.
///
/// Such text can hide a second attribute, for example a `serde` list after a `derive`. The scan
/// reads one attribute for each line, so it would lose that list.
fn tail_fault(what: &str, attr: &str) -> Option<String> {
    let tail = after_attribute(attr);
    (!tail.is_empty() && !tail.starts_with("//"))
        .then(|| format!("{what} has text after an attribute on one line: `{attr}`"))
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

/// Return one fault for each `use` that renames `Deserialize` with `as`, also inside a group.
///
/// A renamed derive such as `#[derive(Decode)]` hides its struct from the parser and from
/// `count_derives`. So the guard refuses the rename itself. The check reads no derive.
fn rename_faults(source: &str) -> Vec<String> {
    let mut faults = Vec::new();
    let mut lines = source.lines().enumerate();
    while let Some((index, line)) = lines.next() {
        let line = line.trim();
        if !strip_visibility(line).starts_with("use ") {
            continue;
        }
        let mut statement = line.to_owned();
        while !statement.contains(';') {
            let Some((_, next)) = lines.next() else { break };
            statement.push(' ');
            statement.push_str(next.trim());
        }
        let all: Vec<&str> = words(&statement).collect();
        if all.windows(2).any(|pair| pair == ["Deserialize", "as"]) {
            let number = index.saturating_add(1);
            faults.push(format!(
                "line {number}: a `use` renames Deserialize with `as`"
            ));
        }
    }
    faults
}

/// Return true when the text has a bracket that opens and every bracket closes.
fn closes_all_brackets(text: &str) -> bool {
    let (mut depth, mut opened) = (0_usize, false);
    for (c, quoted) in mark_quotes(text) {
        match c {
            '{' | '(' | '[' if !quoted => {
                depth = depth.saturating_add(1);
                opened = true;
            }
            '}' | ')' | ']' if !quoted => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    opened && depth == 0
}

/// Return one fault for each `macro_rules!` body that holds a `Deserialize` derive.
///
/// A macro can make a struct whose name and fields are `$` fragments. The guard cannot read the
/// fields of each invocation, so it refuses the body.
fn macro_faults(source: &str) -> Vec<String> {
    let mut faults = Vec::new();
    let mut lines = source.lines().enumerate();
    while let Some((index, line)) = lines.next() {
        if !line.trim().starts_with("macro_rules!") {
            continue;
        }
        let mut body = line.to_owned();
        while !closes_all_brackets(&body) {
            let Some((_, next)) = lines.next() else { break };
            body.push('\n');
            body.push_str(next);
        }
        let number = index.saturating_add(1);
        if body.contains("derive(") && words(&body).any(|word| word == "Deserialize") {
            faults.push(format!(
                "line {number}: a macro_rules body holds a Deserialize derive"
            ));
        }
        if derive_lists(&body).any(|list| list.contains('$')) {
            let name: String = line
                .trim()
                .trim_start_matches("macro_rules!")
                .trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            faults.push(format!(
                "line {number}: macro_rules! {name} builds a derive list from a metavariable, \
                 and the scan cannot read it"
            ));
        }
    }
    faults
}

/// Yield the text inside each `derive(...)` list of `text`, up to the matching bracket.
///
/// A list that never closes runs to the end of the text, so the caller still sees its content.
fn derive_lists(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices("derive(").map(move |(start, marker)| {
        let rest = text.get(start.saturating_add(marker.len())..).unwrap_or("");
        let mut depth = 1_usize;
        let end = rest
            .char_indices()
            .find(|&(_, c)| {
                match c {
                    '(' => depth = depth.saturating_add(1),
                    ')' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                depth == 0
            })
            .map_or(rest.len(), |(offset, _)| offset);
        rest.get(..end).unwrap_or("")
    })
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

/// Return one fault for each `cfg_attr` attribute that wraps `serde(...)`.
///
/// `serde_items` reads only a plain `#[serde(...)]`. A `cfg_attr` can hide a key such as
/// `rename_all`, so the scan refuses it and does not guess the condition.
fn cfg_attr_faults(attrs: &[String], what: &str) -> Vec<String> {
    attrs
        .iter()
        .map(|attr| attr.trim())
        .filter(|attr| attr.starts_with("#[cfg_attr(") && attr.contains("serde("))
        .map(|attr| {
            format!("{what} wraps serde in a cfg_attr, which the scan does not read: `{attr}`")
        })
        .collect()
}

/// Return the name of a struct and the text after its `{`, or `None` for another item.
///
/// An empty name, such as the `$name` of a macro body, gives `None`. The caller then gives a fault.
fn struct_head(line: &str) -> Option<(String, &str)> {
    let rest = strip_visibility(line).strip_prefix("struct ")?;
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    let (name, tail) = rest.split_at(end);
    if name.is_empty() {
        return None;
    }
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

/// Return the text after the comma that ends the first field of the line, or an empty text.
///
/// A comma inside a string, a bracket, or an angle bracket, such as `HashMap<String, String>` or
/// `(u8, u8)`, ends no field. A `//` comment ends the scan.
fn after_field(line: &str) -> &str {
    let (_, ty) = strip_visibility(line).split_once(':').unwrap_or_default();
    let (mut depth, mut end, mut previous) = (0_usize, 0_usize, ' ');
    for (c, quoted) in mark_quotes(ty) {
        end = end.saturating_add(c.len_utf8());
        match c {
            '<' | '(' | '[' | '{' if !quoted => depth = depth.saturating_add(1),
            '>' if !quoted && previous != '-' => depth = depth.saturating_sub(1),
            ')' | ']' | '}' if !quoted => depth = depth.saturating_sub(1),
            '/' if !quoted && previous == '/' => return "",
            ',' if !quoted && depth == 0 => return ty.get(end..).unwrap_or_default().trim(),
            _ => {}
        }
        previous = c;
    }
    ""
}

/// Return a fault when text other than a `//` comment follows the first field on its line.
///
/// Such text can hide a second field or an attribute. The scan reads one field for each line, so it
/// would lose them.
fn field_tail_fault(name: &str, line: &str) -> Option<String> {
    let tail = after_field(line);
    (!tail.is_empty() && !tail.starts_with("//"))
        .then(|| format!("{name} has text after a field on one line: `{line}`"))
}

/// Remove the block comments from a line and trim it. `depth` carries the open comments from the
/// previous line into this one. A `/*` inside a string or after a `//` starts no comment.
fn strip_block_comments(line: &str, depth: &mut usize) -> String {
    let mut kept = String::new();
    let (mut quoted, mut escaped) = (false, false);
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        let at_marker = |next: char, rest: &std::str::Chars<'_>| {
            !quoted && c == '/' && rest.clone().next() == Some(next)
        };
        if *depth > 0 {
            if c == '*' && chars.clone().next() == Some('/') {
                chars.next();
                *depth = depth.saturating_sub(1);
            } else if c == '/' && chars.clone().next() == Some('*') {
                chars.next();
                *depth = depth.saturating_add(1);
            }
        } else if at_marker('*', &chars) {
            chars.next();
            *depth = depth.saturating_add(1);
        } else if at_marker('/', &chars) {
            kept.push(c);
            kept.extend(chars.by_ref());
        } else {
            match c {
                _ if escaped => escaped = false,
                '\\' if quoted => escaped = true,
                '"' => quoted = !quoted,
                _ => {}
            }
            kept.push(c);
        }
    }
    kept.trim().to_owned()
}

/// Read the fields up to the closing brace. Each field comes with its `serde` items.
///
/// The function fails closed. A line that is no field, attribute, comment, or blank line gives a
/// fault that names the struct and the line. The caller adds the file.
fn read_fields<'a>(
    name: &str,
    lines: &mut impl Iterator<Item = &'a str>,
    faults: &mut Vec<String>,
) -> Vec<(String, Vec<SerdeItem>)> {
    let mut fields = Vec::new();
    let mut attrs: Vec<String> = Vec::new();
    let mut depth = 0_usize;
    let mut lines = lines.map(|line| strip_block_comments(line, &mut depth));
    while let Some(line) = lines.next() {
        if line.starts_with('}') {
            break;
        }
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if line.starts_with("#[") {
            let attr = read_attribute(&line, &mut lines);
            faults.extend(tail_fault(name, &attr));
            attrs.push(attr);
        } else if let Some(field) = field_name(&line) {
            faults.extend(field_tail_fault(name, &line));
            faults.extend(cfg_attr_faults(&attrs, &format!("{name}.{field}")));
            fields.push((field, serde_items(&attrs)));
            attrs.clear();
        } else {
            faults.push(format!("{name} has a line that is no field: `{line}`"));
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
    scan.faults.extend(cfg_attr_faults(attrs, &name));
    scan.faults
        .extend(attrs.iter().filter_map(|attr| tail_fault(&name, attr)));
    let fields = match after.trim() {
        "}" => Vec::new(),
        "" => read_fields(&name, lines, &mut scan.faults),
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
pub(super) fn scan_source(source: &str) -> Scan {
    let mut scan = Scan {
        derives: count_derives(source),
        ..Scan::default()
    };
    scan.faults.extend(rename_faults(source));
    scan.faults.extend(macro_faults(source));
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
