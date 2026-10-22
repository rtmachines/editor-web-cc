//! Guard that each key of the approval text files has a user (change `approval-audit-history`, step 7 item S7-12).
//!
//! A user is a quoted key in the code of `src/` or in a template. A `format!` call that builds the
//! key from a prefix of `DYNAMIC_PREFIXES` is a user too. Test code is not a user.
//! A text that no page shows then cannot stay in the bundles.
//!
//! Test code is a file with a test name, a folder `test`, a module that a `#[cfg(test)] mod x;`
//! line declares, or an inline `#[cfg(test)] mod x { ... }` block. The declaration rule finds a
//! new test-only module with no change of this file.

use std::fs;
use std::path::{Path, PathBuf};

use crate::test_support::source_walk::{is_test_path, list_files};

/// The approval text files, relative to the crate root.
const APPROVAL_FILES: &[&str] = &["i18n/en-approval.ftl", "i18n/pl-approval.ftl"];

/// The folders of the crate that hold users of a key, relative to the crate root.
const USER_DIRS: &[&str] = &["src", "templates"];

/// Key prefixes that the code completes at run time with a `format!` call.
///
/// The finding S7-12 names these prefixes. The `format!` calls in `pages/diff_view.rs`,
/// `pages/audit/rows.rs`, `pages/security_events.rs`, `error/view.rs`, `error/answer.rs`, and
/// `error/lists.rs` build these keys. The prefixes `missing-` and `task-stage-` of that finding
/// are not here. Their approval keys have a literal user, so they need no prefix.
const DYNAMIC_PREFIXES: &[&str] = &[
    "audit-action-",
    "audit-entity-",
    "diff-section-",
    "error-",
    "field-",
    "security-kind-",
];

/// The attribute that starts the test-only part of a source file.
const TEST_ONLY_MARKER: &str = "#[cfg(test)]";

/// The file stems that name the folder of their module and not a sub-folder.
const MODULE_ROOT_STEMS: &[&str] = &["mod", "lib", "main"];

/// A source file of the crate: its path relative to the crate root and its text.
type Source = (PathBuf, String);

/// Returns the message key that a line of a Fluent file defines.
fn message_key(line: &str) -> Option<String> {
    let (name, _) = line.split_once('=')?;
    let key = name.trim_end();
    let starts_with_letter = key.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
    let is_name = key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    (starts_with_letter && is_name).then(|| key.to_owned())
}

/// Returns every message key of a Fluent file text.
fn message_keys(ftl: &str) -> Vec<String> {
    ftl.lines().filter_map(message_key).collect()
}

/// Returns the names of the modules that a source text declares as `#[cfg(test)] mod name;`.
fn declared_test_modules(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut is_test_only = false;
    for line in source.lines().map(str::trim) {
        if line == TEST_ONLY_MARKER {
            is_test_only = true;
        } else if !line.starts_with("#[") {
            let declared = module_declaration(line).filter(|_| is_test_only);
            found.extend(declared.map(str::to_owned));
            is_test_only = false;
        }
    }
    found
}

/// Returns the line without its visibility keyword.
fn without_visibility(line: &str) -> &str {
    let line = line.strip_prefix("pub(crate) ").unwrap_or(line);
    line.strip_prefix("pub ").unwrap_or(line)
}

/// Returns the name of the module that a line declares with `mod name;`.
fn module_declaration(line: &str) -> Option<&str> {
    without_visibility(line)
        .strip_prefix("mod ")?
        .strip_suffix(';')
}

/// Returns the file and the folder that hold the module `name`, declared in `declaring`.
fn module_paths(declaring: &Path, name: &str) -> [PathBuf; 2] {
    let folder = declaring
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let stem = declaring.file_stem().and_then(|stem| stem.to_str());
    let base = match stem {
        Some(stem) if !MODULE_ROOT_STEMS.contains(&stem) => folder.join(stem),
        _ => folder,
    };
    [base.join(format!("{name}.rs")), base.join(name)]
}

/// Returns the files and folders of all modules that the sources declare as test-only.
fn test_only_modules(sources: &[Source]) -> Vec<PathBuf> {
    sources
        .iter()
        .flat_map(|(path, text)| {
            declared_test_modules(text)
                .into_iter()
                .flat_map(|name| module_paths(path, &name))
        })
        .collect()
}

/// Returns true when the line opens an inline module, as in `mod tests {`.
fn is_inline_module(line: &str) -> bool {
    let line = without_visibility(line);
    line.starts_with("mod ") && line.ends_with('{')
}

