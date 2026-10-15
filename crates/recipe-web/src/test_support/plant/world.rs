//! The world of the page tests of the plant model (design D12, D20 U13). This module is test code.
//!
//! The module solves one problem: the tests of the machine pages, the unit pages, and the type pages
//! need the same store, the same three sessions, and the same senders. One world gives them, so no
//! area repeats `World`, `get`, `post`, `cookie_of`, or `main_of`. An area adds only what it alone
//! needs, for example the extra machines of the unit pages.

use recipe_domain::identity::Role;

use super::PlantIds;
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::test_support::{Fixture, Reply, TestSession, get_with, post_fields};

/// The header of an HTMX request.
pub(crate) const HTMX: (&str, &str) = ("hx-request", "true");
/// The header of an HTMX request that targets `main`.
pub(crate) const TARGET_CONTENT: (&str, &str) = ("hx-target", "content");
/// The headers of an HTMX request that targets `main`.
pub(crate) const HTMX_CONTENT: [(&str, &str); 2] = [HTMX, TARGET_CONTENT];
/// The cookie that selects the English texts.
pub(crate) const ENGLISH: (&str, &str) = ("cookie", "recipe_lang=en");
/// The cookie that selects the Polish texts.
pub(crate) const POLISH: (&str, &str) = ("cookie", "recipe_lang=pl");

/// Return the language cookie header of `lang`.
pub(crate) fn cookie_of(lang: Lang) -> (&'static str, &'static str) {
    match lang {
        Lang::En => ENGLISH,
        Lang::Pl => POLISH,
    }
}

/// The store of `seed_plant` and a session for each of the three roles.
pub(crate) struct World {
    /// The fixture with the router and the store.
    pub(crate) fixture: Fixture,
    /// The fixed identifiers of the plant data.
    pub(crate) ids: PlantIds,
    /// The session of the Administrator `user1`.
    pub(crate) admin: TestSession,
    /// The session of the ProcessEngineer `user2`.
    pub(crate) engineer: TestSession,
    /// The session of the Operator `user3`.
    pub(crate) operator: TestSession,
}

/// Build the world: three sessions and the plant data of design D12.
pub(crate) fn world() -> World {
    let fixture = Fixture::new();
    let admin = fixture.seed_session(1, Role::Administrator, false);
    let engineer = fixture.seed_session(2, Role::ProcessEngineer, false);
    let operator = fixture.seed_session(3, Role::Operator, false);
    let ids = fixture.seed_plant();
    World {
        fixture,
        ids,
        admin,
        engineer,
        operator,
    }
}

impl World {
    /// Send `GET path` as `session` with the extra headers. A test adds a language cookie.
    pub(crate) async fn get(
        &self,
        session: &TestSession,
        path: &str,
        headers: &[(&str, &str)],
    ) -> Reply {
        let mut all = vec![("cookie", session.cookie.as_str())];
        all.extend_from_slice(headers);
        get_with(&self.fixture.router(), path, &all).await
    }

    /// Send `GET path` as `session` in English.
    pub(crate) async fn get_en(&self, session: &TestSession, path: &str) -> Reply {
        self.get(session, path, &[ENGLISH]).await
    }

    /// Send `GET path` as an HTMX request that targets `main`, in English.
    pub(crate) async fn get_content(&self, session: &TestSession, path: &str) -> Reply {
        self.get(session, path, &[ENGLISH, HTMX, TARGET_CONTENT])
            .await
    }

    /// Send the form `body` to `path` as `session` in English, with the extra headers.
    pub(crate) async fn post_with(
        &self,
        session: &TestSession,
        path: &str,
        body: &str,
        extra: &[(&str, &str)],
    ) -> Reply {
        let mut headers = vec![ENGLISH];
        headers.extend_from_slice(extra);
        post_fields(&self.fixture.router(), path, Some(session), &headers, body).await
    }

    /// Send `POST path` as `session` with the form `body` in English. `htmx` adds `HX-Request`.
    pub(crate) async fn post(
        &self,
        session: &TestSession,
        path: &str,
        body: &str,
        htmx: bool,
    ) -> Reply {
        let extra: &[(&str, &str)] = if htmx { &[HTMX] } else { &[] };
        self.post_with(session, path, body, extra).await
    }

    /// Return the number of the audit entries in the store.
    pub(crate) fn audit_count(&self) -> usize {
        self.fixture.store.audit_entries().len()
    }
}

impl Fixture {
    /// Return a text that changes with every write: the committed rows and the audit entries.
    ///
    /// A refused request must leave it as it was.
    pub(crate) fn snapshot(&self) -> (String, usize) {
        (
            format!("{:?}", self.store.state()),
            self.store.audit_entries().len(),
        )
    }
}

/// Return the text of `key` in `lang` with the argument `name` set to `value`.
pub(crate) fn text_with(lang: Lang, key: &str, name: &str, value: &str) -> String {
    let bundles = Bundles::load().unwrap();
    Texts::new(&bundles, lang).with_arg(key, name, value)
}

/// Return the markup of `main`.
pub(crate) fn main_of(body: &str) -> &str {
    let (_before, rest) = body.split_once("<main").unwrap();
    rest.split("</main>").next().unwrap()
}
