//! Unit tests of the mapping functions of the audit chain adapter (ADR-0086 (g)).
//!
//! No SQL statement gives the shapes that these tests feed in, so only a unit test reaches the
//! refusals. Each bad tuple must give `E-DB-008`, must name the table, and must not show a hash.
//! A unit test may use `unwrap`, `expect`, and indexing (`reliability.md` section 2.3).

use std::error::Error as _;
use std::fmt::Debug;

use recipe_app::error::StorageError;
use recipe_app::ports::{BatchOutcome, ChainBounds, ChainPosition, ChainSeq};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::Sha256;

use super::{
    BatchRow, HEAD_TABLE, HeadRow, LOG_TABLE, bounds, head_of, outcome_of, position, row_position,
};

/// The fill byte of a good hash in the tests that expect a value.
const GOOD_FILL: u8 = 0xAB;

/// The two fill bytes of the disclosure check. Both differ in every encoding of a byte.
const FILLS: [u8; 2] = [0x12, 0xED];

/// The length of a good hash.
const GOOD_LEN: usize = 32;

fn good_hash() -> Vec<u8> {
    vec![GOOD_FILL; GOOD_LEN]
}

fn good_sha() -> Sha256 {
    Sha256::from([GOOD_FILL; 32])
}

/// A hash of `len` bytes that all equal `fill`, or no hash.
fn hash_of(fill: u8, len: Option<usize>) -> Option<Vec<u8>> {
    len.map(|len| vec![fill; len])
}

fn at(seq: i64, hash: Option<Sha256>) -> ChainPosition {
    ChainPosition {
        seq: ChainSeq::from(seq),
        hash,
    }
}

/// Refuse `case`, check `E-DB-008`, and collect every text of the error.
///
/// The texts are `Display`, `Debug`, and the `Display` of every error in the source chain.
fn refusal_texts<T: Debug>(result: Result<T, StorageError>, case: &str) -> Vec<String> {
    let error = result.expect_err(case);
    assert_eq!(error.code(), ErrorCode::DB_008, "{case}");
    let mut texts = vec![error.to_string(), format!("{error:?}")];
    let mut cause = error.source();
    while let Some(inner) = cause {
        texts.push(inner.to_string());
        cause = inner.source();
    }
    texts
}

/// Assert `E-DB-008`, the table name in the cause, and no hash byte in any text of the error.
///
/// `build` gets a fill byte and makes the malformed input from it. The helper runs `build` with
/// two different fill bytes. Both inputs have the same length. Equal texts prove that no byte
/// of the hash reaches any text, in any encoding.
fn assert_refused<T: Debug>(
    build: impl Fn(u8) -> Result<T, StorageError>,
    table: &str,
    case: &str,
) {
    let [first_fill, second_fill] = FILLS;
    let first = refusal_texts(build(first_fill), case);
    let second = refusal_texts(build(second_fill), case);
    assert_eq!(first, second, "{case}: a text depends on the hash bytes");
    assert!(
        first.iter().any(|text| text.contains(table)),
        "{case}: no text names the table {table}"
    );
}

#[test]
fn position_refuses_a_negative_seq_and_a_hash_that_is_not_32_bytes() {
    let cases = [
        ("negative seq without a hash", -1, None),
        ("negative seq with a hash", -7, Some(GOOD_LEN)),
        ("hash of 31 bytes", 5, Some(31)),
        ("hash of 33 bytes", 5, Some(33)),
        ("hash of 0 bytes", 5, Some(0)),
    ];
    for (case, seq, len) in cases {
        let build = |fill| position(HEAD_TABLE, seq, hash_of(fill, len));
        assert_refused(build, HEAD_TABLE, case);
    }
}

#[test]
fn position_returns_a_permitted_shape_as_read() {
    let cases = [
        ("start", 0, None, at(0, None)),
        ("head after the start without a hash", 4, None, at(4, None)),
        (
            "head with a hash",
            4,
            Some(good_hash()),
            at(4, Some(good_sha())),
        ),
        (
            "hash at the start",
            0,
            Some(good_hash()),
            at(0, Some(good_sha())),
        ),
    ];
    for (case, seq, hash, expected) in cases {
        assert_eq!(position(HEAD_TABLE, seq, hash).unwrap(), expected, "{case}");
    }
}

