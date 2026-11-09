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
//!
//! The module `registry` holds the table, `scan` reads the structs of one source text, and
//! `compare` matches the scan with the table. This file holds the guard and the tests of the scan and of the comparison.

mod compare;
mod registry;
mod scan;

use std::path::Path;

use compare::{SELF_DIR, Source, coverage_faults, read_sources};
use registry::{Cover, Entry, TABLE};
use scan::{Scan, ScannedStruct, scan_source};

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
fn xss_coverage_scan_refuses_a_cfg_attr_that_wraps_serde() {
    let head = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\n";
    let on_struct = format!(
        "{head}#[cfg_attr(all(), serde(rename_all = \"UPPERCASE\"))]\nstruct F {{\n    name: String,\n}}\n"
    );
    let on_field = format!(
        "{head}struct F {{\n    #[cfg_attr(all(), serde(alias = \"other\"))]\n    name: String,\n}}\n"
    );

    let struct_scan = scan_source(&on_struct);
    let field_scan = scan_source(&on_field);

    assert_eq!(
        struct_scan.faults,
        [
            "F wraps serde in a cfg_attr, which the scan does not read: `#[cfg_attr(all(), serde(rename_all = \"UPPERCASE\"))]`"
        ]
    );
    assert_eq!(
        field_scan.faults,
        [
            "F.name wraps serde in a cfg_attr, which the scan does not read: `#[cfg_attr(all(), serde(alias = \"other\"))]`"
        ]
    );
}

#[test]
fn xss_coverage_scan_accepts_a_cfg_attr_without_serde() {
    let source = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\n#[cfg_attr(test, derive(Debug))]\nstruct F {\n    #[cfg_attr(test, allow(dead_code))]\n    name: String,\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan.faults, Vec::<String>::new());
}

#[test]
fn xss_coverage_scan_refuses_text_after_an_attribute_above_a_struct() {
    // Under `#[rustfmt::skip]` a derive and a serde attribute can share one line.
    let source = "#[derive(Deserialize)] #[serde(rename_all = \"UPPERCASE\")]\n#[serde(deny_unknown_fields)]\nstruct F {\n    name: String,\n}\n";

    let scan = scan_source(source);

    assert_eq!(
        scan.faults,
        [
            "F has text after an attribute on one line: `#[derive(Deserialize)] #[serde(rename_all = \"UPPERCASE\")]`"
        ]
    );
}

#[test]
fn xss_coverage_scan_accepts_a_line_comment_after_an_attribute_above_a_struct() {
    let source = "#[derive(Deserialize)] // note\n#[serde(deny_unknown_fields)]\nstruct F {\n    name: String,\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan.faults, Vec::<String>::new());
}

