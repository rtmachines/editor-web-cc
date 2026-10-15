//! Contract cases of `PlantRepository::list_machines` (design D4).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions.

use recipe_app::error::StorageError;
use recipe_app::ports::{MachineFilter, MachineSummary};
use recipe_domain::plant::{Machine, PlcPlatform};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::harness::{Harness, begin};
use super::seed::{code, insert, machine, machine_id, name, unit_id};

/// The page that holds every row of a case.
const WHOLE: (u64, u32) = (0, 100);

/// Read one page in a transaction that nothing commits.
async fn list(
    h: &dyn Harness,
    text: Option<&str>,
    include_archived: bool,
    (offset, limit): (u64, u32),
) -> Vec<MachineSummary> {
    let filter = MachineFilter {
        text: text.map(name),
        include_archived,
    };
    let mut tx = begin(h).await;
    tx.plant()
        .list_machines(&filter, offset, limit)
        .await
        .unwrap()
}

/// Return the codes of the rows as text, in the order of the rows.
fn codes(rows: &[MachineSummary]) -> Vec<String> {
    rows.iter().map(|row| row.code.to_string()).collect()
}

/// A live machine with its own name and the platform `S7_1500`.
fn named(n: u128, code_text: &str, machine_name: &str) -> Machine {
    Machine::create(
        machine_id(n),
        code(code_text),
        name(machine_name),
        PlcPlatform::S7_1500,
        (unit_id(n.checked_mul(10).unwrap()), name("Main unit")),
    )
}

