use super::*;

/// The error codes `E-RCP` of design D13.
const RCP_ERROR_KEYS: &[&str] = &[
    "error-E-RCP-001",
    "error-E-RCP-002",
    "error-E-RCP-003",
    "error-E-RCP-004",
    "error-E-RCP-005",
    "error-E-RCP-006",
    "error-E-RCP-007",
    "error-E-RCP-008",
    "error-E-RCP-009",
    "error-E-RCP-010",
    "error-E-RCP-011",
    "error-E-RCP-012",
    "error-E-RCP-013",
    "error-E-RCP-014",
    "error-E-RCP-015",
    "error-E-RCP-016",
    "error-E-RCP-017",
    "error-E-RCP-018",
    "error-E-RCP-019",
    "error-E-RCP-020",
    "error-E-RCP-021",
    "error-E-RCP-022",
    "error-E-RCP-023",
    "error-E-RCP-024",
    "error-E-RCP-025",
    "error-E-RCP-026",
    "error-E-RCP-027",
    "error-E-RCP-028",
    "error-E-RCP-029",
    "error-E-RCP-030",
    "error-E-RCP-031",
    "error-E-RCP-032",
    "error-E-RCP-033",
    "error-E-RCP-034",
    "error-E-RCP-035",
    "error-E-RCP-036",
    "error-E-RCP-050",
    "error-E-RCP-051",
    "error-E-RCP-052",
    "error-E-RCP-053",
    "error-E-RCP-054",
];

/// The error codes `E-PLT` that the recipe pages add to D13.
const PLT_ERROR_KEYS: &[&str] = &[
    "error-E-PLT-019",
    "error-E-PLT-020",
    "error-E-PLT-030",
    "error-E-PLT-031",
    "error-E-PLT-032",
    "error-E-PLT-033",
    "error-E-PLT-034",
    "error-E-PLT-035",
    "error-E-PLT-036",
    "error-E-PLT-037",
    "error-E-PLT-038",
];

/// The expected forms, the budgets, and the rebind reasons of D13.
const EXPECTED_KEYS: &[&str] = &[
    "expected-json",
    "expected-depth",
    "expected-object",
    "expected-array",
    "expected-array-count",
    "expected-index",
    "expected-scalar",
    "expected-wstring",
    "expected-string",
    "expected-char",
    "expected-literal",
    "expected-count",
    "expected-integer",
    "expected-number-range",
    "expected-number-digits",
    "expected-parameter-type",
    "expected-new-value",
    "expected-bool",
    "expected-decimal",
    "expected-leaf",
    "expected-known-leaf",
    "budget-parameters",
    "budget-leaves",
    "budget-limit-rows",
    "budget-version-numbers",
    "rebind-reason-source-not-struct",
    "rebind-reason-target-not-struct",
    "rebind-reason-target-not-found",
    "rebind-reason-other-struct",
    "rebind-reason-not-newer",
    "rebind-reason-not-used",
    "rebind-reason-plan-mismatch",
];

/// The states, the missing messages, and the operation names of D13.
const STATE_AND_MISSING_KEYS: &[&str] = &[
    "status-deleted",
    "status-outdated",
    "status-discarded",
    "missing-malformed-recipe",
    "missing-malformed-version",
    "missing-malformed-parameter",
    "missing-parameter",
    "operation-list-deleted-recipes",
];

/// The reasons of the form fields of D13.
const REASON_KEYS: &[&str] = &[
    "reason-reference-number",
    "reason-change-note",
    "reason-description",
    "reason-translation-description",
    "reason-tag",
    "reason-ordinal",
    "reason-s7-address",
    "reason-based-on",
    "reason-status",
    "reason-value-key",
    "reason-limit-row",
    "reason-max-length",
    "reason-lang-list",
    "reason-reload",
];

/// The field labels of D13.
const FIELD_KEYS: &[&str] = &[
    "field-machine-code",
    "field-reference-number",
    "field-recipe-name",
    "field-change-note",
    "field-recipe-q",
    "field-recipe-machine",
    "field-recipe-status",
    "field-show-deleted",
    "field-based-on",
    "field-unit-path",
    "field-tag",
    "field-name-en",
    "field-description-en",
    "field-type",
    "field-s7-address",
    "field-ordinal",
    "field-row-version",
    "field-value-type",
    "field-limit-mode",
    "field-value-leaf",
    "field-limit-min",
    "field-limit-max",
    "field-limit-max-length",
    "field-translation-name",
    "field-translation-description",
    "field-new-member",
];

/// The recipe list, recipe, and version texts of D13.
const RECIPE_PAGE_KEYS: &[&str] = &[
    "recipes-caption",
    "recipes-search-hint",
    "recipes-any-machine",
    "recipes-any-status",
    "recipes-filter-submit",
    "recipes-new-link",
    "recipes-col-reference",
    "recipes-col-name",
    "recipes-col-machine",
    "recipes-col-approved",
    "recipes-col-draft",
    "recipes-none",
    "recipe-new-title",
    "recipe-reference-hint",
    "recipe-change-note-hint",
    "recipe-create-submit",
    "recipe-machine-choose",
    "recipe-title",
    "recipe-machine",
    "recipe-name",
    "recipe-rename-submit",
    "recipe-renamed",
    "versions-caption",
    "versions-col-number",
    "versions-col-state",
    "versions-col-change-note",
    "versions-col-created",
    "versions-col-approved",
    "version-new-link",
    "version-new-title",
    "version-empty-draft",
    "version-based-on-hint",
    "version-create-submit",
    "version-title",
    "version-state",
    "version-change-note",
    "version-created-by",
    "version-created-at",
    "version-approved-by",
    "version-approved-at",
    "version-updated-at",
    "version-platform",
];

