//! Source scan of the out-of-band forms (design D7, ADR-0041 (e)).
//!
//! An out-of-band form replaces itself in the page. A submit button without a fixed `id` loses
//! the focus in that swap. The scan reads the template sources, finds each `<form` start tag
//! that holds `hx-swap-oob`, and checks every submit button of the form. A button counts when the
//! form renders it through a macro of an imported template, a macro of the same template, or an
//! `{% include %}`. The scan expands these calls recursively. A call that the scan cannot resolve
//! fails the test.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// The forms that hold `hx-swap-oob`, as the template file of each form (design D7, table).
const EXPECTED_FORM_FILES: [&str; 17] = [
    "approve.html",
    "machine-connection.html",
    "machine-new.html",
    "machine.html",
    "machine.html",
    "parameter-form.html",
    "password.html",
    "rebind-form.html",
    "recipe-new.html",
    "recipe.html",
    "translation-row.html",
    "udt-form.html",
    "unit-form.html",
    "unit.html",
    "user-new.html",
    "user-role-form.html",
    "version-new.html",
];

fn templates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("templates")
}

/// Collect the template files below `dir`, except the directory `test`.
fn template_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "test") {
                template_files(&path, found);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension == "html")
        {
            found.push(path);
        }
    }
}

/// Split `text`, which starts with `<`, into its start tag and the text after the tag. A `>`
/// inside `{{ }}` or `{% %}` does not end the tag. The tag can span several lines.
fn split_tag(text: &str) -> Option<(&str, &str)> {
    let mut rest = text;
    loop {
        if let Some(after) = rest.strip_prefix("{{") {
            rest = after.split_once("}}")?.1;
        } else if let Some(after) = rest.strip_prefix("{%") {
            rest = after.split_once("%}")?.1;
        } else if let Some(after) = rest.strip_prefix('>') {
            return Some((text.strip_suffix(after)?, after));
        } else {
            let mut chars = rest.chars();
            chars.next()?;
            rest = chars.as_str();
        }
    }
}

/// Return each start tag of the element `<name` in `text` with the text after that tag.
fn start_tags<'a>(text: &'a str, name: &str) -> Vec<(&'a str, &'a str)> {
    let open = format!("<{name}");
    let mut tags = Vec::new();
    let mut rest = text;
    while let Some(index) = rest.find(&open) {
        let from = &rest[index..];
        let (tag, after) = split_tag(from).expect("start tag has no end");
        let next = from[open.len()..].chars().next();
        if next.is_some_and(|c| c.is_whitespace() || c == '>') {
            tags.push((tag, after));
        }
        rest = after;
    }
    tags
}

/// Return true when `tag` is a submit button: a `<button` that is not of type `button` or
/// `reset`, or an `<input type="submit">`.
fn is_submit(tag: &str) -> bool {
    if tag.starts_with("<input") {
        return tag.contains("type=\"submit\"");
    }
    !tag.contains("type=\"button\"") && !tag.contains("type=\"reset\"")
}

/// Return the text of a template tag between `{%` and `%}` without the whitespace control
/// characters `-`, `+`, `~` and without the surrounding white space.
fn tag_body(inner: &str) -> &str {
    inner
        .trim_matches(|c| c == '-' || c == '+' || c == '~')
        .trim()
}

/// Return the text of `tag` that always renders. A `{{ }}` expression becomes the letter `X`. The
/// text inside a `{% %}` block, such as `{% if %}` to `{% endif %}`, is dropped, because the block
/// can leave it out.
fn unconditional_text(tag: &str) -> String {
    let mut text = String::new();
    let mut depth = 0_usize;
    let mut rest = tag;
    loop {
        if let Some(after) = rest.strip_prefix("{{") {
            let Some((_, next)) = after.split_once("}}") else {
                break;
            };
            if depth == 0 {
                text.push('X');
            }
            rest = next;
        } else if let Some(after) = rest.strip_prefix("{%") {
            let Some((inner, next)) = after.split_once("%}") else {
                break;
            };
            depth = block_depth(depth, tag_body(inner));
            rest = next;
        } else {
            let mut chars = rest.chars();
            let Some(c) = chars.next() else {
                break;
            };
            if depth == 0 {
                text.push(c);
            }
            rest = chars.as_str();
        }
    }
    text
}

/// Return the block depth after the template tag with the body `body`.
fn block_depth(depth: usize, body: &str) -> usize {
    let word = body.split_whitespace().next().unwrap_or_default();
    if word.starts_with("end") {
        depth.saturating_sub(1)
    } else if ["if", "for", "match", "with", "block", "filter", "call"].contains(&word) {
        depth.saturating_add(1)
    } else {
        depth
    }
}