#[test]
fn xss_coverage_scan_refuses_text_after_a_field_on_one_line() {
    // Under `#[rustfmt::skip]` a field line can hold a second field or an attribute.
    let head = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct F {\n";
    let second_field = format!("{head}    a: String, b: String,\n}}\n");
    let attribute = format!("{head}    a: String, #[serde(rename = \"x\")]\n    b: String,\n}}\n");

    let second_field_scan = scan_source(&second_field);
    let attribute_scan = scan_source(&attribute);

    assert_eq!(
        second_field_scan.faults,
        ["F has text after a field on one line: `a: String, b: String,`"]
    );
    assert_eq!(
        attribute_scan.faults,
        ["F has text after a field on one line: `a: String, #[serde(rename = \"x\")]`"]
    );
}

#[test]
fn xss_coverage_scan_accepts_commas_inside_a_field_line() {
    let head = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct F {\n";
    let fields = [
        "a: HashMap<String, String>,",
        "a: (u8, u8),",
        "a: [u8; 2],",
        "a: fn(u8, u8) -> u8,",
        "#[serde(default)] // x, y\n    a: u8,",
        "a: String, // note",
        "a: String // x, y",
        "pub(crate) a: Option<Vec<u8>>,",
    ];
    for field in fields {
        let source = format!("{head}    {field}\n}}\n");

        let scan = scan_source(&source);

        assert_eq!(scan.faults, Vec::<String>::new(), "{field}");
        assert_eq!(scan.structs.len(), 1, "{field}");
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

#[test]
fn xss_coverage_scan_refuses_a_renamed_deserialize_derive() {
    let body = "#[derive(Decode)]\n#[serde(deny_unknown_fields)]\nstruct A {\n    a: u8,\n}\n";
    let uses = [
        "use serde::Deserialize as Decode;\n",
        "use serde::{Serialize, Deserialize as Decode};\n",
        "pub(crate) use serde::{\n    Serialize,\n    Deserialize\n        as Decode,\n};\n",
    ];
    for item in uses {
        let source = format!("\n{item}{body}");

        let scan = scan_source(&source);

        assert_eq!(scan.faults.len(), 1, "{item}: {:?}", scan.faults);
        let fault = scan.faults.first().unwrap();
        assert!(fault.contains("line 2"), "{item}: {fault}");
        assert!(fault.contains("Deserialize"), "{item}: {fault}");
    }
}

#[test]
fn xss_coverage_comparison_names_the_file_of_a_renamed_deserialize_derive() {
    let text = "use serde::Deserialize as Decode;\n#[derive(Decode)]\nstruct A {\n    a: u8,\n}\n";
    let sources = [source("pages/form.rs", text)];

    let faults = coverage_faults(&sources, &[]);

    assert!(
        faults
            .iter()
            .any(|fault| fault.starts_with("pages/form.rs: ") && fault.contains("line 1")),
        "{faults:?}"
    );
}

#[test]
fn xss_coverage_scan_refuses_a_deserialize_derive_in_a_macro_body() {
    let source = "use serde::Deserialize;\n\nmacro_rules! request {\n    ($name:ident, $f:ident) => {\n        #[derive(Deserialize)]\n        #[serde(deny_unknown_fields)]\n        struct $name {\n            $f: String,\n        }\n    };\n}\n";

    let scan = scan_source(source);

    let macro_faults: Vec<&String> = scan
        .faults
        .iter()
        .filter(|fault| fault.contains("macro_rules"))
        .collect();
    assert_eq!(macro_faults.len(), 1, "{:?}", scan.faults);
    assert!(
        macro_faults.iter().all(|fault| fault.contains("line 3")),
        "{macro_faults:?}"
    );
}

#[test]
fn xss_coverage_scan_refuses_a_struct_with_an_empty_or_macro_name() {
    for head in ["struct $name {", "pub struct {"] {
        let source = format!(
            "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\n{head}\n    a: u8,\n}}\n"
        );

        let scan = scan_source(&source);

        assert_eq!(scan.structs.len(), 0, "{head}: {:?}", scan.structs);
        assert!(
            scan.faults
                .iter()
                .any(|fault| fault.contains("no struct with named fields")),
            "{head}: {:?}",
            scan.faults
        );
    }
}

#[test]
fn xss_coverage_scan_refuses_a_macro_derive_list_built_from_a_metavariable() {
    let source = "macro_rules! req {\n    ($d:ident, $name:ident, $f:ident) => {\n        #[derive($d)]\n        struct $name {\n            $f: String,\n        }\n    };\n}\nreq!(Deserialize, Hidden, text);\n";

    let scan = scan_source(source);

    assert_eq!(
        scan.faults,
        vec![
            "line 1: macro_rules! req builds a derive list from a metavariable, \
             and the scan cannot read it"
                .to_owned()
        ]
    );
}

#[test]
fn xss_coverage_scan_accepts_a_macro_body_without_a_derive_list() {
    let source = "macro_rules! twice {\n    ($e:expr) => {\n        $e + $e\n    };\n}\n";

    assert_eq!(scan_source(source), Scan::default());
}

#[test]
fn xss_coverage_scan_accepts_a_plain_use_and_a_macro_without_the_derive() {
    let source = "use serde::Deserialize;\nuse serde::{Deserialize, Serialize as Ser};\n\nmacro_rules! font {\n    ($name:ident) => {\n        #[derive(Debug)]\n        struct $name {\n            a: u8,\n        }\n    };\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan, Scan::default());
}

#[test]
fn xss_coverage_scan_reads_a_field_after_a_leading_block_comment() {
    let source = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    text: String,\n    /* Submitted note */ note: Option<String>,\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan.faults, Vec::<String>::new());
    let names: Vec<&str> = scan
        .structs
        .iter()
        .flat_map(|item| &item.wire_names)
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["text", "note"]);
}

#[test]
fn xss_coverage_scan_reads_a_block_comment_over_several_lines() {
    let source = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    text: String,\n    /* The first line\n       the second line */ note: Option<String>,\n    /*\n    old: u8,\n    */\n    /* outer /* nested */ still a comment: u8, */\n    last: String,\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan.faults, Vec::<String>::new());
    let names: Vec<&str> = scan
        .structs
        .iter()
        .flat_map(|item| &item.wire_names)
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["text", "note", "last"]);
}

