//! Contract cases of `AuditReads::security_events` (task 5.2, ADR-0051 (h)).
//!
//! Most cases give each event its own client address. Then no two events share a row of the
//! aggregation per minute, also when the case runs across the end of a minute. Only the cases
//! of the aggregation record two events from one canonical address.

use std::net::IpAddr;

use recipe_app::identity::new_request_id;
use recipe_app::ports::{
    CursorPage, LoginMatch, SecurityEventFilter, SecurityEventId, SecurityEventRow, TimeRange,
};
use recipe_domain::audit::{LoginHmac, LoginSubject, SecurityEvent, SecurityEventKind, UserAgent};
use recipe_domain::identity::{KeyRing, Login, TypedLogin, UserId};
use recipe_domain::timestamp::Timestamp;
use serde_json::json;
use time::Date;

use super::fixtures::{
    WHOLE, address, begin_read, day_start, mapped_address, native_address, second_user, shifted,
};
use crate::contract::harness::Harness;

/// The current key of the login key ring of these cases.
const CURRENT_KEY: &str = "0202020202020202020202020202020202020202020202020202020202020202";
/// The older key of the login key ring of these cases.
const OLDER_KEY: &str = "0303030303030303030303030303030303030303030303030303030303030303";

/// Return the HMAC of the typed login `login` under `key`.
fn hmac(key: &str, login: &str) -> LoginHmac {
    LoginHmac::compute(
        &KeyRing::parse(key).unwrap(),
        &TypedLogin::parse(login).unwrap(),
    )
}

/// Return a `login_failed` event of an unknown login from `client`.
fn failed(hmac: LoginHmac, client: IpAddr) -> SecurityEvent {
    let request = new_request_id().unwrap();
    SecurityEvent::login_failed(LoginSubject::Unknown(hmac), request, client, None)
}

/// Return a `forbidden` event of `user` from `client` with the User-Agent `Agent/1`.
fn forbidden(user: UserId, client: IpAddr) -> SecurityEvent {
    let request = new_request_id().unwrap();
    let agent = Some(UserAgent::from("Agent/1"));
    SecurityEvent::forbidden(user, "ListSecurityEvents", request, Some(client), agent)
}

/// Record each event through the sink of the adapter, in order.
async fn record(h: &dyn Harness, events: Vec<SecurityEvent>) {
    let sink = h.security_events();
    for event in events {
        sink.record(event).await.unwrap();
    }
}

/// Read one page of the security events in a transaction that nothing commits.
async fn events(
    h: &dyn Harness,
    filter: &SecurityEventFilter,
    before: Option<SecurityEventId>,
    limit: u32,
) -> CursorPage<SecurityEventRow> {
    let mut tx = begin_read(h).await;
    tx.audit_reads()
        .security_events(filter, before, limit)
        .await
        .unwrap()
}

/// Return the client addresses of the rows, in the order of the rows.
fn clients(rows: &[SecurityEventRow]) -> Vec<Option<IpAddr>> {
    rows.iter().map(|row| row.client_ip).collect()
}

/// The filter `login` with the typed text `typed`. `login` is `None` outside the `Login` grammar.
fn of_login(typed: &str) -> SecurityEventFilter {
    SecurityEventFilter {
        login: Some(LoginMatch {
            login: Login::parse(typed).ok(),
            hmac: hmac(CURRENT_KEY, typed),
        }),
        ..SecurityEventFilter::default()
    }
}