/// Return true when `tag` has an `id` attribute with a value that is not empty and that no
/// `{% %}` block makes conditional.
fn has_fixed_id(tag: &str) -> bool {
    let text = unconditional_text(tag);
    text.match_indices("id=\"").any(|(index, _)| {
        let (head, tail) = text.split_at(index);
        head.ends_with(char::is_whitespace)
            && tail
                .strip_prefix("id=\"")
                .and_then(|value| value.split_once('"'))
                .is_some_and(|(value, _)| !value.trim().is_empty())
    })
}

/// One template source: its macros, its imports, and the text outside the macros.
struct Template {
    /// The macro name and the macro body.
    macros: BTreeMap<String, String>,
    /// The alias of an import line and the template path of that line.
    imports: BTreeMap<String, String>,
    /// The source without the macro definitions. An include renders this text.
    outside: String,
}

/// Return the text after `keyword` when `body` starts with the word `keyword`.
fn after_keyword<'a>(body: &'a str, keyword: &str) -> Option<&'a str> {
    body.strip_prefix(keyword)
        .filter(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

/// A macro that the parser has opened and not yet closed: its name and its text so far.
type OpenMacro = (String, String);

/// Split the text before the next `{% %}` tag from the inner text of that tag.
/// Move `rest` behind the tag. The tag is `None` when no tag is left.
fn split_next_tag<'a>(rest: &mut &'a str) -> (&'a str, Option<&'a str>) {
    match rest.split_once("{%") {
        Some((text, after)) => {
            let (inner, next) = after.split_once("%}").expect("template tag has no end");
            *rest = next;
            (text, Some(inner))
        }
        None => (rest, None),
    }
}

/// Add `text` to the open macro, or to `outside` when no macro is open.
fn push_text(current: &mut Option<OpenMacro>, outside: &mut String, text: &str) {
    match current.as_mut() {
        Some((_, body)) => body.push_str(text),
        None => outside.push_str(text),
    }
}

/// Apply one tag to the parser state. A macro opener starts a macro, a macro closer stores it,
/// and every other tag stays in the text.
fn apply_tag(
    inner: &str,
    current: &mut Option<OpenMacro>,
    macros: &mut BTreeMap<String, String>,
    outside: &mut String,
) {
    let body = tag_body(inner);
    if let (None, Some(after)) = (&*current, after_keyword(body, "macro")) {
        let name = after.trim_start().split('(').next().unwrap().trim();
        *current = Some((name.to_owned(), String::new()));
    } else if let (Some(_), Some(_)) = (&*current, after_keyword(body, "endmacro")) {
        let (name, macro_body) = current.take().unwrap();
        macros.insert(name, macro_body);
    } else {
        push_text(current, outside, &format!("{{%{inner}%}}"));
    }
}

/// Return the alias and the template path of each import line in `source`.
fn parse_imports(source: &str) -> BTreeMap<String, String> {
    let mut imports = BTreeMap::new();
    for part in source.split("import \"").skip(1) {
        let (path, rest) = part.split_once('"').unwrap();
        let alias: String = rest
            .trim_start()
            .strip_prefix("as ")
            .unwrap()
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        imports.insert(alias, path.to_owned());
    }
    imports
}

/// Parse the macro definitions, the import lines, and the text outside the macros of `source`.
/// The opener `{% macro` and the closer `{% endmacro %}` can have any white space control.
fn parse_template(source: &str) -> Template {
    let mut macros = BTreeMap::new();
    let mut outside = String::new();
    let mut current: Option<OpenMacro> = None;
    let mut rest = source;
    loop {
        let (text, tag) = split_next_tag(&mut rest);
        push_text(&mut current, &mut outside, text);
        let Some(inner) = tag else {
            break;
        };
        apply_tag(inner, &mut current, &mut macros, &mut outside);
    }
    assert!(current.is_none(), "macro has no endmacro");
    Template {
        macros,
        imports: parse_imports(source),
        outside,
    }
}

/// Every template below `templates/`, by the path that an import line and an include use.
type Templates = BTreeMap<String, Template>;

fn load_templates(files: &[PathBuf]) -> Templates {
    files
        .iter()
        .map(|path| {
            let relative = path.strip_prefix(templates_dir()).unwrap();
            let key = relative.to_string_lossy().replace('\\', "/");
            (key, parse_template(&fs::read_to_string(path).unwrap()))
        })
        .collect()
}