#[test]
fn head_of_refuses_a_bad_head_a_single_null_bound_and_a_bad_anchor() {
    let good = Some(GOOD_LEN);
    let cases = [
        ("negative last_seq", -1, None, None, None, None, HEAD_TABLE),
        (
            "short hash",
            3,
            Some(31),
            Some(1),
            Some(3),
            None,
            HEAD_TABLE,
        ),
        ("only min is NULL", 3, good, None, Some(3), None, LOG_TABLE),
        ("only max is NULL", 3, good, Some(1), None, None, LOG_TABLE),
        (
            "bad anchor text",
            3,
            good,
            Some(1),
            Some(3),
            Some("x"),
            "app_setting",
        ),
    ];
    for (case, seq, len, min, max, anchor, table) in cases {
        let build = |fill| {
            let row: HeadRow = (seq, hash_of(fill, len), min, max, anchor.map(Into::into));
            head_of(row)
        };
        assert_refused(build, table, case);
    }
}

#[test]
fn head_of_returns_a_head_after_the_start_without_a_hash_as_read() {
    let head = head_of((6, None, Some(1), Some(6), Some("4".into()))).unwrap();
    assert_eq!(head.position, at(6, None));
    let bounds = ChainBounds {
        min: ChainSeq::from(1),
        max: ChainSeq::from(6),
    };
    assert_eq!(head.bounds, Some(bounds));
    assert_eq!(head.anchor, ChainSeq::from(4));
}

#[test]
fn head_of_returns_an_empty_chain_as_the_start() {
    let head = head_of((0, None, None, None, None)).unwrap();
    assert_eq!(head.position, ChainPosition::START);
    assert_eq!(head.bounds, None);
    assert_eq!(head.anchor, ChainSeq::ZERO);
}

#[test]
fn row_position_refuses_a_row_after_the_start_without_a_hash() {
    let cases = [
        ("row 1 without a hash", 1, None),
        ("row 9 without a hash", 9, None),
        ("negative seq", -3, Some(GOOD_LEN)),
        ("hash of 31 bytes", 2, Some(31)),
    ];
    for (case, seq, len) in cases {
        let build = |fill| row_position(seq, hash_of(fill, len));
        assert_refused(build, LOG_TABLE, case);
    }
}

#[test]
fn row_position_returns_a_good_row_and_the_start() {
    assert_eq!(
        row_position(2, Some(good_hash())).unwrap(),
        at(2, Some(good_sha()))
    );
    assert_eq!(row_position(0, None).unwrap(), ChainPosition::START);
}

#[test]
fn bounds_refuse_exactly_one_null() {
    for (case, min, max) in [("min only", Some(1), None), ("max only", None, Some(9))] {
        assert_refused(|_fill| bounds(min, max), LOG_TABLE, case);
    }
}

#[test]
fn bounds_return_none_for_two_nulls_and_pass_every_value_through() {
    assert_eq!(bounds(None, None).unwrap(), None);
    let cases = [(1, 9), (0, 9), (-4, 9), (-4, -2), (3, 3)];
    for (min, max) in cases {
        let expected = ChainBounds {
            min: ChainSeq::from(min),
            max: ChainSeq::from(max),
        };
        assert_eq!(
            bounds(Some(min), Some(max)).unwrap(),
            Some(expected),
            "{min}..{max}"
        );
    }
}

#[test]
fn outcome_of_refuses_a_break_without_last_clean_and_a_bad_clean_batch() {
    let good = Some(GOOD_LEN);
    let cases = [
        ("break without last_clean", Some(4), 3, Some(6), good),
        ("last row without a hash", None, 3, Some(6), None),
        ("rows without a last row", None, 2, None, None),
        ("negative row count", None, -1, None, None),
        ("negative last seq", None, 1, Some(-2), good),
        ("last row with a short hash", None, 1, Some(2), Some(31)),
    ];
    for (case, first_break, rows, last_seq, len) in cases {
        let build = |fill| {
            let row: BatchRow = (first_break, None, rows, last_seq, hash_of(fill, len));
            outcome_of(row)
        };
        assert_refused(build, LOG_TABLE, case);
    }
}

#[test]
fn outcome_of_maps_a_break_to_broken_and_no_break_to_clean() {
    let broken = outcome_of((Some(4), Some(2), 3, Some(6), Some(good_hash()))).unwrap();
    let expected = BatchOutcome::Broken {
        first_break: ChainSeq::from(4),
        last_clean: ChainSeq::from(2),
    };
    assert_eq!(broken, expected);
    let empty = outcome_of((None, None, 0, None, None)).unwrap();
    assert_eq!(
        empty,
        BatchOutcome::Clean {
            rows: 0,
            last: None
        }
    );
    let full = outcome_of((None, None, 3, Some(6), Some(good_hash()))).unwrap();
    assert_eq!(
        full,
        BatchOutcome::Clean {
            rows: 3,
            last: Some(at(6, Some(good_sha())))
        }
    );
    let with_last_clean = outcome_of((None, Some(2), 0, None, None)).unwrap();
    assert_eq!(
        with_last_clean,
        BatchOutcome::Clean {
            rows: 0,
            last: None
        }
    );
}