/// Returns the source text before its inline test module.
///
/// The inline test module runs to the end of the file. The lint `items_after_test_module`
/// of clippy keeps it the last item. A cut that is too long makes a key look unused,
/// and the test then fails.
fn production_part(source: &str) -> &str {
    let mut offset = 0_usize;
    let mut marker_at: Option<usize> = None;
    for line in source.split_inclusive('\n') {
        let text = line.trim();
        if text == TEST_ONLY_MARKER {
            marker_at.get_or_insert(offset);
        } else if !text.starts_with("#[") {
            if let Some(start) = marker_at.filter(|_| is_inline_module(text)) {
                return source.get(..start).unwrap_or(source);
            }
            marker_at = None;
        }
        offset = offset.checked_add(line.len()).unwrap();
    }
    source
}

/// Returns true for a file that can hold a user of a key.
fn has_user_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "rs" || extension == "html")
}

/// Reads the code and the templates of the crate.
fn read_sources(root: &Path) -> Vec<Source> {
    let mut sources = Vec::new();
    for dir in USER_DIRS {
        for path in list_files(&root.join(dir)).unwrap() {
            if has_user_extension(&path) {
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                sources.push((relative, fs::read_to_string(&path).unwrap()));
            }
        }
    }
    sources
}

/// Returns the text of all code and templates that can use a key.
fn user_text(sources: &[Source]) -> String {
    let modules = test_only_modules(sources);
    let parts: Vec<&str> = sources
        .iter()
        .filter(|(path, _)| !is_test_path(path) && !modules.iter().any(|m| path.starts_with(m)))
        .map(|(_, text)| production_part(text))
        .collect();
    parts.join("\n")
}

/// Returns true when the text holds the needle, and the character before it is not `=`.
///
/// The needle starts with a quote. A quote after `=` opens an attribute value, as in
/// `class="diff-none"`. That value is not a user of a key.
fn has_outside_attribute(users: &str, needle: &str) -> bool {
    users
        .match_indices(needle)
        .any(|(at, _)| users.get(..at).is_some_and(|before| !before.ends_with('=')))
}

/// Returns true when the text quotes the key, or builds it from a listed prefix.
fn has_user(key: &str, prefixes: &[&str], users: &str) -> bool {
    has_outside_attribute(users, &format!("\"{key}\""))
        || prefixes.iter().any(|prefix| {
            key.starts_with(prefix) && has_outside_attribute(users, &format!("\"{prefix}{{"))
        })
}

/// Returns the keys that have no user.
fn unused_keys(keys: &[String], prefixes: &[&str], users: &str) -> Vec<String> {
    keys.iter()
        .filter(|key| !has_user(key, prefixes, users))
        .cloned()
        .collect()
}

fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn approval_keys(file: &str) -> Vec<String> {
    message_keys(&fs::read_to_string(crate_root().join(file)).unwrap())
}

fn owned(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|key| (*key).to_owned()).collect()
}

#[test]
fn reads_only_message_keys_from_a_fluent_file() {
    let ftl = "# comment\nnav-audit = Audit\nlong-text =\n    First line = not a key\n    .title = Attribute\n-term = Term\nerror-E-EXP-004 = Code\n_bad = no\n";

    let keys = message_keys(ftl);

    assert_eq!(keys, ["nav-audit", "long-text", "error-E-EXP-004"]);
}

#[test]
fn treats_test_files_and_test_folders_as_test_code() {
    let cases = [
        ("src/i18n/approval_texts_tests.rs", true),
        ("src/csrf/tests.rs", true),
        ("src/style_tests/rules.rs", true),
        ("src/test_support/recipes.rs", true),
        ("src/pages/types/test_kit.rs", true),
        ("templates/test/danger-action.html", true),
        ("src/pages/approval/approve.rs", false),
        ("templates/approve.html", false),
    ];
    for (path, expected) in cases {
        assert_eq!(is_test_path(Path::new(path)), expected, "path: {path}");
    }
}

#[test]
fn reads_the_modules_that_a_cfg_test_line_declares() {
    let source = "mod plain;\n#[cfg(test)]\nmod a_tests;\n#[cfg(test)]\n#[allow(dead_code)]\npub(crate) mod c;\n#[cfg(test)]\nmod inline {\n}\n#[cfg(test)]\nfn helper() {}\nmod after;\n";

    let modules = declared_test_modules(source);

    assert_eq!(modules, ["a_tests", "c"]);
}

#[test]
fn maps_a_declared_module_to_its_file_and_folder() {
    let cases = [
        ("src/pages/parameters.rs", "src/pages/parameters/tree_kit"),
        ("src/routes.rs", "src/routes/matrix"),
        ("src/lib.rs", "src/test_support"),
        ("src/pages/mod.rs", "src/pages/kit"),
    ];
    for (declaring, module) in cases {
        let name = module.rsplit('/').next().unwrap();

        let [file, folder] = module_paths(Path::new(declaring), name);

        assert_eq!(file, Path::new(&format!("{module}.rs")), "file of {module}");
        assert_eq!(folder, Path::new(module), "folder of {module}");
    }
}

