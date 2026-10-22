//! The `forbidden` event of the refused pairs of the new routes of step P3.4 (design D10, task 14.1).
//!
//! This module is test code. The matrix rows of `routes::tests` check each route and each of the six
//! principals. `recipe_access_tests` checks the Operator column of the three areas with the shared
//! helper `assert_one_forbidden_event`. This module checks the other refused pairs of the same rows.
//! It reads the rows of the matrix, so it holds no second list of routes.

use super::matrix::{Endpoint, PRINCIPALS, Principal, Want, endpoint};
use super::recipe_access_tests::{area_rows, assert_one_forbidden_event};
use super::tests::p34_rows;

/// The number of the refused pairs that `recipe_access_tests` does not check.
const REFUSED_PAIRS: usize = 7;

/// A refused pair: a matrix endpoint and a principal whose answer is `403`.
struct RefusedPair {
    /// The endpoint of the matrix row.
    endpoint: Endpoint,
    /// The principal that the row refuses.
    principal: Principal,
}

/// Return the refused pairs of one matrix row.
fn refused_of_row((row, wants): (Endpoint, [Want; 6])) -> Vec<RefusedPair> {
    PRINCIPALS
        .into_iter()
        .zip(wants)
        .filter(|(_principal, want)| matches!(want, Want::Forbidden))
        .map(|(principal, _want)| RefusedPair {
            endpoint: endpoint(row.method.clone(), row.path, row.body),
            principal,
        })
        .collect()
}

/// Return the refused pairs that `recipe_access_tests` does not check.
///
/// The Operator pairs of the three areas are left to `recipe_access_tests`. The Operator pairs of the
/// P3.4 rows are included here.
fn refused_pairs() -> Vec<RefusedPair> {
    let area = area_rows()
        .into_iter()
        .flat_map(refused_of_row)
        .filter(|pair| !matches!(pair.principal, Principal::Operator));
    p34_rows()
        .into_iter()
        .flat_map(refused_of_row)
        .chain(area)
        .collect()
}

/// Each refused pair answers `403 E-WEB-002` and records exactly one `forbidden` event.
#[tokio::test]
async fn approval_access_a_refused_pair_records_one_forbidden_event() {
    let pairs = refused_pairs();
    assert_eq!(
        pairs.len(),
        REFUSED_PAIRS,
        "the refused pairs of the matrix rows that this module reads"
    );
    for pair in pairs {
        assert_one_forbidden_event(&pair.endpoint, pair.principal).await;
    }
}
