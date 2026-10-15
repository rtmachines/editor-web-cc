//! Tests of the form components of the plant pages: the named field, the select, the checkbox, the
//! list input, the member row, and the pager (design D5, D8, and D10, `design-system.md` 10.5).
//!
//! A test template below `templates/test/` calls one macro and takes every text from a struct
//! field. The expected markup is written here, in the form of the design.

use askama::Template;

use super::{Bundles, Lang, Texts, text};

// --- The field with an id that differs from its name ------------------------------------------

#[derive(Template)]
#[template(path = "test/named-field.html")]
struct NamedFieldPage<'a> {
    t: Texts<'a>,
    id: &'static str,
    name: &'static str,
    label: String,
    hint: String,
    kind: &'static str,
    required: bool,
    data: bool,
    maxlength: u16,
    autocomplete: &'static str,
    value: String,
}

/// Render the field `machine_name`, which sends the name `name`, in `lang`.
fn named_field(lang: Lang, hint: &str, required: bool) -> String {
    named_field_with(lang, hint, required, "")
}

/// Render the field `machine_name` with the current `value`. An empty value omits the attribute.
fn named_field_with(lang: Lang, hint: &str, required: bool, value: &str) -> String {
    let bundles = Bundles::load().unwrap();
    NamedFieldPage {
        t: Texts::new(&bundles, lang),
        id: "machine_name",
        name: "name",
        label: text(lang, "field-machine-name"),
        hint: hint.to_owned(),
        kind: "text",
        required,
        data: false,
        maxlength: 200,
        autocomplete: "off",
        value: value.to_owned(),
    }
    .render()
    .unwrap()
}

#[test]
fn components_named_field_writes_the_value_after_the_autocomplete_and_escapes_it() {
    let html = named_field_with(Lang::En, "", true, "Press line");
    assert!(
        html.contains("autocomplete=\"off\" value=\"Press line\" aria-required=\"true\">"),
        "{html}"
    );
    let hostile = named_field_with(Lang::En, "", true, "\"><script>x</script>");
    assert!(!hostile.contains("<script"), "{hostile}");
    assert!(
        hostile.contains("value=\"&#34;&#62;&#60;script&#62;"),
        "{hostile}"
    );
    assert_eq!(hostile.matches("value=").count(), 1, "{hostile}");
}

#[test]
fn components_named_field_with_an_empty_value_writes_no_value_attribute() {
    let html = named_field_with(Lang::En, "", true, "");
    assert!(!html.contains("value="), "{html}");
}

#[test]
fn components_named_field_uses_the_id_for_the_label_the_hint_and_the_input() {
    for lang in [Lang::En, Lang::Pl] {
        let label = text(lang, "field-machine-name");
        let word = text(lang, "form-required");
        let expected = format!(
            "<div class=\"field\"><label for=\"machine_name\">{label} <span class=\"field-required\">{word}</span></label><span class=\"field-hint\" id=\"machine_name-hint\">Hint text</span><input id=\"machine_name\" name=\"name\" type=\"text\" maxlength=\"200\" autocomplete=\"off\" aria-required=\"true\" aria-describedby=\"machine_name-hint\"></div>"
        );
        assert_eq!(
            named_field(lang, "Hint text", true).trim(),
            expected,
            "{lang}"
        );
    }
}

#[test]
fn components_named_field_without_a_hint_or_the_required_word_omits_them() {
    let html = named_field(Lang::En, "", false);
    for omitted in [
        "field-hint",
        "field-required",
        "aria-required",
        "aria-describedby",
    ] {
        assert!(!html.contains(omitted), "found {omitted}: {html}");
    }
    assert!(html.contains("<input id=\"machine_name\" name=\"name\" type=\"text\""));
}

// --- The select -------------------------------------------------------------------------------

struct SelectOption {
    value: &'static str,
    text: String,
}

#[derive(Template)]
#[template(path = "test/select.html")]
struct SelectPage<'a> {
    t: Texts<'a>,
    name: &'static str,
    label: String,
    hint: String,
    options: Vec<SelectOption>,
    selected: String,
    required: bool,
}

/// The five platforms with the value of the form and the text of the bundle.
const PLATFORMS: [(&str, &str); 5] = [
    ("CodesysIec", "platform-codesys-iec"),
    ("TwinCat3", "platform-twincat3"),
    ("S7_1200", "platform-s7-1200"),
    ("S7_1500", "platform-s7-1500"),
    ("S7_300", "platform-s7-300"),
];

