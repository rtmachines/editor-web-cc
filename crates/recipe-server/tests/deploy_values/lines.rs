//! A small reader for the shape of `compose.yaml`. It uses line and indent rules and no YAML crate.
//!
//! The file uses block style with two spaces per level. A full-line comment and a blank line do
//! not count. A line ending of CRLF counts as LF, because `str::lines` removes both.

/// The lines of a text without blank lines and without full-line comments.
pub fn code_lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim_end)
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .collect()
}

/// A shell line without its trailing comment. A `#` starts a comment only after a space and outside quotes.
pub fn without_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut after_space = true;
    for (index, c) in line.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c == '#' && after_space => return line.get(..index).unwrap_or(line),
            None if c == '"' || c == '\'' => quote = Some(c),
            None => {}
        }
        after_space = c.is_whitespace();
    }
    line
}

/// The number of leading white-space bytes. A blank line counts all its bytes.
pub fn indent_of(line: &str) -> usize {
    line.chars()
        .take_while(|c| c.is_whitespace())
        .map(char::len_utf8)
        .sum()
}

/// The value of a scalar: a quoted text without its quotes, or an unquoted text without a trailing
/// ` #` comment. A mark inside a quoted text stays. A closing quote that a word follows does not
/// end the text, so the whole text stays.
pub fn unquote(text: &str) -> &str {
    let trimmed = text.trim();
    for quote in ['"', '\''] {
        if let Some((inner, after)) = trimmed
            .strip_prefix(quote)
            .and_then(|rest| rest.split_once(quote))
        {
            let after = after.trim();
            if after.is_empty() || after.starts_with('#') {
                return inner;
            }
            return trimmed;
        }
    }
    trimmed.split(" #").next().unwrap_or(trimmed).trim_end()
}

/// The value of the first line `key: value` or `- key: value`.
pub fn field<'a>(lines: &[&'a str], key: &str) -> Option<&'a str> {
    lines.iter().find_map(|line| {
        let trimmed = line.trim_start();
        let trimmed = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        let rest = trimmed.strip_prefix(key)?.strip_prefix(':')?;
        Some(unquote(rest))
    })
}

/// Like `field`, but only a line with exactly this indent counts.
pub fn field_at<'a>(lines: &[&'a str], indent: usize, key: &str) -> Option<&'a str> {
    let at_indent: Vec<&'a str> = lines
        .iter()
        .copied()
        .filter(|line| indent_of(line) == indent)
        .collect();
    field(&at_indent, key)
}

/// The lines below the line `header` (trimmed) with this indent, up to the next line with at most this indent.
pub fn section<'a>(lines: &[&'a str], indent: usize, header: &str) -> Vec<&'a str> {
    let mut found = false;
    let mut out = Vec::new();
    for line in lines {
        let level = indent_of(line);
        if found {
            if level <= indent {
                break;
            }
            out.push(*line);
        } else if level == indent && line.trim() == header {
            found = true;
        }
    }
    out
}

/// The items of a block list, without the dash and without quotes.
pub fn list_items<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    lines
        .iter()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .map(unquote)
        .collect()
}

/// The items of a flow list such as `["CMD", "wget"]`. An item holds no comma.
/// Only the literal `[]` gives an empty list. Any other empty item stays as an empty text.
pub fn flow_list(text: &str) -> Option<Vec<String>> {
    let text = text.trim();
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    if text == "[]" {
        return Some(Vec::new());
    }
    Some(
        inner
            .split(',')
            .map(|item| unquote(item).to_owned())
            .collect(),
    )
}

/// One service block of the file.
pub struct Service<'a> {
    pub name: &'a str,
    pub lines: Vec<&'a str>,
}

const SERVICE_KEY_INDENT: usize = 4;
const SUB_KEY_INDENT: usize = 6;
/// The item that `Service::list` gives for a key that has no value and no readable item.
pub const UNREADABLE_ITEM: &str = "<unreadable>";

