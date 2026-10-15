use super::*;

/// The texts of the plant model pages of design D13: key, English text, Polish text.
const PLANT_TEXTS: &[(&str, &str, &str)] = &[
    ("nav-machines", "Machines", "Maszyny"),
    ("status-archived", "Archived", "Zarchiwizowana"),
    ("field-code", "Machine code", "Kod maszyny"),
    ("field-machine-name", "Machine name", "Nazwa maszyny"),
    ("field-plc-platform", "PLC platform", "Platforma PLC"),
    (
        "field-main-unit-name",
        "Name of the main unit",
        "Nazwa jednostki głównej",
    ),
    ("field-parent-path", "Parent unit", "Jednostka nadrzędna"),
    (
        "field-segment",
        "Last number of the unit path",
        "Ostatni numer ścieżki jednostki",
    ),
    ("field-unit-name", "Unit name", "Nazwa jednostki"),
    ("field-type-name", "Type name", "Nazwa typu"),
    ("field-members", "Members", "Składniki"),
    (
        "field-machine-q",
        "Code or name contains",
        "Kod lub nazwa zawiera",
    ),
    (
        "field-show-archived",
        "Show the archived machines",
        "Pokaż zarchiwizowane maszyny",
    ),
    ("field-archived", "Archived object", "Zarchiwizowany obiekt"),
    ("field-add", "Add a member", "Dodaj składnik"),
    (
        "reason-machine-code",
        "a machine code is M and a number from 1 to 9999999",
        "kod maszyny to M i liczba od 1 do 9999999",
    ),
    (
        "reason-name",
        "a name has 1 to 200 characters and no control character",
        "nazwa ma od 1 do 200 znaków i nie ma znaków sterujących",
    ),
    (
        "reason-platform",
        "choose a PLC platform from the list",
        "wybierz platformę PLC z listy",
    ),
    (
        "reason-type-name",
        "a type name has 1 to 64 letters, digits, and underscores and starts with a letter or an underscore. It is not new or a primitive name",
        "nazwa typu ma od 1 do 64 liter, cyfr i podkreśleń i zaczyna się literą lub podkreśleniem. Nie może to być new ani nazwa typu prostego",
    ),
    (
        "reason-member-name",
        "a member name has 1 to 64 letters, digits, and underscores and starts with a letter or an underscore",
        "nazwa składnika ma od 1 do 64 liter, cyfr i podkreśleń i zaczyna się literą lub podkreśleniem",
    ),
    (
        "reason-member-rows",
        "each member row needs a name and a type. Clear both fields of a row that you do not want",
        "każdy wiersz składnika wymaga nazwy i typu. Wyczyść oba pola wiersza, którego nie chcesz",
    ),
    (
        "reason-member-count",
        "the form holds at most 1024 member rows",
        "formularz ma najwyżej 1024 wiersze składników",
    ),
    (
        "reason-flag",
        "the value must be 1. Reload the page and try again",
        "wartość musi wynosić 1. Odśwież stronę i spróbuj ponownie",
    ),
    (
        "reason-search-text",
        "the search text has at most 200 characters and no control character",
        "szukany tekst ma najwyżej 200 znaków i nie ma znaków sterujących",
    ),
    ("missing-malformed-machine", "Machine", "Maszyna"),
    ("missing-malformed-type", "Type", "Typ"),
    (
        "missing-type-revision",
        "Revision { $revision } of type { $name } of machine { $code }",
        "Rewizja { $revision } typu { $name } maszyny { $code }",
    ),
    (
        "platform-codesys-iec",
        "CODESYS (IEC 61131-3)",
        "CODESYS (IEC 61131-3)",
    ),
    ("platform-twincat3", "TwinCAT 3", "TwinCAT 3"),
    ("platform-s7-1200", "Siemens S7-1200", "Siemens S7-1200"),
    ("platform-s7-1500", "Siemens S7-1500", "Siemens S7-1500"),
    ("platform-s7-300", "Siemens S7-300", "Siemens S7-300"),
    ("column-code", "Code", "Kod"),
    ("column-name", "Name", "Nazwa"),
    ("column-platform", "PLC platform", "Platforma PLC"),
    ("column-state", "State", "Stan"),
    ("column-revision", "Revision", "Rewizja"),
    ("column-members", "Members", "Składniki"),
    ("column-recipe", "Recipe", "Receptura"),
    ("column-version", "Version", "Wersja"),
    ("column-tag", "Tag", "Tag"),
    ("column-type", "Type", "Typ"),
    ("pager-label", "Pages", "Strony"),
    ("pager-previous", "Previous page", "Poprzednia strona"),
    ("pager-next", "Next page", "Następna strona"),
    ("pager-current", "Page { $page }", "Strona { $page }"),
    ("machines-title", "Machines", "Maszyny"),
    ("machines-caption", "Machines of the line", "Maszyny linii"),
    (
        "machines-empty",
        "No machine matches. Archived machines show only with the filter.",
        "Żadna maszyna nie pasuje. Zarchiwizowane maszyny widać tylko z filtrem.",
    ),
    ("machines-search-submit", "Search", "Szukaj"),
    ("machines-new-link", "New machine", "Nowa maszyna"),
    (
        "machine-archived-toast",
        "Machine { $code } is archived.",
        "Maszyna { $code } została zarchiwizowana.",
    ),
    ("machine-new-title", "New machine", "Nowa maszyna"),
    (
        "machine-code-hint",
        "M and a number from 1 to 9999999, for example M7.",
        "M i liczba od 1 do 9999999, na przykład M7.",
    ),
    (
        "machine-platform-hint",
        "The platform cannot change after the first recipe.",
        "Platformy nie można zmienić po pierwszej recepturze.",
    ),
    (
        "machine-main-unit-hint",
        "The system creates the main unit U1 with this name.",
        "System tworzy jednostkę główną U1 o tej nazwie.",
    ),
    ("machine-create-submit", "Create machine", "Utwórz maszynę"),
    ("machine-title", "Machine { $code }", "Maszyna { $code }"),
    (
        "machine-types-link",
        "Open the UDT types",
        "Otwórz typy UDT",
    ),
    ("machine-units-title", "Unit tree", "Drzewo jednostek"),
    (
        "machine-rename-submit",
        "Rename machine",
        "Zmień nazwę maszyny",
    ),
    (
        "machine-renamed",
        "Machine { $code } has the new name.",
        "Maszyna { $code } ma nową nazwę.",
    ),
    (
        "machine-archive-summary",
        "Archive the machine",
        "Archiwizuj maszynę",
    ),
    (
        "machine-archive-warning",
        "The machine and its unit tree become read-only. You cannot undo this. The recipes stay readable, and a new machine can use the code { $code }.",
        "Maszyna i jej drzewo jednostek będą tylko do odczytu. Tej operacji nie można cofnąć. Receptury pozostaną dostępne, a nowa maszyna może użyć kodu { $code }.",
    ),
    (
        "machine-archive-submit",
        "Archive machine { $code }",
        "Archiwizuj maszynę { $code }",
    ),
    (
        "machine-archived-notice",
        "This machine is archived. The page is read-only.",
        "Ta maszyna jest zarchiwizowana. Strona jest tylko do odczytu.",
    ),
    ("machine-archived-at", "Archived on", "Data archiwizacji"),
    (
        "tree-add-main-unit",
        "Add a main unit",
        "Dodaj jednostkę główną",
    ),
    ("tree-add-sub-unit", "Add a sub-unit", "Dodaj podjednostkę"),
    (
        "tree-add-sub-unit-to",
        "Add a sub-unit to { $path }",
        "Dodaj podjednostkę do { $path }",
    ),
    (
        "unit-tree-title",
        "Unit tree of machine { $code }",
        "Drzewo jednostek maszyny { $code }",
    ),
    (
        "unit-new-title",
        "New unit of machine { $code }",
        "Nowa jednostka maszyny { $code }",
    ),
    (
        "unit-new-under",
        "New unit under { $path }",
        "Nowa jednostka w { $path }",
    ),
    ("unit-new-main", "New main unit", "Nowa jednostka główna"),
    (
        "unit-parent-none",
        "None. The unit is a main unit.",
        "Brak. Jednostka jest jednostką główną.",
    ),
    (
        "unit-segment-hint",
        "A number from 1 to 64. The form proposes the first free number.",
        "Liczba od 1 do 64. Formularz proponuje pierwszy wolny numer.",
    ),
    (
        "unit-no-free-segment",
        "All numbers from 1 to 64 under this parent are in use.",
        "Wszystkie numery od 1 do 64 w tej jednostce są zajęte.",
    ),
    ("unit-create-submit", "Add unit", "Dodaj jednostkę"),
    (
        "unit-added",
        "Unit { $path } is added.",
        "Jednostka { $path } została dodana.",
    ),
    (
        "unit-archived-toast",
        "Unit { $path } and its sub-units are archived.",
        "Jednostka { $path } i jej podjednostki zostały zarchiwizowane.",
    ),
    ("unit-title", "Unit { $path }", "Jednostka { $path }"),
    ("unit-machine", "Machine", "Maszyna"),
    ("unit-path", "Unit path", "Ścieżka jednostki"),
    ("unit-children-title", "Sub-units", "Podjednostki"),
    (
        "unit-children-empty",
        "The unit has no sub-unit.",
        "Jednostka nie ma podjednostek.",
    ),
    ("unit-rename-submit", "Rename unit", "Zmień nazwę jednostki"),
    (
        "unit-renamed",
        "Unit { $path } has the new name.",
        "Jednostka { $path } ma nową nazwę.",
    ),
    (
        "unit-archive-summary",
        "Archive the unit",
        "Archiwizuj jednostkę",
    ),
    (
        "unit-archive-warning",
        "The unit and all its sub-units become archived. You cannot undo this. The system keeps their history.",
        "Jednostka i wszystkie jej podjednostki zostaną zarchiwizowane. Tej operacji nie można cofnąć. System zachowa ich historię.",
    ),
    (
        "unit-archive-count",
        "Sub-units that the system also archives: { $count }.",
        "Podjednostki, które system także zarchiwizuje: { $count }.",
    ),
    (
        "unit-archive-submit",
        "Archive unit { $path }",
        "Archiwizuj jednostkę { $path }",
    ),
    (
        "types-title",
        "UDT types of machine { $code }",
        "Typy UDT maszyny { $code }",
    ),
    ("types-caption", "Struct revisions", "Rewizje struktur"),
    (
        "types-empty",
        "The machine has no UDT type.",
        "Maszyna nie ma typów UDT.",
    ),
    ("types-new-link", "New UDT type", "Nowy typ UDT"),
    ("type-latest", "latest", "najnowsza"),
    ("udt-new-title", "New UDT type", "Nowy typ UDT"),
    (
        "udt-revision-title",
        "New revision of { $name }",
        "Nowa rewizja { $name }",
    ),
    (
        "udt-name-hint",
        "A letter or an underscore first, then letters, digits, and underscores. At most 64 characters.",
        "Najpierw litera lub podkreślenie, potem litery, cyfry i podkreślenia. Najwyżej 64 znaki.",
    ),
    (
        "udt-member-row",
        "Member { $number }",
        "Składnik { $number }",
    ),
    (
        "udt-member-name",
        "Name of member { $number }",
        "Nazwa składnika { $number }",
    ),
    (
        "udt-member-type",
        "Type of member { $number }",
        "Typ składnika { $number }",
    ),
    (
        "udt-type-hint",
        "A type expression, for example INT, STRING(20), ST_Zone@3, or ARRAY[1..8] OF REAL.",
        "Wyrażenie typu, na przykład INT, STRING(20), ST_Zone@3 lub ARRAY[1..8] OF REAL.",
    ),
    (
        "udt-empty-rows-hint",
        "The system skips a row with an empty name and an empty type. Clear both fields of a member that you do not want.",
        "System pomija wiersz z pustą nazwą i pustym typem. Wyczyść oba pola składnika, którego nie chcesz.",
    ),
    ("udt-add-member", "Add a member", "Dodaj składnik"),
    (
        "udt-max-rows",
        "The form holds the maximum of 1024 members.",
        "Formularz ma maksymalną liczbę 1024 składników.",
    ),
    ("udt-create-submit", "Create type", "Utwórz typ"),
    ("udt-revision-submit", "Create revision", "Utwórz rewizję"),
    (
        "type-title",
        "Type { $name }@{ $revision }",
        "Typ { $name }@{ $revision }",
    ),
    ("type-revisions-title", "Revisions", "Rewizje"),
    ("type-tree-title", "Type tree", "Drzewo typu"),
    ("type-supersedes", "Supersedes", "Zastępuje"),
    (
        "type-usage-link",
        "Show the use of this revision",
        "Pokaż użycie tej rewizji",
    ),
    ("type-new-revision-link", "New revision", "Nowa rewizja"),
    (
        "usage-title",
        "Use of { $name }@{ $revision }",
        "Użycie { $name }@{ $revision }",
    ),
    (
        "usage-caption",
        "Parameters that use the revision",
        "Parametry, które używają rewizji",
    ),
    (
        "usage-empty",
        "No parameter uses this revision.",
        "Żaden parametr nie używa tej rewizji.",
    ),
];