async fn the_login_filter_matches_the_user_and_the_hmac_under_the_current_key(h: &dyn Harness) {
    let mhoffman = second_user(h, "mhoffman").await;
    let ghost = hmac(CURRENT_KEY, "ghost");
    record(
        h,
        vec![
            forbidden(mhoffman, address(1)),
            failed(ghost, address(2)),
            failed(ghost, address(3)),
            failed(hmac(OLDER_KEY, "ghost"), address(4)),
            failed(hmac(CURRENT_KEY, "mhoffman"), address(5)),
            failed(hmac(CURRENT_KEY, "jan@firma.pl"), address(6)),
        ],
    )
    .await;

    let of_ghost = events(h, &of_login("ghost"), None, WHOLE).await;
    let of_user = events(h, &of_login("mhoffman"), None, WHOLE).await;
    let of_mail = events(h, &of_login("jan@firma.pl"), None, WHOLE).await;

    assert_eq!(
        clients(&of_ghost.rows),
        [Some(address(3)), Some(address(2))]
    );
    assert_eq!(clients(&of_user.rows), [Some(address(5)), Some(address(1))]);
    assert_eq!(clients(&of_mail.rows), [Some(address(6))]);
    let row = &of_user.rows[1];
    assert_eq!(row.kind, SecurityEventKind::Forbidden);
    assert_eq!(row.count, 1);
    assert_eq!(row.login, Some(Login::parse("mhoffman").unwrap()));
    assert_eq!(row.user_agent, Some(UserAgent::from("Agent/1")));
    assert_eq!(
        row.detail,
        Some(json!({ "operation": "ListSecurityEvents" }))
    );
    assert_eq!(of_ghost.rows[0].login, None);
    assert_eq!(of_ghost.rows[0].detail, None);
    assert_eq!(of_ghost.rows[0].user_agent, None);
}

contract_case!(
    the_login_filter_matches_the_user_and_the_hmac_under_the_current_key,
    memory_the_login_filter_matches_the_user_and_the_hmac_under_the_current_key,
    postgres_the_login_filter_matches_the_user_and_the_hmac_under_the_current_key
);

async fn the_login_filter_and_the_kind_filter_must_both_match(h: &dyn Harness) {
    let mhoffman = second_user(h, "mhoffman").await;
    record(
        h,
        vec![
            forbidden(mhoffman, address(1)),
            failed(hmac(CURRENT_KEY, "mhoffman"), address(2)),
        ],
    )
    .await;
    let of_kind = |kind| SecurityEventFilter {
        kind: Some(kind),
        ..of_login("mhoffman")
    };

    let failed_rows = events(h, &of_kind(SecurityEventKind::LoginFailed), None, WHOLE).await;
    let forbidden_rows = events(h, &of_kind(SecurityEventKind::Forbidden), None, WHOLE).await;

    assert_eq!(clients(&failed_rows.rows), [Some(address(2))]);
    assert_eq!(clients(&forbidden_rows.rows), [Some(address(1))]);
}

contract_case!(
    the_login_filter_and_the_kind_filter_must_both_match,
    memory_the_login_filter_and_the_kind_filter_must_both_match,
    postgres_the_login_filter_and_the_kind_filter_must_both_match
);

/// Return the date, the hour, and the minute of `at` in UTC.
fn minute_of(at: Timestamp) -> (Date, u8, u8) {
    let time = at.as_offset_date_time();
    (time.date(), time.hour(), time.minute())
}

async fn two_failed_logins_from_one_address_in_one_minute_give_one_row(h: &dyn Harness) {
    let ghost = hmac(CURRENT_KEY, "ghost");
    record(
        h,
        vec![failed(ghost, address(8)), failed(ghost, address(8))],
    )
    .await;

    let rows = events(h, &SecurityEventFilter::default(), None, WHOLE)
        .await
        .rows;

    let counts: Vec<u32> = rows.iter().map(|row| row.count).collect();
    if let [newer, older] = rows.as_slice() {
        // The two events crossed the end of a minute, so each minute has its own row.
        assert_ne!(minute_of(newer.occurred_at), minute_of(older.occurred_at));
        assert_eq!(counts, [1, 1]);
    } else {
        assert_eq!(counts, [2]);
    }
}

contract_case!(
    two_failed_logins_from_one_address_in_one_minute_give_one_row,
    memory_two_failed_logins_from_one_address_in_one_minute_give_one_row,
    postgres_two_failed_logins_from_one_address_in_one_minute_give_one_row
);

/// Record two `forbidden` events and two `login_failed` events from the addresses 1 to 4.
async fn seed_four_events(h: &dyn Harness) {
    let ghost = hmac(CURRENT_KEY, "ghost");
    record(
        h,
        vec![
            forbidden(h.user(), address(1)),
            failed(ghost, address(2)),
            failed(ghost, address(3)),
            forbidden(h.user(), address(4)),
        ],
    )
    .await;
}

