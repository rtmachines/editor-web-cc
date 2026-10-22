//! The world of the history, audit, and security event page tests (design D13). Test code.
//!
//! The module solves one problem: the tests of the history page, the audit list, the audit entry,
//! and the security events need the same store. The store comes from `seed_approval_recipes`, so the
//! audit log holds the entries that the real use cases wrote. The world adds the Administrator
//! `user1`, who created `R-100`, `R-200`, and `R-300`, and an Operator, who has the number 3. It
//! also adds short senders for the three pages and the page helpers that the three page test files
//! share. The diff, approval, and version page tests also use the world. The methods
//! `stored_of` and `row_version_of` read the store for them.

use axum::http::StatusCode;
use recipe_app::blocking::{AppSettings, PageSizes};
use recipe_app::ports::{Isolation, Transaction as _};
use recipe_domain::identity::Role;
use recipe_domain::recipe::{RecipeVersionId, ReferenceNumber, StoredParameter};

use crate::test_support::plant::{HTMX, TARGET_CONTENT, main_of};
use crate::test_support::recipes::approval::ApprovalData;
use crate::test_support::{Fixture, Reply, TestSession, get_page, get_with};

/// The first cell of a row: the time in a span that does not break (finding DR-3).
pub(crate) const TIME_CELL: &str = "<tr><td><span class=\"data time\">";

/// The store with the approval data, the stored creator `user1`, and a session for the Operator.
pub(crate) struct AuditWorld {
    /// The fixture with the router and the store.
    pub(crate) fixture: Fixture,
    /// The data of `seed_approval_recipes`: the Administrator `admin`, `anowak`, and `jsmith`.
    pub(crate) data: ApprovalData,
    /// The session of an Operator.
    pub(crate) operator: TestSession,
}

/// Build the world with the default page sizes.
pub(crate) async fn audit_world() -> AuditWorld {
    audit_world_with(AppSettings::default().pages).await
}

/// Build the world with the page sizes `pages`.
///
/// The paging tests use it with a small page. Every world seeds `user1`, the creator of `R-100`,
/// `R-200`, and `R-300`.
pub(crate) async fn audit_world_with(pages: PageSizes) -> AuditWorld {
    let fixture = Fixture::with_app_settings(AppSettings {
        pages,
        ..AppSettings::default()
    });
    let data = fixture.seed_approval_recipes().await;
    seed_recipe_creator(&fixture);
    let operator = fixture.seed_session(3, Role::Operator, false);
    AuditWorld {
        fixture,
        data,
        operator,
    }
}

/// The number of the creator of `R-100`, `R-200`, and `R-300`, whom `seed_recipes` names.
const RECIPE_CREATOR_NUMBER: u8 = 1;
/// A valid password for the creator. No test logs in as the creator.
const CREATOR_PASSWORD: &str = "correct horse battery";

/// Store the Administrator `user1`, the creator of `R-100`, `R-200`, and `R-300`.
///
/// `seed_recipes` stores no row of this user. A page that reads the creator login of those
/// versions answers `500 E-DB-008` without this row. The user has no session.
fn seed_recipe_creator(fixture: &Fixture) {
    fixture.seed_account(
        RECIPE_CREATOR_NUMBER,
        Role::Administrator,
        CREATOR_PASSWORD,
        false,
    );
}

/// The number of rows in the body of the table in `main`.
pub(crate) fn body_rows(main: &str) -> usize {
    main.matches(TIME_CELL).count()
}

/// Return the ids of the entries that `main` links, in the order of the rows.
pub(crate) fn entry_ids(main: &str) -> Vec<u64> {
    main.split("href=\"/audit/")
        .skip(1)
        .filter_map(|rest| rest.split('"').next()?.parse().ok())
        .collect()
}

/// Return the address of the older page that the pager of `main` offers.
pub(crate) fn older_href(main: &str) -> String {
    let (head, _tail) = main.split_once("\">Older entries</a>").unwrap();
    let (_before, href) = head.rsplit_once("href=\"").unwrap();
    href.replace("&#38;", "&")
}

/// Return the query of `href` as sorted pairs of name and value.
pub(crate) fn query_pairs(href: &str) -> Vec<(String, String)> {
    let (_path, query) = href.split_once('?').unwrap();
    let mut pairs: Vec<(String, String)> = query
        .split('&')
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap();
            (name.to_owned(), value.to_owned())
        })
        .collect();
    pairs.sort();
    pairs
}

