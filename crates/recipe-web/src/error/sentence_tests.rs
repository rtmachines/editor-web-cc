//! Tests of the English fallback of the error sentence (low-round-c, review fix R6).
//!
//! The fallback runs when the bundle of the UI language has no key `error-<code>`.
//! `E-AUD-002` has no key, because no user request returns it (`reliability.md` section 1.2).

use fluent::FluentArgs;
use recipe_app::error::{AppError, AuditError};
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use uuid::Uuid;

use super::WebError;
use super::answer::{ErrorContext, sentence_of};
use crate::i18n::Lang;
use crate::test_support::state;

#[test]
fn sentence_code_without_a_bundle_key_falls_back_to_the_english_display() {
    let state = state();
    let key = format!("error-{}", ErrorCode::AUD_002);
    for lang in [Lang::Pl, Lang::En] {
        assert!(state.bundles().message(lang, &key, None).is_none(), "{key}");
    }
    let error = WebError::App(AppError::Audit(AuditError::ChainBroken { chain_seq: 7 }));
    let ctx = ErrorContext {
        request_id: RequestId::from(Uuid::nil()),
        lang: Lang::Pl,
        htmx: true,
        retry: None,
        state: &state,
    };

    let sentence = sentence_of(&error, ErrorCode::AUD_002, &FluentArgs::new(), &ctx);

    assert_eq!(
        sentence,
        "Audit entry 7 does not match the hash chain. Contact the administrator."
    );
}