async fn the_event_range_keeps_from_and_drops_until(h: &dyn Harness) {
    seed_four_events(h).await;
    let all = events(h, &SecurityEventFilter::default(), None, WHOLE).await;
    let (newest, oldest) = (all.rows[0].occurred_at, all.rows[3].occurred_at);
    let in_range = |from, until| SecurityEventFilter {
        range: TimeRange { from, until },
        ..SecurityEventFilter::default()
    };

    let whole_days = in_range(Some(day_start(oldest, 0)), Some(day_start(newest, 1)));
    let from_oldest = in_range(Some(oldest), None);
    let after = in_range(Some(shifted(newest, 1)), None);
    let before = in_range(None, Some(oldest));

    assert_eq!(events(h, &whole_days, None, WHOLE).await.rows.len(), 4);
    assert_eq!(events(h, &from_oldest, None, WHOLE).await.rows.len(), 4);
    assert!(events(h, &after, None, WHOLE).await.rows.is_empty());
    assert!(events(h, &before, None, WHOLE).await.rows.is_empty());
}

contract_case!(
    the_event_range_keeps_from_and_drops_until,
    memory_the_event_range_keeps_from_and_drops_until,
    postgres_the_event_range_keeps_from_and_drops_until
);

async fn kind_client_and_cursor_keep_their_rows(h: &dyn Harness) {
    seed_four_events(h).await;
    let filter = |kind, client_ip| SecurityEventFilter {
        kind,
        client_ip,
        ..SecurityEventFilter::default()
    };
    let all = events(h, &SecurityEventFilter::default(), None, WHOLE).await;
    let first = events(h, &SecurityEventFilter::default(), None, 2).await;
    let rest = events(
        h,
        &SecurityEventFilter::default(),
        Some(first.rows[1].id),
        2,
    )
    .await;

    let failed_rows = events(
        h,
        &filter(Some(SecurityEventKind::LoginFailed), None),
        None,
        WHOLE,
    )
    .await;
    let at_three = events(h, &filter(None, Some(address(3))), None, WHOLE).await;

    assert_eq!(clients(&all.rows), [4, 3, 2, 1].map(|n| Some(address(n))));
    assert!(all.rows.windows(2).all(|pair| pair[0].id > pair[1].id));
    assert_eq!(
        clients(&failed_rows.rows),
        [Some(address(3)), Some(address(2))]
    );
    assert_eq!(clients(&at_three.rows), [Some(address(3))]);
    assert_eq!(clients(&first.rows), [Some(address(4)), Some(address(3))]);
    assert!(first.has_older);
    assert_eq!(clients(&rest.rows), [Some(address(2)), Some(address(1))]);
    assert!(!rest.has_older, "a last page with exactly `limit` rows");
}

contract_case!(
    kind_client_and_cursor_keep_their_rows,
    memory_kind_client_and_cursor_keep_their_rows,
    postgres_kind_client_and_cursor_keep_their_rows
);

/// The filter of one client address.
fn of_client(client_ip: IpAddr) -> SecurityEventFilter {
    SecurityEventFilter {
        client_ip: Some(client_ip),
        ..SecurityEventFilter::default()
    }
}

async fn an_event_from_a_mapped_address_lists_the_ipv4_address(h: &dyn Harness) {
    record(h, vec![forbidden(h.user(), mapped_address())]).await;

    let all = events(h, &SecurityEventFilter::default(), None, WHOLE).await;

    assert_eq!(clients(&all.rows), [Some(native_address())]);
}

contract_case!(
    an_event_from_a_mapped_address_lists_the_ipv4_address,
    memory_an_event_from_a_mapped_address_lists_the_ipv4_address,
    postgres_an_event_from_a_mapped_address_lists_the_ipv4_address
);

async fn the_client_filter_finds_an_event_from_a_mapped_address(h: &dyn Harness) {
    record(
        h,
        vec![
            forbidden(h.user(), mapped_address()),
            forbidden(h.user(), address(1)),
        ],
    )
    .await;

    let of_native = events(h, &of_client(native_address()), None, WHOLE).await;
    let of_mapped = events(h, &of_client(mapped_address()), None, WHOLE).await;

    assert_eq!(clients(&of_native.rows), [Some(native_address())]);
    assert_eq!(clients(&of_mapped.rows), [Some(native_address())]);
}