/// A whole-second instant, so that the database keeps it without a loss.
fn at(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

/// A machine that is archived at `seconds`.
fn archived(n: u128, code_text: &str, seconds: i64) -> Machine {
    let mut built = machine(n, code_text);
    built.archive(at(seconds), None).unwrap();
    built
}

async fn machines_come_in_the_order_of_the_number_of_their_code(h: &dyn Harness) {
    insert(h, &machine(1, "M10")).await;
    insert(h, &machine(2, "M2")).await;
    insert(h, &machine(3, "M1")).await;

    let rows = list(h, None, false, WHOLE).await;

    assert_eq!(codes(&rows), ["M1", "M2", "M10"]);
}

async fn no_search_text_lists_every_live_machine_with_its_columns(h: &dyn Harness) {
    insert(h, &machine(4, "M1")).await;
    insert(h, &named(5, "M2", "Łódź")).await;

    let rows = list(h, None, false, WHOLE).await;

    assert_eq!(
        rows,
        [
            MachineSummary {
                id: machine_id(4),
                code: code("M1"),
                name: name("Press"),
                platform: PlcPlatform::TwinCat3,
                archived_at: None,
            },
            MachineSummary {
                id: machine_id(5),
                code: code("M2"),
                name: name("Łódź"),
                platform: PlcPlatform::S7_1500,
                archived_at: None,
            },
        ]
    );
}

async fn a_search_in_other_case_finds_a_polish_name(h: &dyn Harness) {
    insert(h, &machine(6, "M1")).await;
    insert(h, &named(7, "M2", "Łódź")).await;

    let upper_lower = list(h, Some("łÓDŹ"), false, WHOLE).await;
    let all_upper = list(h, Some("ŁÓDŹ"), false, WHOLE).await;
    let ascii = list(h, Some("PRESS"), false, WHOLE).await;

    assert_eq!(codes(&upper_lower), ["M2"]);
    assert_eq!(codes(&all_upper), ["M2"]);
    assert_eq!(codes(&ascii), ["M1"]);
}

async fn a_search_on_a_part_of_the_code_finds_the_code(h: &dyn Harness) {
    insert(h, &machine(8, "M1")).await;
    insert(h, &machine(9, "M2")).await;
    insert(h, &machine(10, "M10")).await;

    let number = list(h, Some("10"), false, WHOLE).await;
    let prefix = list(h, Some("m1"), false, WHOLE).await;

    assert_eq!(codes(&number), ["M10"]);
    assert_eq!(codes(&prefix), ["M1", "M10"]);
}

async fn a_wildcard_character_of_the_search_matches_itself_only(h: &dyn Harness) {
    insert(h, &machine(11, "M1")).await;
    insert(h, &named(12, "M2", "50% Press")).await;
    insert(h, &named(13, "M3", "Line_A")).await;

    let percent = list(h, Some("%"), false, WHOLE).await;
    let underscore = list(h, Some("_"), false, WHOLE).await;

    assert_eq!(codes(&percent), ["M2"]);
    assert_eq!(codes(&underscore), ["M3"]);
}

async fn archived_machines_show_only_with_the_flag_and_follow_the_live_one(h: &dyn Harness) {
    insert(h, &machine(14, "M1")).await;
    insert(h, &machine(15, "M5")).await;
    insert(h, &archived(16, "M5", 1_700_000_100)).await;
    insert(h, &archived(17, "M5", 1_700_000_200)).await;
    insert(h, &archived(18, "M3", 1_700_000_300)).await;

    let live_only = list(h, None, false, WHOLE).await;
    let with_archived = list(h, None, true, WHOLE).await;

    assert_eq!(codes(&live_only), ["M1", "M5"]);
    assert_eq!(codes(&with_archived), ["M1", "M3", "M5", "M5", "M5"]);
    let m5: Vec<_> = with_archived
        .iter()
        .filter(|row| row.code == code("M5"))
        .map(|row| (row.id, row.archived_at))
        .collect();
    assert_eq!(
        m5,
        [
            (machine_id(15), None),
            (machine_id(17), Some(at(1_700_000_200))),
            (machine_id(16), Some(at(1_700_000_100))),
        ]
    );
}

async fn the_search_also_applies_to_archived_machines_with_the_flag(h: &dyn Harness) {
    insert(h, &machine(19, "M1")).await;
    insert(h, &archived(20, "M12", 1_700_000_100)).await;

    let without = list(h, Some("12"), false, WHOLE).await;
    let with = list(h, Some("12"), true, WHOLE).await;

    assert!(without.is_empty(), "{without:?}");
    assert_eq!(codes(&with), ["M12"]);
}

async fn offset_and_limit_cut_the_page(h: &dyn Harness) {
    for (n, text) in [(21, "M1"), (22, "M2"), (23, "M3"), (24, "M4"), (25, "M5")] {
        insert(h, &machine(n, text)).await;
    }

    let first = list(h, None, false, (0, 3)).await;
    let middle = list(h, None, false, (1, 2)).await;
    let tail = list(h, None, false, (4, 10)).await;
    let beyond = list(h, None, false, (5, 10)).await;
    let none = list(h, None, false, (0, 0)).await;

    assert_eq!(codes(&first), ["M1", "M2", "M3"]);
    assert_eq!(codes(&middle), ["M2", "M3"]);
    assert_eq!(codes(&tail), ["M5"]);
    assert!(beyond.is_empty(), "{beyond:?}");
    assert!(none.is_empty(), "{none:?}");
}

async fn an_offset_above_bigint_is_a_storage_error_not_an_empty_list(h: &dyn Harness) {
    insert(h, &machine(21, "M1")).await;
    let largest = u64::try_from(i64::MAX).unwrap();
    let mut tx = begin(h).await;

    let at_limit = tx
        .plant()
        .list_machines(&MachineFilter::default(), largest, 10)
        .await;
    let beyond = tx
        .plant()
        .list_machines(
            &MachineFilter::default(),
            largest.checked_add(1).unwrap(),
            10,
        )
        .await;

    assert!(at_limit.unwrap().is_empty());
    assert!(matches!(
        beyond.unwrap_err(),
        StorageError::Other { sqlstate: None, .. }
    ));
}

contract_case!(
    an_offset_above_bigint_is_a_storage_error_not_an_empty_list,
    an_offset_above_bigint_is_a_storage_error_not_an_empty_list_memory,
    an_offset_above_bigint_is_a_storage_error_not_an_empty_list_postgres
);
contract_case!(
    machines_come_in_the_order_of_the_number_of_their_code,
    machines_come_in_the_order_of_the_number_of_their_code_memory,
    machines_come_in_the_order_of_the_number_of_their_code_postgres
);
contract_case!(
    no_search_text_lists_every_live_machine_with_its_columns,
    no_search_text_lists_every_live_machine_with_its_columns_memory,
    no_search_text_lists_every_live_machine_with_its_columns_postgres
);
contract_case!(
    a_search_in_other_case_finds_a_polish_name,
    a_search_in_other_case_finds_a_polish_name_memory,
    a_search_in_other_case_finds_a_polish_name_postgres
);
contract_case!(
    a_search_on_a_part_of_the_code_finds_the_code,
    a_search_on_a_part_of_the_code_finds_the_code_memory,
    a_search_on_a_part_of_the_code_finds_the_code_postgres
);
contract_case!(
    a_wildcard_character_of_the_search_matches_itself_only,
    a_wildcard_character_of_the_search_matches_itself_only_memory,
    a_wildcard_character_of_the_search_matches_itself_only_postgres
);
contract_case!(
    archived_machines_show_only_with_the_flag_and_follow_the_live_one,
    archived_machines_show_only_with_the_flag_and_follow_the_live_one_memory,
    archived_machines_show_only_with_the_flag_and_follow_the_live_one_postgres
);
contract_case!(
    the_search_also_applies_to_archived_machines_with_the_flag,
    the_search_also_applies_to_archived_machines_with_the_flag_memory,
    the_search_also_applies_to_archived_machines_with_the_flag_postgres
);
contract_case!(
    offset_and_limit_cut_the_page,
    offset_and_limit_cut_the_page_memory,
    offset_and_limit_cut_the_page_postgres
);
