use super::plant_texts_tests::assert_table_text;
use super::*;

/// The texts of the review fixes of step P3.2 (design D13 and D20): key, English text, Polish text.
const REVIEW_TEXTS: &[(&str, &str, &str)] = &[
    (
        "error-E-PLT-001",
        "Machine code { $code } is already in use. Use a different code.",
        "Kod maszyny { $code } jest już używany. Użyj innego kodu.",
    ),
    (
        "error-E-PLT-002",
        "Unit path { $path } has more than 5 levels. Use at most 5 levels.",
        "Ścieżka jednostki { $path } ma więcej niż 5 poziomów. Użyj najwyżej 5 poziomów.",
    ),
    (
        "error-E-PLT-003",
        "The parent unit of { $path } does not exist. Create the parent unit first.",
        "Jednostka nadrzędna dla { $path } nie istnieje. Najpierw utwórz jednostkę nadrzędną.",
    ),
    (
        "error-E-PLT-004",
        "Unit { $path } already exists on this machine.",
        "Jednostka { $path } już istnieje na tej maszynie.",
    ),
    (
        "error-E-PLT-005",
        "Type { $type_name } contains itself. Delete the circular member.",
        "Typ { $type_name } zawiera sam siebie. Usuń składnik, który tworzy cykl.",
    ),
    (
        "error-E-PLT-006",
        "Member { $member } of { $type_name } uses a type that does not exist on { $platform }.",
        "Składnik { $member } typu { $type_name } używa typu, który nie istnieje na platformie { $platform }.",
    ),
    (
        "error-E-PLT-008",
        "Unit { $path } is the last main unit of machine { $code }. A machine needs one main unit.",
        "Jednostka { $path } jest ostatnią jednostką główną maszyny { $code }. Maszyna potrzebuje jednej jednostki głównej.",
    ),
    (
        "error-E-PLT-009",
        "Unit { $path } is archived. Select a live unit.",
        "Jednostka { $path } jest zarchiwizowana. Wybierz aktywną jednostkę.",
    ),
    (
        "error-E-PLT-010",
        "Machine { $code } is archived.",
        "Maszyna { $code } jest zarchiwizowana.",
    ),
    (
        "error-E-PLT-011",
        "Machine { $code } has a draft in recipe { $reference }. Approve or discard the draft first.",
        "Maszyna { $code } ma wersję roboczą w recepturze { $reference }. Najpierw zatwierdź albo odrzuć wersję roboczą.",
    ),
    (
        "error-E-PLT-012",
        "Unit { $path } is used by the draft of recipe { $reference }. Move or delete those parameters first.",
        "Jednostka { $path } jest używana w wersji roboczej receptury { $reference }. Najpierw przenieś albo usuń te parametry.",
    ),
    (
        "error-E-PLT-013",
        "The type { $type_expr } is not valid: { $reason }. Use a type such as INT, STRING(20), ST_Zone@3, or ARRAY[1..8] OF REAL.",
        "Typ { $type_expr } jest niepoprawny: { $reason }. Użyj typu takiego jak INT, STRING(20), ST_Zone@3 lub ARRAY[1..8] OF REAL.",
    ),
    (
        "error-E-PLT-014",
        "Unit { $path } is used by the approved version of recipe { $reference }. An Administrator can retire the version.",
        "Jednostka { $path } jest używana w zatwierdzonej wersji receptury { $reference }. Administrator może wycofać tę wersję.",
    ),
    (
        "error-E-PLT-015",
        "Type { $type_name } already exists on this machine. Create a new revision instead.",
        "Typ { $type_name } już istnieje na tej maszynie. Zamiast tego utwórz nową rewizję.",
    ),
    (
        "error-E-PLT-016",
        "Type { $type_name } has no members. Add at least one member.",
        "Typ { $type_name } nie ma składników. Dodaj co najmniej jeden składnik.",
    ),
    (
        "error-E-PLT-017",
        "{ $text } is not a unit path. A unit path is U and 1 to 5 numbers from 1 to 64, with a dot between two numbers. Example: U1.2.",
        "{ $text } nie jest ścieżką jednostki. Ścieżka jednostki to U i od 1 do 5 liczb od 1 do 64, z kropką między dwiema liczbami. Przykład: U1.2.",
    ),
    (
        "error-E-PLT-018",
        "Type { $type_name } is larger than the limit { $limit }. Use fewer elements, fewer members, or fewer levels.",
        "Typ { $type_name } przekracza limit { $limit }. Użyj mniej elementów, mniej składników lub mniej poziomów.",
    ),
    (
        "error-E-PLT-021",
        "Type { $type_name } has two members with the name { $member }. Give each member a different name.",
        "Typ { $type_name } ma dwa składniki o nazwie { $member }. Nadaj każdemu składnikowi inną nazwę.",
    ),
    (
        "type-reason-grammar",
        "the text does not match the type grammar",
        "tekst nie pasuje do gramatyki typu",
    ),
    (
        "type-reason-unknown-struct",
        "the struct or the revision does not exist on the machine",
        "struktura albo rewizja nie istnieje na tej maszynie",
    ),
    (
        "type-reason-not-on-platform",
        "the type does not exist on the platform of the machine",
        "typ nie istnieje na platformie maszyny",
    ),
    (
        "type-reason-out-of-range",
        "a bound, a dimension, or a length is outside its range",
        "granica, wymiar albo długość jest poza zakresem",
    ),
    (
        "type-reason-row-shape",
        "the stored type row does not match its kind",
        "zapisany wiersz typu nie pasuje do swojego rodzaju",
    ),
    (
        "type-limit-leaves",
        "{ $count } leaves",
        "{ $count } elementów skalarnych",
    ),
    (
        "type-limit-depth",
        "{ $count } levels",
        "{ $count } poziomów",
    ),
    (
        "type-limit-members",
        "{ $count } members",
        "{ $count } składników",
    ),
    (
        "field-platform-choose",
        "Choose a PLC platform",
        "Wybierz platformę PLC",
    ),
    (
        "list-page-after-end",
        "Page { $page } is after the last page.",
        "Strona { $page } jest za ostatnią stroną.",
    ),
    (
        "list-first-page",
        "Go to the first page",
        "Przejdź do pierwszej strony",
    ),
];

#[test]
fn review_texts_each_key_has_the_table_text_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    for &(key, en, pl) in REVIEW_TEXTS {
        assert_table_text(&bundles, Lang::En, key, en);
        assert_table_text(&bundles, Lang::Pl, key, pl);
    }
}

#[test]
fn review_texts_field_query_has_the_new_text() {
    let bundles = Bundles::load().unwrap();
    assert_eq!(
        bundles.message(Lang::En, "field-query", None),
        Some("Search".to_owned())
    );
    assert_eq!(
        bundles.message(Lang::Pl, "field-query", None),
        Some("Wyszukiwanie".to_owned())
    );
}
