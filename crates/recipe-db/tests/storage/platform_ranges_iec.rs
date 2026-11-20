//! Tests of `deploy/db/check-platform-ranges.sql` for the `iec_wide_text` part (ADR-0201 (a)).
//!
//! The query lists a `WSTRING` value on `CodesysIec` or `TwinCat3` with a character outside the
//! Basic Multilingual Plane. The seed helpers come from `platform_ranges.rs`.

use std::collections::BTreeSet;

use crate::platform_ranges::{Expected, Fate, Row, add, json_text, rows, settle, site, wide_type};
use crate::support::fixtures::TWIN_CAT;
use crate::support::{Role, TestDatabase};

const S7_1500: &str = "S7_1500";
/// A character outside the Basic Multilingual Plane (ADR-0201 (a)).
const NON_BMP: &str = "\u{1F600}";
/// The last character of the Basic Multilingual Plane (ADR-0201 (a)).
const BMP_LAST: &str = "\u{FFFF}";

#[tokio::test]
async fn adr_0201_iec_wide_text_lists_only_iec_rows() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let mut expected = Expected::default();
    let text = format!("A{NON_BMP}");

    let twin_cat = site(&mut conn, TWIN_CAT).await;
    let wstring = wide_type(&mut conn, "WSTRING", TWIN_CAT).await;
    add(
        &mut conn,
        &twin_cat,
        wstring,
        "WSTRING_NON_BMP_TWIN_CAT",
        None,
        &json_text(&text),
    )
    .await;
    let place = settle(&mut conn, &twin_cat, Fate::Approved).await;
    expected.hit(&place, "WSTRING_NON_BMP_TWIN_CAT", "iec_wide_text", &text);

    let last = site(&mut conn, TWIN_CAT).await;
    let wstring = wide_type(&mut conn, "WSTRING", TWIN_CAT).await;
    let last_text = format!("A{BMP_LAST}");
    add(
        &mut conn,
        &last,
        wstring,
        "NO_HIT_WSTRING_FFFF_TWIN_CAT",
        None,
        &json_text(&last_text),
    )
    .await;
    settle(&mut conn, &last, Fate::Draft).await;

    let s7 = site(&mut conn, S7_1500).await;
    let wstring = wide_type(&mut conn, "WSTRING", S7_1500).await;
    add(
        &mut conn,
        &s7,
        wstring,
        "NO_HIT_WSTRING_NON_BMP_S7_1500",
        None,
        &json_text(&text),
    )
    .await;
    settle(&mut conn, &s7, Fate::Draft).await;

    let mut app = database.connect(Role::App).await;
    let found: BTreeSet<Row> = rows(&mut app)
        .await
        .into_iter()
        .filter(|row| row.5 == "iec_wide_text")
        .collect();

    assert_eq!(found, expected.0);
}