contract_case!(
    the_client_filter_finds_an_event_from_a_mapped_address,
    memory_the_client_filter_finds_an_event_from_a_mapped_address,
    postgres_the_client_filter_finds_an_event_from_a_mapped_address
);

async fn a_mapped_and_a_native_failed_login_in_one_minute_give_one_row(h: &dyn Harness) {
    let ghost = hmac(CURRENT_KEY, "ghost");
    record(
        h,
        vec![
            failed(ghost, mapped_address()),
            failed(ghost, native_address()),
        ],
    )
    .await;

    let rows = events(h, &SecurityEventFilter::default(), None, WHOLE)
        .await
        .rows;

    let counts: Vec<u32> = rows.iter().map(|row| row.count).collect();
    if let [newer, older] = rows.as_slice() {
        // The two events crossed the end of a minute, so each minute has its own row.
        assert_ne!(minute_of(newer.occurred_at), minute_of(older.occurred_at));
        assert_eq!(counts, [1, 1]);
    } else {
        assert_eq!(counts, [2]);
    }
    assert!(
        rows.iter()
            .all(|row| row.client_ip == Some(native_address()))
    );
}

contract_case!(
    a_mapped_and_a_native_failed_login_in_one_minute_give_one_row,
    memory_a_mapped_and_a_native_failed_login_in_one_minute_give_one_row,
    postgres_a_mapped_and_a_native_failed_login_in_one_minute_give_one_row
);

/// Record a failed login of `decoy` and then one of `target`, both from `address(n)`.
///
/// Return the single row of that address. Return `None` when the two events crossed the end of a
/// minute, because then each minute has its own row with the count 1. Any other result fails.
async fn aggregate_pair(
    h: &dyn Harness,
    n: u8,
    decoy: &str,
    target: &str,
) -> Option<SecurityEventRow> {
    let client = address(n);
    record(
        h,
        vec![
            failed(hmac(CURRENT_KEY, decoy), client),
            failed(hmac(CURRENT_KEY, target), client),
        ],
    )
    .await;
    let rows = events(h, &of_client(client), None, WHOLE).await.rows;
    match <[SecurityEventRow; 1]>::try_from(rows) {
        Ok([row]) => Some(row),
        Err(rows) => {
            let counts: Vec<u32> = rows.iter().map(|row| row.count).collect();
            assert_eq!(counts, [1, 1], "the rows of {client}: {rows:?}");
            assert_ne!(
                minute_of(rows[0].occurred_at),
                minute_of(rows[1].occurred_at)
            );
            None
        }
    }
}

/// Return the logins of the first try that fell into one minute, and its aggregated row.
///
/// Each try uses its own address and its own two logins, so a crossed try leaves no row that a
/// later filter finds.
async fn first_aggregated_pair(h: &dyn Harness) -> Option<(String, String, SecurityEventRow)> {
    for n in 20..23_u8 {
        let (decoy, target) = (format!("decoy{n}"), format!("target{n}"));
        if let Some(row) = aggregate_pair(h, n, &decoy, &target).await {
            return Some((decoy, target, row));
        }
    }
    None
}

/// Regression test of the limit that ADR-0051 (h) states. The filter `login` matches an
/// aggregated row by its first event only, and the filter `client_ip` finds every attempt.
async fn an_aggregated_failed_login_row_keeps_the_login_of_its_first_event(h: &dyn Harness) {
    let (decoy, target, row) = first_aggregated_pair(h)
        .await
        .expect("one of three tries stays inside one minute");

    let of_decoy = events(h, &of_login(&decoy), None, WHOLE).await;
    let of_target = events(h, &of_login(&target), None, WHOLE).await;

    assert_eq!(row.count, 2);
    assert_eq!(of_decoy.rows.len(), 1);
    assert_eq!(of_decoy.rows[0].id, row.id);
    assert_eq!(of_decoy.rows[0].count, 2);
    assert!(of_target.rows.is_empty());
}

contract_case!(
    an_aggregated_failed_login_row_keeps_the_login_of_its_first_event,
    memory_an_aggregated_failed_login_row_keeps_the_login_of_its_first_event,
    postgres_an_aggregated_failed_login_row_keeps_the_login_of_its_first_event
);