/// The parameter list and parameter form texts of D13.
const PARAMETER_KEYS: &[&str] = &[
    "parameters-caption",
    "parameters-col-tag",
    "parameters-col-name",
    "parameters-col-type",
    "parameters-col-unit",
    "parameters-col-address",
    "parameters-col-state",
    "parameters-empty",
    "parameters-units-label",
    "parameters-unit-link",
    "parameter-add-link",
    "parameter-new-title",
    "parameter-tag-hint",
    "parameter-s7-hint",
    "parameter-ordinal-hint",
    "parameter-show-fields",
    "parameter-fields-first",
    "value-type-notice",
    "parameter-add-submit",
    "parameter-added",
    "parameter-title",
    "parameter-key",
    "parameter-save-submit",
    "parameter-saved",
    "parameter-delete-summary",
    "parameter-delete-warning",
    "parameter-delete-submit",
    "parameter-deleted",
];

/// The value grid texts of D13.
const GRID_KEYS: &[&str] = &[
    "grid-caption",
    "grid-col-path",
    "grid-col-type",
    "grid-col-value",
    "grid-col-min",
    "grid-col-max",
    "grid-col-max-length",
    "grid-all-elements",
    "grid-limit-from-all",
    "grid-length-from-all",
    "grid-limit-none-from-all",
    "grid-not-applicable",
    "grid-group",
    "grid-mode-split",
    "grid-mode-join",
    "grid-mode-current-all",
    "grid-mode-current-each",
    "grid-mode-join-notice",
    "grid-hint-integer",
    "grid-hint-number",
    "grid-hint-duration",
    "grid-hint-date",
    "grid-hint-date-time",
    "grid-hint-time-of-day",
    "grid-hint-string",
    "grid-true",
    "grid-false",
    "grid-limits-empty",
];

/// The translation, outdated, rebind, and conflict texts of D13.
const TRANSLATION_AND_CONFLICT_KEYS: &[&str] = &[
    "translations-caption",
    "translations-col-lang",
    "translations-col-base",
    "translation-none",
    "translation-save-submit",
    "translation-saved",
    "translation-delete-summary",
    "translation-delete-warning",
    "translation-delete-submit",
    "translation-deleted",
    "outdated-caption",
    "outdated-hint",
    "outdated-col-struct",
    "rebind-title",
    "rebind-new-members",
    "rebind-no-new-members",
    "rebind-submit",
    "rebind-done",
    "rebind-too-large",
    "conflict-row-version",
    "conflict-last-change",
    "conflict-no-editor",
    "conflict-reload-warning",
    "conflict-reload",
    "errors-count",
    "errors-more",
];

/// Return true when the bundle of `lang` holds the message `key` with a non-empty text.
fn has_text(bundles: &Bundles, lang: Lang, key: &str) -> bool {
    bundles
        .bundle(lang)
        .get_message(key)
        .and_then(|message| message.value())
        .is_some_and(|pattern| !pattern.elements.is_empty())
}

/// Assert that each key of `keys` has a non-empty text in both bundles `en` and `pl`.
fn assert_keys_have_text(keys: &[&str]) {
    let bundles = Bundles::load().unwrap();
    for &key in keys {
        for lang in [Lang::En, Lang::Pl] {
            assert!(has_text(&bundles, lang, key), "{key} in {lang}");
        }
    }
}

#[test]
fn i18n_recipe_texts_rcp_error_keys_have_text_in_both_bundles() {
    assert_keys_have_text(RCP_ERROR_KEYS);
}

#[test]
fn i18n_recipe_texts_plt_error_keys_have_text_in_both_bundles() {
    assert_keys_have_text(PLT_ERROR_KEYS);
}

#[test]
fn i18n_recipe_texts_expected_keys_have_text_in_both_bundles() {
    assert_keys_have_text(EXPECTED_KEYS);
}

#[test]
fn i18n_recipe_texts_state_and_missing_keys_have_text_in_both_bundles() {
    assert_keys_have_text(STATE_AND_MISSING_KEYS);
}

#[test]
fn i18n_recipe_texts_reason_keys_have_text_in_both_bundles() {
    assert_keys_have_text(REASON_KEYS);
}

#[test]
fn i18n_recipe_texts_field_keys_have_text_in_both_bundles() {
    assert_keys_have_text(FIELD_KEYS);
}

#[test]
fn i18n_recipe_texts_recipe_page_keys_have_text_in_both_bundles() {
    assert_keys_have_text(RECIPE_PAGE_KEYS);
}

#[test]
fn i18n_recipe_texts_parameter_keys_have_text_in_both_bundles() {
    assert_keys_have_text(PARAMETER_KEYS);
}

#[test]
fn i18n_recipe_texts_grid_keys_have_text_in_both_bundles() {
    assert_keys_have_text(GRID_KEYS);
}

#[test]
fn i18n_recipe_texts_translation_and_conflict_keys_have_text_in_both_bundles() {
    assert_keys_have_text(TRANSLATION_AND_CONFLICT_KEYS);
}