impl<'a> Service<'a> {
    /// A value on a key of the service.
    pub fn scalar(&self, key: &str) -> Option<&'a str> {
        field_at(&self.lines, SERVICE_KEY_INDENT, key)
    }

    /// The lines below a key of the service.
    pub fn section(&self, key: &str) -> Vec<&'a str> {
        section(&self.lines, SERVICE_KEY_INDENT, &format!("{key}:"))
    }

    /// The items of a block list on a key of the service.
    pub fn items(&self, key: &str) -> Vec<&'a str> {
        list_items(&self.section(key))
    }

    /// The items of a block sequence that has the indent of its key, such as `cap_add:` and then `- A`.
    fn indentless_items(&self, key: &str) -> Vec<&'a str> {
        let header = format!("{key}:");
        let below: Vec<&'a str> = self
            .lines
            .iter()
            .copied()
            .skip_while(|line| !(indent_of(line) == SERVICE_KEY_INDENT && line.trim() == header))
            .skip(1)
            .take_while(|line| {
                indent_of(line) == SERVICE_KEY_INDENT && line.trim_start().starts_with("- ")
            })
            .collect();
        list_items(&below)
    }

    /// The items of a list on a key of the service, in block style, in indentless block style, or in
    /// flow style such as `[A, B]`. A single value that is no list counts as one item.
    /// Only the literal `[]` gives an empty list. A key with no value and no readable item gives
    /// the item `UNREADABLE_ITEM`, so a comparison reports a fault and no spelling hides an item.
    pub fn list(&self, key: &str) -> Vec<String> {
        let block = self.items(key);
        if !block.is_empty() {
            return block.into_iter().map(String::from).collect();
        }
        let Some(text) = self.scalar(key) else {
            return Vec::new();
        };
        if let Some(items) = flow_list(text) {
            return items;
        }
        if !text.is_empty() {
            return vec![text.to_owned()];
        }
        let indentless = self.indentless_items(key);
        if indentless.is_empty() {
            return vec![UNREADABLE_ITEM.to_owned()];
        }
        indentless.into_iter().map(String::from).collect()
    }

    /// The merge-key lines (`<<: ...`) of the service. This reader cannot follow a merge.
    pub fn merge_lines(&self) -> Vec<&'a str> {
        self.lines
            .iter()
            .copied()
            .filter(|line| line.trim_start().starts_with("<<:"))
            .collect()
    }

    /// The value of one variable of `environment`.
    pub fn env(&self, name: &str) -> Option<&'a str> {
        field_at(&self.section("environment"), SUB_KEY_INDENT, name)
    }

    /// The value of one key of `healthcheck`.
    pub fn health(&self, key: &str) -> Option<&'a str> {
        field_at(&self.section("healthcheck"), SUB_KEY_INDENT, key)
    }

    /// The names under `networks`, in block list style, in map style, or in flow list style, sorted.
    /// A value on the key that is no flow list gives the item `UNREADABLE_ITEM`, so no spelling hides a network.
    pub fn networks(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .section("networks")
            .into_iter()
            .filter(|line| indent_of(line) == SUB_KEY_INDENT)
            .map(|line| {
                let trimmed = line.trim();
                let name = trimmed.strip_prefix("- ").unwrap_or(trimmed);
                name.split(':').next().unwrap_or(name).to_owned()
            })
            .collect();
        if let Some(text) = self.scalar("networks").filter(|text| !text.is_empty()) {
            names.extend(flow_list(text).unwrap_or_else(|| vec![UNREADABLE_ITEM.to_owned()]));
        }
        names.sort();
        names
    }
}

/// The parsed file: its code lines and its services.
pub struct Compose<'a> {
    pub all: Vec<&'a str>,
    pub services: Vec<Service<'a>>,
}

impl<'a> Compose<'a> {
    /// Split the text into services.
    pub fn parse(text: &'a str) -> Compose<'a> {
        let all = code_lines(text);
        let mut services: Vec<Service<'a>> = Vec::new();
        let mut in_services = false;
        for line in &all {
            let level = indent_of(line);
            let trimmed = line.trim();
            if level == 0 {
                in_services = trimmed == "services:";
            } else if !in_services {
                continue;
            } else if level == 2 && trimmed.ends_with(':') {
                services.push(Service {
                    name: trimmed.trim_end_matches(':'),
                    lines: Vec::new(),
                });
            } else if let Some(service) = services.last_mut() {
                service.lines.push(line);
            }
        }
        Compose { all, services }
    }

    /// The service with this name.
    pub fn service(&self, name: &str) -> Option<&Service<'a>> {
        self.services.iter().find(|service| service.name == name)
    }
}