#[test]
fn finds_the_test_only_modules_of_this_crate() {
    let modules = test_only_modules(&read_sources(crate_root()));

    for expected in [
        "src/routes/matrix.rs",
        "src/pages/parameters/tree_kit.rs",
        "src/pages/types/usage_kit.rs",
        "src/pages/versions/rebind_world.rs",
    ] {
        assert!(
            modules.iter().any(|module| module == Path::new(expected)),
            "{expected} is not a test-only module"
        );
    }
}

#[test]
fn cuts_only_an_inline_test_module() {
    let inline = "fn a() {}\n#[cfg(test)]\n#[allow(dead_code)]\nmod tests {\n    \"key\"\n}\n";
    let declared = "#[cfg(test)]\nmod a_tests;\nfn b() { t.get(\"key\"); }\n";
    let helper = "#[cfg(test)]\nfn helper() {}\nfn c() { t.get(\"key\"); }\n";

    assert_eq!(production_part(inline), "fn a() {}\n");
    assert_eq!(production_part(declared), declared);
    assert_eq!(production_part(helper), helper);
}

#[test]
fn user_text_holds_production_code_and_leaves_out_every_kind_of_test_code() {
    let source = |path: &str, text: &str| (PathBuf::from(path), text.to_owned());
    let sources = [
        source("src/a.rs", "#[cfg(test)]\nmod kit;\nt.get(\"k1\")\n"),
        source("src/a/kit.rs", "t.get(\"k2\")\n"),
        source("src/a/kit/deep.rs", "t.get(\"k3\")\n"),
        source("templates/test/x.html", "t.get(\"k4\")\n"),
        source("src/b_tests.rs", "t.get(\"k5\")\n"),
        source("templates/b.html", "t.get(\"k6\")\n"),
        source(
            "src/c.rs",
            "t.get(\"k7\")\n#[cfg(test)]\nmod tests {\n    t.get(\"k8\")\n}\n",
        ),
    ];

    let users = user_text(&sources);

    for (key, expected) in [
        ("k1", true),
        ("k2", false),
        ("k3", false),
        ("k4", false),
        ("k5", false),
        ("k6", true),
        ("k7", true),
        ("k8", false),
    ] {
        assert_eq!(users.contains(&format!("\"{key}\"")), expected, "key {key}");
    }
}

#[test]
fn counts_a_quoted_key_or_a_listed_prefix_as_a_user() {
    let users = "t.get(\"nav-audit\") format!(\"diff-section-{}\", kind)";
    let keys = owned(&["nav-audit", "diff-section-limits", "diff-none"]);

    let unused = unused_keys(&keys, &["diff-section-"], users);

    assert_eq!(unused, ["diff-none"]);
}

#[test]
fn does_not_count_a_longer_or_shorter_quoted_key_as_a_user() {
    let users = "t.get(\"nav-audit-x\") t.get(\"nav-audit\")";
    let keys = owned(&["nav", "audit", "nav-audit-x", "nav-audit", "nav-audit-x-y"]);

    let unused = unused_keys(&keys, &[], users);

    assert_eq!(unused, ["nav", "audit", "nav-audit-x-y"]);
}

#[test]
fn does_not_count_an_attribute_value_as_a_user() {
    let users = "<span class=\"diff-none\"> <i class=\"field-{{ kind }}\"> t.get(\"diff-empty\")";
    let keys = owned(&["diff-none", "field-tag", "diff-empty"]);

    let unused = unused_keys(&keys, &["field-"], users);

    assert_eq!(unused, ["diff-none", "field-tag"]);
}

#[test]
fn ignores_a_prefix_that_the_code_does_not_build() {
    let keys = owned(&["diff-section-limits"]);

    let unused = unused_keys(&keys, &["diff-section-"], "t.get(\"diff-none\")");

    assert_eq!(unused, ["diff-section-limits"]);
}

#[test]
fn each_approval_key_has_a_user_in_code_or_a_template() {
    let users = user_text(&read_sources(crate_root()));
    for file in APPROVAL_FILES {
        let keys = approval_keys(file);
        for known in ["nav-audit", "error-E-EXP-004"] {
            assert!(keys.iter().any(|key| key == known), "{file} has no {known}");
        }

        let unused = unused_keys(&keys, DYNAMIC_PREFIXES, &users);

        assert!(
            unused.is_empty(),
            "{file} has keys with no user: {unused:?}"
        );
    }
}

#[test]
fn each_dynamic_prefix_has_a_builder_and_a_key() {
    let users = user_text(&read_sources(crate_root()));
    let keys = approval_keys(APPROVAL_FILES[0]);
    for prefix in DYNAMIC_PREFIXES {
        assert!(
            has_outside_attribute(&users, &format!("\"{prefix}{{")),
            "no code builds keys from the prefix {prefix}"
        );
        assert!(
            keys.iter().any(|key| key.starts_with(prefix)),
            "no approval key has the prefix {prefix}"
        );
    }
}