/// Return the names of the placeables of a table text, for example `code` for `{ $code }`.
fn placeable_names(text: &'static str) -> impl Iterator<Item = &'static str> {
    text.split("{ $")
        .skip(1)
        .filter_map(|after| after.split_once(" }").map(|(name, _tail)| name))
}

/// Return the arguments that give each placeable of a table text its own name as value.
fn name_arguments(text: &'static str) -> FluentArgs<'static> {
    let mut args = FluentArgs::new();
    for name in placeable_names(text) {
        args.set(name, name);
    }
    args
}

/// Return a table text with each placeable replaced by its name, the text the bundle gives.
fn with_names(text: &str) -> String {
    text.replace("{ $", "").replace(" }", "")
}

/// Assert that the bundle of `lang` formats the key `key` to the table text.
pub(super) fn assert_table_text(bundles: &Bundles, lang: Lang, key: &str, text: &'static str) {
    let args = name_arguments(text);
    assert_eq!(
        bundles.message(lang, key, Some(&args)),
        Some(with_names(text)),
        "{key} in {lang}"
    );
}

#[test]
fn i18n_plant_texts_each_key_has_the_table_text_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    for &(key, en, pl) in PLANT_TEXTS {
        assert_table_text(&bundles, Lang::En, key, en);
        assert_table_text(&bundles, Lang::Pl, key, pl);
    }
}
