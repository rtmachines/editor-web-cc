//! The world of the history, audit, and security event page tests (design D13). Test code.
//!
//! The module solves one problem: the tests of the history page, the audit list, the audit entry,
//! and the security events need the same store. The store comes from `seed_approval_recipes`, so the
//! audit log holds the entries that the real use cases wrote. The world adds an Operator, who has
//! the number 3, and short senders for the three pages.

use recipe_domain::identity::Role;

use crate::test_support::plant::{HTMX, TARGET_CONTENT};
use crate::test_support::recipes::approval::ApprovalData;
use crate::test_support::{Fixture, Reply, TestSession, get_page, get_with};

/// The store with the approval data and a session for the Operator.
pub(crate) struct AuditWorld {
    /// The fixture with the router and the store.
    pub(crate) fixture: Fixture,
    /// The data of `seed_approval_recipes`: the Administrator `admin`, `anowak`, and `jsmith`.
    pub(crate) data: ApprovalData,
    /// The session of an Operator.
    pub(crate) operator: TestSession,
}

/// Build the world.
pub(crate) async fn audit_world() -> AuditWorld {
    let fixture = Fixture::new();
    let data = fixture.seed_approval_recipes().await;
    let operator = fixture.seed_session(3, Role::Operator, false);
    AuditWorld {
        fixture,
        data,
        operator,
    }
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
}
