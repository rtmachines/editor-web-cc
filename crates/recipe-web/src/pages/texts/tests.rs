use std::fs;
use std::path::Path;

use recipe_domain::identity::Role;

use super::*;

/// Return every file below `dir`, also in the subdirectories (design D11).
fn files_below(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_below(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// The calls of the templates that take the key as the first argument.
const TEMPLATE_MARKERS: [&str; 3] = ["t.get(\"", "t.with(\"", "t.with_arg(\""];

/// Return every key that `t.get("...")`, `t.with("...", ...)`, or `t.with_arg("...", ...)` names in
/// the templates.
fn template_keys() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let mut keys = Vec::new();
    for path in files_below(&dir) {
        let text = fs::read_to_string(&path).unwrap();
        for key in keys_of_template(&text) {
            keys.push((path.display().to_string(), key));
        }
    }
    keys
}

/// Return the key of each call in `TEMPLATE_MARKERS` in the text of one template.
fn keys_of_template(text: &str) -> Vec<String> {
    TEMPLATE_MARKERS
        .iter()
        .flat_map(|marker| text.split(marker).skip(1))
        .map(|part| part.split('"').next().unwrap().to_owned())
        .collect()
}

#[test]
fn pages_texts_the_template_reader_finds_get_with_and_with_arg() {
    let template =
        r#"{{ t.get("key-a") }} {{ t.with("key-b", args) }} {{ t.with_arg("key-c", "n", 1) }}"#;

    assert_eq!(keys_of_template(template), ["key-a", "key-b", "key-c"]);
}

#[test]
fn pages_texts_every_template_key_exists_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    let keys = template_keys();
    assert!(!keys.is_empty());
    for (file, key) in keys {
        assert!(bundles.has_message(Lang::En, &key), "{file}: {key} (en)");
        assert!(bundles.has_message(Lang::Pl, &key), "{file}: {key} (pl)");
    }
}

/// The receivers of the calls `.get(`, `.with(`, and `.with_arg(` that read a key of the bundles.
///
/// The list is short on purpose. A header map, a query map, and a request also have `.get(`, and
/// their keys are no keys of the bundles.
const TEXT_RECEIVERS: [&str; 2] = ["t", "texts"];
/// The receivers of the call `.message(` that read a key of the bundles.
const BUNDLE_RECEIVERS: [&str; 2] = ["bundles", "bundles()"];
/// The methods of `Texts` that take the key as the first argument.
const TEXT_METHODS: [&str; 3] = [".get(", ".with(", ".with_arg("];
/// The method of `Bundles` that takes the key as the second argument.
const MESSAGE_METHOD: &str = ".message(";

/// Return true when `text` ends with the whole word `word`, not with the end of a longer name.
fn ends_with_word(text: &str, word: &str) -> bool {
    text.strip_suffix(word)
        .is_some_and(|rest| !rest.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_'))
}

/// Return the text of the string literal at the start of `args`, or `None` for any other start.
fn first_literal(args: &str) -> Option<String> {
    let rest = args.trim_start().strip_prefix('"')?;
    rest.split_once('"').map(|(key, _tail)| key.to_owned())
}

/// Return the keys of the calls `method` of the receivers `receivers` in `code`, with their offset.
fn keys_of_calls(
    code: &str,
    method: &str,
    receivers: &[&str],
    skip_first_argument: bool,
) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (at, _) in code.match_indices(method) {
        let (before, rest) = code.split_at(at);
        let before = before.trim_end();
        if !receivers.iter().any(|name| ends_with_word(before, name)) {
            continue;
        }
        let Some(args) = rest.strip_prefix(method) else {
            continue;
        };
        let args = if skip_first_argument {
            args.split_once(',').map(|(_first, others)| others)
        } else {
            Some(args)
        };
        if let Some(key) = args.and_then(first_literal) {
            found.push((at, key));
        }
    }
    found
}

/// Return the literal keys that the calls of the receivers `t`, `texts`, and `bundles` name.
///
/// A comment line is skipped. A key that a call builds with `format!` or reads from a variable
/// is not a literal and is skipped too.
fn literal_keys(source: &str) -> Vec<String> {
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut found: Vec<(usize, String)> = TEXT_METHODS
        .iter()
        .flat_map(|method| keys_of_calls(&code, method, &TEXT_RECEIVERS, false))
        .collect();
    found.extend(keys_of_calls(
        &code,
        MESSAGE_METHOD,
        &BUNDLE_RECEIVERS,
        true,
    ));
    found.sort();
    found.into_iter().map(|(_at, key)| key).collect()
}