/// Render the platform list in `lang` with `selected` and the `hint`.
fn platform_select(lang: Lang, selected: &str, hint: &str) -> String {
    platform_select_with(lang, selected, hint, false)
}

/// Render the platform list with the choice `required`.
fn platform_select_with(lang: Lang, selected: &str, hint: &str, required: bool) -> String {
    let bundles = Bundles::load().unwrap();
    let page = SelectPage {
        t: Texts::new(&bundles, lang),
        name: "plc_platform",
        label: text(lang, "field-plc-platform"),
        hint: hint.to_owned(),
        options: PLATFORMS
            .iter()
            .map(|(value, key)| SelectOption {
                value,
                text: text(lang, key),
            })
            .collect(),
        selected: selected.to_owned(),
        required,
    };
    // The macro takes the texts and writes none of them, so the test reads the field here.
    assert!(!page.t.get("form-required").is_empty());
    page.render().unwrap()
}

#[test]
fn components_select_lists_each_option_and_marks_the_selected_one() {
    for lang in [Lang::En, Lang::Pl] {
        let hint = text(lang, "machine-platform-hint");
        let options: String = PLATFORMS
            .iter()
            .map(|(value, key)| {
                let mark = if *value == "S7_1500" { " selected" } else { "" };
                format!(
                    "<option value=\"{value}\"{mark}>{}</option>",
                    text(lang, key)
                )
            })
            .collect();
        let expected = format!(
            "<div class=\"field\"><label for=\"plc_platform\">{}</label><span class=\"field-hint\" id=\"plc_platform-hint\">{hint}</span><select id=\"plc_platform\" name=\"plc_platform\" aria-describedby=\"plc_platform-hint\">{options}</select></div>",
            text(lang, "field-plc-platform"),
        );
        assert_eq!(
            platform_select(lang, "S7_1500", &hint).trim(),
            expected,
            "{lang}"
        );
    }
}

#[test]
fn components_select_without_a_selected_value_or_a_hint_marks_nothing_and_omits_the_hint() {
    let html = platform_select(Lang::En, "", "");
    assert_eq!(html.matches("<option").count(), 5);
    assert!(!html.contains("selected"), "{html}");
    assert!(!html.contains("field-hint"));
    assert!(!html.contains("aria-describedby"));
}

#[test]
fn components_select_marks_exactly_one_option_and_escapes_its_values() {
    let html = platform_select(Lang::En, "S7_300", "h");
    assert_eq!(html.matches(" selected").count(), 1, "{html}");
    assert!(html.contains("<option value=\"S7_300\" selected>Siemens S7-300</option>"));
    let bundles = Bundles::load().unwrap();
    let page = SelectPage {
        t: Texts::new(&bundles, Lang::En),
        name: "plc_platform",
        label: "<b>".to_owned(),
        hint: "<i>".to_owned(),
        options: vec![SelectOption {
            value: "x\"><script>",
            text: "<script>alert(1)</script>".to_owned(),
        }],
        selected: String::new(),
        required: false,
    };
    assert!(!page.t.get("form-required").is_empty());
    let html = page.render().unwrap();
    for raw in ["<script", "<b>", "<i>", "x\"><"] {
        assert!(!html.contains(raw), "found {raw}: {html}");
    }
}

/// RQ-2: a required list has the required mark in its label and `aria-required` on the select.
#[test]
fn components_select_required_marks_the_label_and_the_select_like_a_required_input() {
    for lang in [Lang::En, Lang::Pl] {
        let hint = text(lang, "machine-platform-hint");
        let html = platform_select_with(lang, "", &hint, true);
        let head = format!(
            "<label for=\"plc_platform\">{} <span class=\"field-required\">{}</span></label>",
            text(lang, "field-plc-platform"),
            text(lang, "form-required")
        );
        assert!(html.contains(&head), "{lang}: {html}");
        assert!(
            html.contains("<select id=\"plc_platform\" name=\"plc_platform\" aria-required=\"true\" aria-describedby=\"plc_platform-hint\">"),
            "{lang}: {html}"
        );
        let plain = platform_select_with(lang, "", &hint, false);
        assert!(!plain.contains("field-required"), "{plain}");
        assert!(!plain.contains("aria-required"), "{plain}");
    }
}

// --- The checkbox -----------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "test/check.html")]
struct CheckPage<'a> {
    t: Texts<'a>,
    name: &'static str,
    label: String,
    checked: bool,
}