#[test]
fn xss_coverage_scan_keeps_a_comment_marker_inside_a_string() {
    let source = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    #[serde(rename = \"a/*b\")]\n    a: String,\n    b: String, // not /* a block\n    c: String,\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan.faults, Vec::<String>::new());
    let names: Vec<&str> = scan
        .structs
        .iter()
        .flat_map(|item| &item.wire_names)
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["a/*b", "b", "c"]);
}

#[test]
fn xss_coverage_scan_refuses_a_line_of_the_body_that_is_no_field() {
    let source = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    text: String,\n    this is no field\n    note: Option<String>,\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan.faults.len(), 1, "{:?}", scan.faults);
    let fault = scan.faults.first().unwrap();
    assert!(fault.contains("Form"), "{fault}");
    assert!(fault.contains("this is no field"), "{fault}");
}

#[test]
fn xss_coverage_scan_refuses_a_field_on_the_line_of_its_attribute() {
    let fields = [
        "#[serde(default)] a: u8,",
        "#[serde(rename = \"x]y\")] a: u8,",
        "#[serde(default)] #[serde(rename = \"x\")] a: u8,",
    ];
    for field in fields {
        let source = format!(
            "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {{\n    {field}\n}}\n"
        );

        let scan = scan_source(&source);

        assert_eq!(
            scan.faults,
            [format!(
                "Form has text after an attribute on one line: `{field}`"
            )],
            "{field}"
        );
    }
}

#[test]
fn xss_coverage_scan_reads_an_attribute_over_several_lines_and_a_comment_after_it() {
    let source = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    #[serde(\n        default,\n        rename = \"x]y\"\n    )]\n    a: u8,\n    #[serde(default)] // a note\n    b: u8,\n    #[serde(rename = \"c]\")]\n    c: u8,\n}\n";

    let scan = scan_source(source);

    assert_eq!(scan.faults, Vec::<String>::new());
    let names: Vec<&str> = scan
        .structs
        .iter()
        .flat_map(|item| &item.wire_names)
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["x]y", "b", "c]"]);
}

#[test]
fn xss_coverage_comparison_names_the_file_of_a_line_that_is_no_field() {
    let text = "#[derive(Deserialize)]\n#[serde(deny_unknown_fields)]\nstruct Form {\n    text: String,\n    this is no field\n}\n";
    let sources = [source("pages/form.rs", text)];

    let faults = coverage_faults(&sources, &[]);

    assert!(
        faults
            .iter()
            .any(|fault| fault.starts_with("pages/form.rs: ")
                && fault.contains("Form")
                && fault.contains("this is no field")),
        "{faults:?}"
    );
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
        source(&format!("{SELF_DIR}mod.rs"), "fn xss_covers_text() {}"),
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