/// Return the path of each call in `text`: `["grids", "actions"]` for `grids::actions(`, and
/// `["member_row"]` for `member_row(`. A method call (`t.get(`), a variant (`Some(`), a path of a
/// Rust type (`String::new(`), and a path of `crate` are not macro calls.
fn macro_calls(text: &str) -> Vec<Vec<&str>> {
    let is_path_char = |c: char| c.is_alphanumeric() || c == '_' || c == ':' || c == '.';
    text.split_inclusive(|c: char| !is_path_char(c))
        .filter_map(|piece| {
            let token = piece.strip_suffix('(')?;
            let first = token.chars().next()?;
            if token.contains('.') || !(first.is_lowercase() || first == '_') {
                return None;
            }
            let path: Vec<&str> = token.split("::").collect();
            (path.first() != Some(&"crate")).then_some(path)
        })
        .collect()
}

/// Return the template path of each include in `text`.
fn includes(text: &str) -> Vec<&str> {
    text.split("include \"")
        .skip(1)
        .filter_map(|part| part.split_once('"').map(|(path, _)| path))
        .collect()
}

/// Append to `bodies` the text that `text` of the template `file` renders through a macro call or
/// an include, then the text that this text renders (design D7, steps 2 and 3). `seen` stops a
/// cycle. `faults` gets each call that the scan cannot resolve.
fn expand(
    templates: &Templates,
    file: &str,
    text: &str,
    bodies: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    faults: &mut Vec<String>,
) {
    let Some(template) = templates.get(file) else {
        faults.push(format!("{file}: the template does not exist"));
        return;
    };
    for call in macro_calls(text) {
        let shown = call.join("::");
        let target = match call.as_slice() {
            [_] => Some((file, call[0])),
            [alias, name] => template
                .imports
                .get(*alias)
                .map(|path| (path.as_str(), *name)),
            _ => None,
        };
        let found = target.and_then(|(path, name)| {
            let body = templates.get(path)?.macros.get(name)?;
            Some((path, name, body))
        });
        let Some((path, name, body)) = found else {
            faults.push(format!("{file}: unresolved macro call {shown}("));
            continue;
        };
        if seen.insert(format!("{path}::{name}")) {
            bodies.push(body.clone());
            expand(templates, path, body, bodies, seen, faults);
        }
    }
    for path in includes(text) {
        let Some(included) = templates.get(path) else {
            faults.push(format!("{file}: unresolved include {path}"));
            continue;
        };
        if seen.insert(format!("include {path}")) {
            bodies.push(included.outside.clone());
            expand(templates, path, &included.outside, bodies, seen, faults);
        }
    }
}

/// Return the faults of the submit buttons of the out-of-band form in `file` with the body `body`.
/// A button of a macro or of an included template counts, because the form renders it.
fn check_form(file: &str, body: &str, templates: &Templates) -> Vec<String> {
    let mut texts = vec![body.to_owned()];
    let mut faults = Vec::new();
    expand(
        templates,
        file,
        body,
        &mut texts,
        &mut BTreeSet::new(),
        &mut faults,
    );
    if !faults.is_empty() {
        return faults;
    }
    let buttons: Vec<&str> = texts
        .iter()
        .flat_map(|text| {
            let mut tags = start_tags(text, "button");
            tags.extend(start_tags(text, "input"));
            tags.into_iter().map(|(tag, _)| tag)
        })
        .filter(|tag| is_submit(tag))
        .collect();
    if buttons.is_empty() {
        return vec![format!("{file}: the out-of-band form has no submit button")];
    }
    buttons
        .into_iter()
        .filter(|tag| !has_fixed_id(tag))
        .map(|tag| format!("{file}: submit button without id: {tag}"))
        .collect()
}

/// Scan one template. Return the file name of each out-of-band form and the faults.
fn scan_template(path: &Path, templates: &Templates) -> (Vec<String>, Vec<String>) {
    let source = fs::read_to_string(path).unwrap();
    let file = path.file_name().unwrap().to_str().unwrap();
    let mut forms = Vec::new();
    let mut faults = Vec::new();
    for (tag, after) in start_tags(&source, "form") {
        if !tag.contains("hx-swap-oob") {
            continue;
        }
        let body = after.split_once("</form>").expect("form has no end").0;
        forms.push(file.to_owned());
        faults.extend(check_form(file, body, templates));
    }
    (forms, faults)
}

#[test]
fn oob_forms_every_out_of_band_form_has_a_submit_button_with_a_fixed_id() {
    let mut files = Vec::new();
    template_files(&templates_dir(), &mut files);
    files.sort();
    let templates = load_templates(&files);
    let mut forms = Vec::new();
    let mut faults = Vec::new();
    for path in &files {
        let (found, problems) = scan_template(path, &templates);
        forms.extend(found);
        faults.extend(problems);
    }
    assert_eq!(forms, EXPECTED_FORM_FILES, "the out-of-band forms");
    assert!(faults.is_empty(), "{}", faults.join("\n"));
}