/// Render the checkbox `show_archived` in `lang`.
fn check(lang: Lang, checked: bool) -> String {
    let bundles = Bundles::load().unwrap();
    let page = CheckPage {
        t: Texts::new(&bundles, lang),
        name: "show_archived",
        label: text(lang, "field-show-archived"),
        checked,
    };
    // The macro takes the texts and writes none of them, so the test reads the field here.
    assert!(!page.t.get("form-required").is_empty());
    page.render().unwrap()
}

#[test]
fn components_check_label_holds_the_checkbox_and_its_text() {
    for lang in [Lang::En, Lang::Pl] {
        let label = text(lang, "field-show-archived");
        let expected = format!(
            "<label class=\"check\"><input type=\"checkbox\" id=\"show_archived\" name=\"show_archived\" value=\"1\"><span>{label}</span></label>"
        );
        assert_eq!(check(lang, false).trim(), expected, "{lang}");
    }
}

#[test]
fn components_check_checked_adds_the_attribute_and_the_value_stays_1() {
    let html = check(Lang::En, true);
    assert!(html.contains("value=\"1\" checked>"), "{html}");
    assert!(!check(Lang::En, false).contains("checked"));
}

// --- The input with a list of proposals -------------------------------------------------------

#[derive(Template)]
#[template(path = "test/listed.html")]
struct ListedPage<'a> {
    t: Texts<'a>,
    name: &'static str,
    label: String,
    hint_id: &'static str,
    list_id: &'static str,
    value: String,
}

/// Render the input `type_name` with the value `value`.
fn listed(value: &str) -> String {
    let bundles = Bundles::load().unwrap();
    let page = ListedPage {
        t: Texts::new(&bundles, Lang::En),
        name: "type_name",
        label: text(Lang::En, "field-type-name"),
        hint_id: "udt-type-hint",
        list_id: "type-choices",
        value: value.to_owned(),
    };
    // The macro takes the texts and writes none of them, so the test reads the field here.
    assert!(!page.t.get("form-required").is_empty());
    page.render().unwrap()
}

#[test]
fn components_listed_input_names_its_list_its_hint_and_turns_off_the_autocomplete() {
    let expected = "<div class=\"field\"><label for=\"type_name\">Type name</label><input id=\"type_name\" name=\"type_name\" type=\"text\" class=\"data\" list=\"type-choices\" autocomplete=\"off\" autocapitalize=\"none\" spellcheck=\"false\" aria-describedby=\"udt-type-hint\" value=\"ST_Zone@2\"></div>";
    assert_eq!(listed("ST_Zone@2").trim(), expected);
}

#[test]
fn components_listed_input_escapes_its_value_and_has_none_of_the_forbidden_attributes() {
    let html = listed("\"><script>alert(1)</script>");
    assert!(!html.contains("<script"), "{html}");
    assert!(
        html.contains("value=\"&#34;&#62;&#60;script&#62;"),
        "{html}"
    );
    for forbidden in [" required", " pattern=", " minlength=", " placeholder="] {
        assert!(!html.contains(forbidden), "found {forbidden}");
    }
}

// --- The member row ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "test/member-row.html")]
struct MemberRowPage<'a> {
    t: Texts<'a>,
    index: u16,
    number: u16,
    name: String,
    type_text: String,
    focus: bool,
}

/// Render the member row `index` for the row number `number`.
fn member_row(
    lang: Lang,
    index: u16,
    number: u16,
    name: &str,
    type_text: &str,
    focus: bool,
) -> String {
    let bundles = Bundles::load().unwrap();
    MemberRowPage {
        t: Texts::new(&bundles, lang),
        index,
        number,
        name: name.to_owned(),
        type_text: type_text.to_owned(),
        focus,
    }
    .render()
    .unwrap()
}

/// The expected markup of a row in English with the extra attributes of the name input.
fn expected_row(index: u16, number: u16, name: &str, type_text: &str, focus: &str) -> String {
    format!(
        "<fieldset class=\"member-row\"><legend>Member {number}</legend><div class=\"member-fields\"><div class=\"field\"><label for=\"members-{index}-name\">Name of member {number}</label><input id=\"members-{index}-name\" name=\"members[{index}].name\" type=\"text\" class=\"data\" autocapitalize=\"none\" spellcheck=\"false\" maxlength=\"64\" autocomplete=\"off\" value=\"{name}\"{focus}></div><div class=\"field\"><label for=\"members-{index}-type\">Type of member {number}</label><input id=\"members-{index}-type\" name=\"members[{index}].type\" type=\"text\" class=\"data\" list=\"type-choices\" autocomplete=\"off\" autocapitalize=\"none\" spellcheck=\"false\" aria-describedby=\"udt-type-hint\" value=\"{type_text}\"></div></div></fieldset>"
    )
}