/// Return true when the path is a test file or a helper of the tests, by the name of its parts.
fn is_test_path(path: &Path) -> bool {
    path.components().any(|part| {
        let name = part.as_os_str().to_string_lossy();
        let name = name.strip_suffix(".rs").unwrap_or(&name);
        name == "tests" || name.ends_with("_tests") || name.starts_with("test_")
    })
}

/// Return every literal key of the code below `src/`, with the file. Test files are left out.
fn source_keys() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut keys = Vec::new();
    for path in files_below(&root) {
        let relative = path.strip_prefix(&root).unwrap();
        let is_code = path.extension().is_some_and(|suffix| suffix == "rs");
        if !is_code || is_test_path(relative) {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        for key in literal_keys(&text) {
            keys.push((relative.display().to_string(), key));
        }
    }
    keys
}

/// A sample of code with every call shape that the reader of the keys must find or skip.
const SAMPLE: &str = r#"
    let a = t.get("key-a");
    let b = texts.with("key-b", &args);
    let c = self.texts.with_arg("key-c", "name", value);
    let d = ctx
        .state
        .bundles()
        .message(ctx.lang, "key-d", None);
    let e = bundles.message(lang, "key-e", Some(&args));
    let f = page.t.get("key-f");
    // t.get("key-comment") is only an example in a comment.
    /// `t.with_arg("key-doc", "n", 1)` is only an example in a doc comment.
    let g = headers.get("x-request-id");
    let h = map.with("not-a-key", 1);
    let i = request.get("not-a-key-either");
    let j = bundles.message(lang, &format!("error-{code}"), None);
    let k = t.get(key);
    let l = Layout::for_session(t, request, t.get("key-l"), None);
"#;

#[test]
fn pages_texts_the_reader_finds_the_literal_keys_of_the_calls() {
    let keys = literal_keys(SAMPLE);
    assert_eq!(
        keys,
        [
            "key-a", "key-b", "key-c", "key-d", "key-e", "key-f", "key-l"
        ]
    );
}

#[test]
fn pages_texts_every_literal_key_of_the_code_exists_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    let keys = source_keys();
    let known = ("error/answer.rs", "error-page-title");
    assert!(
        keys.iter()
            .any(|(file, key)| file.replace('\\', "/") == known.0 && key == known.1),
        "the reader must find the key of the error page in the code"
    );
    for (file, key) in keys {
        assert!(bundles.has_message(Lang::En, &key), "{file}: {key} (en)");
        assert!(bundles.has_message(Lang::Pl, &key), "{file}: {key} (pl)");
    }
}

#[test]
fn pages_texts_a_missing_key_shows_the_key() {
    let bundles = Bundles::load().unwrap();
    let texts = Texts::new(&bundles, Lang::En);
    assert_eq!(texts.get("no-such-key"), "no-such-key");
}

#[test]
fn pages_texts_get_follows_the_language() {
    let bundles = Bundles::load().unwrap();
    assert_eq!(
        Texts::new(&bundles, Lang::En).get("language-name"),
        "English"
    );
    assert_eq!(
        Texts::new(&bundles, Lang::Pl).get("language-name"),
        "Polski"
    );
}

#[test]
fn pages_texts_with_inserts_the_arguments() {
    let bundles = Bundles::load().unwrap();
    let mut args = FluentArgs::new();
    args.set("limit", "1 MiB");
    let text = Texts::new(&bundles, Lang::En).with("error-E-WEB-003", &args);
    assert_eq!(text, "The request is larger than 1 MiB.");
}

#[test]
fn pages_texts_role_names_follow_the_language() {
    let bundles = Bundles::load().unwrap();
    let english = Texts::new(&bundles, Lang::En);
    let polish = Texts::new(&bundles, Lang::Pl);
    assert_eq!(english.role(Role::ProcessEngineer), "Process engineer");
    assert_eq!(polish.role(Role::ProcessEngineer), "Inżynier procesu");
    assert_eq!(english.role(Role::Operator), "Operator");
}