/// Read `path` as the Administrator. Return the markup of `main`.
pub(crate) async fn admin_main(world: &AuditWorld, path: &str) -> String {
    let reply = world.get_en(&world.data.admin, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    main_of(&reply.body).to_owned()
}

/// Return the content of the object cell (the last plain `td`) of each table row in `main`.
fn object_cells(main: &str) -> Vec<&str> {
    main.split("<tr>")
        .skip(1)
        .filter_map(|row| {
            let (_before, cell) = row.rsplit_once("<td>")?;
            cell.split_once("</td>").map(|(content, _after)| content)
        })
        .collect()
}

/// Split an object cell into its type word and the label inside `<span class="data">`.
///
/// Return `None` when the cell does not have the form `word <span class="data">label</span>`.
fn word_and_label(cell: &str) -> Option<(&str, &str)> {
    let (word, rest) = cell.split_once(" <span class=\"data\">")?;
    let label = rest.strip_suffix("</span>")?;
    (!word.contains('<') && !label.contains('<')).then_some((word, label))
}

/// Return true when `label` is the short id of a row that is gone: eight hexadecimal digits.
fn is_short_id(label: &str) -> bool {
    label.len() == 8 && label.chars().all(|letter| letter.is_ascii_hexdigit())
}

/// Assert that every object cell of `main` shows its type word outside `<span class="data">` and
/// its label or short id inside it (finding CX-DR-2). The cells must hold a recipe row, a version
/// row, and a row with a missing label, which shows the short id.
pub(crate) fn assert_object_cells_mark_the_label(
    main: &str,
    recipe_word: &str,
    version_word: &str,
) {
    let cells = object_cells(main);
    let pairs: Vec<(&str, &str)> = cells
        .iter()
        .filter_map(|cell| word_and_label(cell))
        .collect();
    assert_eq!(
        pairs.len(),
        cells.len(),
        "a cell without the span: {cells:?}"
    );
    assert!(pairs.contains(&(recipe_word, "R-500")), "{cells:?}");
    assert!(
        pairs
            .iter()
            .any(|(word, label)| *word == version_word && label.parse::<u32>().is_ok()),
        "{cells:?}"
    );
    assert!(
        pairs.iter().any(|(_word, label)| is_short_id(label)),
        "{cells:?}"
    );
}

impl AuditWorld {
    /// Send `GET path` as `session` in the language `code`.
    pub(crate) async fn get(&self, session: &TestSession, code: &str, path: &str) -> Reply {
        get_page(&self.fixture, Some(session), code, path).await
    }

    /// Send `GET path` as `session` in English.
    pub(crate) async fn get_en(&self, session: &TestSession, path: &str) -> Reply {
        self.get(session, "en", path).await
    }

    /// Send `GET path` as an HTMX request that targets `main`, in English.
    pub(crate) async fn get_content(&self, session: &TestSession, path: &str) -> Reply {
        let cookie = format!("{}; recipe_lang=en", session.cookie);
        get_with(
            &self.fixture.router(),
            path,
            &[("cookie", cookie.as_str()), HTMX, TARGET_CONTENT],
        )
        .await
    }

    /// Return the stored parameter `tag` of `version`.
    pub(crate) async fn stored_of(&self, version: RecipeVersionId, tag: &str) -> StoredParameter {
        let mut tx = self
            .fixture
            .store
            .begin_memory(Isolation::ReadOnly)
            .unwrap();
        let stored = tx.recipes().version_parameters(version).await.unwrap();
        drop(tx);
        stored
            .into_iter()
            .find(|parameter| parameter.tag.as_str() == tag)
            .unwrap()
    }

    /// Return the `row_version` that the store holds for version `number` of the recipe
    /// `reference`.
    pub(crate) async fn row_version_of(&self, reference: &str, number: u32) -> i64 {
        let reference = ReferenceNumber::parse(reference).unwrap();
        let recipe = self
            .fixture
            .store
            .begin_memory(Isolation::ReadOnly)
            .unwrap()
            .recipes()
            .recipe_by_reference(&reference)
            .await
            .unwrap()
            .unwrap();
        let header = recipe
            .versions()
            .iter()
            .find(|header| header.number() == number)
            .unwrap();
        header.row_version()
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::audit_world;

    // The world seeds `R-200` and `R-300` through `seed_recipes`. That seed names the user number 1
    // as the creator. Without a row of that user, the version page answers `500 E-DB-008`.
    #[tokio::test]
    async fn the_version_pages_of_r200_and_r300_answer_200_because_the_world_seeds_their_creator() {
        let world = audit_world().await;

        for path in ["/recipes/R-200/versions/1", "/recipes/R-300/versions/1"] {
            let reply = world.get_en(&world.data.admin, path).await;

            assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
        }
    }
}