#[test]
fn components_member_row_has_the_ids_the_names_and_the_labels_with_the_row_number() {
    let html = member_row(Lang::En, 0, 1, "temp_max", "REAL", false);
    assert_eq!(html.trim(), expected_row(0, 1, "temp_max", "REAL", ""));
}

#[test]
fn components_member_row_index_and_number_are_two_values() {
    let html = member_row(Lang::En, 4, 5, "speed", "ARRAY[1..4] OF REAL", false);
    assert_eq!(
        html.trim(),
        expected_row(4, 5, "speed", "ARRAY[1..4] OF REAL", "")
    );
    assert!(html.contains("id=\"members-4-name\""));
    assert!(html.contains("name=\"members[4].type\""));
}

#[test]
fn components_member_row_with_the_focus_flag_puts_autofocus_on_the_name_input_only() {
    let html = member_row(Lang::En, 1, 2, "", "", true);
    assert_eq!(html.trim(), expected_row(1, 2, "", "", " autofocus"));
    assert_eq!(html.matches("autofocus").count(), 1);
    assert!(html.contains("id=\"members-1-name\" name=\"members[1].name\""));
}

#[test]
fn components_member_row_labels_follow_the_language() {
    let html = member_row(Lang::Pl, 2, 3, "a", "b", false);
    for label in [
        "Sk\u{142}adnik 3",
        "Nazwa sk\u{142}adnika 3",
        "Typ sk\u{142}adnika 3",
    ] {
        assert!(html.contains(label), "lost {label}: {html}");
    }
}

#[test]
fn components_member_row_escapes_name_and_type_and_writes_no_script_or_hx_attribute() {
    let html = member_row(Lang::En, 0, 1, "\"><script>x</script>", "<b>", false);
    for raw in ["<script", "<b>", " hx-", " style=", " on"] {
        assert!(!html.contains(raw), "found {raw}: {html}");
    }
    assert!(html.contains("value=\"&#34;&#62;&#60;script&#62;x&#60;/script&#62;\""));
}

// --- The pager --------------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "test/pager.html")]
struct PagerPage<'a> {
    t: Texts<'a>,
    previous: String,
    next: String,
    current: u32,
}

/// Render the pager on page `current` with the two addresses. An empty address omits its link.
fn pager(lang: Lang, previous: &str, next: &str, current: u32) -> String {
    let bundles = Bundles::load().unwrap();
    PagerPage {
        t: Texts::new(&bundles, lang),
        previous: previous.to_owned(),
        next: next.to_owned(),
        current,
    }
    .render()
    .unwrap()
}

#[test]
fn components_pager_is_a_labelled_nav_with_two_target_links_and_the_current_page() {
    for (lang, label, previous, next, current) in [
        (Lang::En, "Pages", "Previous page", "Next page", "Page 2"),
        (
            Lang::Pl,
            "Strony",
            "Poprzednia strona",
            "Nast\u{119}pna strona",
            "Strona 2",
        ),
    ] {
        let expected = format!(
            "<nav class=\"pager\" aria-label=\"{label}\"><a class=\"target-link\" href=\"/machines?page=1\">{previous}</a><span class=\"pager-current\" aria-current=\"page\">{current}</span><a class=\"target-link\" href=\"/machines?page=3\">{next}</a></nav>"
        );
        let html = pager(lang, "/machines?page=1", "/machines?page=3", 2);
        assert_eq!(html.trim(), expected, "{lang}");
    }
}

#[test]
fn components_pager_omits_a_link_without_an_address() {
    let first = pager(Lang::En, "", "/machines?page=2", 1);
    assert!(!first.contains("Previous page"), "{first}");
    assert!(first.contains("Next page"));
    assert_eq!(first.matches("<a ").count(), 1);
    let last = pager(Lang::En, "/machines?page=1", "", 2);
    assert!(!last.contains("Next page"), "{last}");
    assert_eq!(last.matches("<a ").count(), 1);
    let alone = pager(Lang::En, "", "", 1);
    assert_eq!(alone.matches("<a ").count(), 0);
    assert!(alone.contains("Page 1"));
}

#[test]
fn components_pager_escapes_the_address_and_keeps_every_link_a_target_link() {
    let html = pager(Lang::En, "/x\"><script>alert(1)</script>", "/y?a=1&b=2", 2);
    assert!(!html.contains("<script"), "{html}");
    assert!(html.contains("href=\"/y?a=1&#38;b=2\""), "{html}");
    assert_eq!(
        html.matches("<a class=\"target-link\"").count(),
        html.matches("<a ").count()
    );
}
