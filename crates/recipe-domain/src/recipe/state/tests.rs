use super::Transition::{Approve, Discard, Edit, Retire};
use super::VersionState::{Approved, Discarded, LiveDraft, Retired};
use super::*;

#[test]
fn apply_accepts_four_pairs_and_forbids_the_other_twelve() {
    let cases = [
        (LiveDraft, Approve, Ok(Approved)),
        (
            LiveDraft,
            Retire,
            Err(ForbiddenTransition {
                from: LiveDraft,
                step: Retire,
            }),
        ),
        (LiveDraft, Discard, Ok(Discarded)),
        (LiveDraft, Edit, Ok(LiveDraft)),
        (
            Discarded,
            Approve,
            Err(ForbiddenTransition {
                from: Discarded,
                step: Approve,
            }),
        ),
        (
            Discarded,
            Retire,
            Err(ForbiddenTransition {
                from: Discarded,
                step: Retire,
            }),
        ),
        (
            Discarded,
            Discard,
            Err(ForbiddenTransition {
                from: Discarded,
                step: Discard,
            }),
        ),
        (
            Discarded,
            Edit,
            Err(ForbiddenTransition {
                from: Discarded,
                step: Edit,
            }),
        ),
        (
            Approved,
            Approve,
            Err(ForbiddenTransition {
                from: Approved,
                step: Approve,
            }),
        ),
        (Approved, Retire, Ok(Retired)),
        (
            Approved,
            Discard,
            Err(ForbiddenTransition {
                from: Approved,
                step: Discard,
            }),
        ),
        (
            Approved,
            Edit,
            Err(ForbiddenTransition {
                from: Approved,
                step: Edit,
            }),
        ),
        (
            Retired,
            Approve,
            Err(ForbiddenTransition {
                from: Retired,
                step: Approve,
            }),
        ),
        (
            Retired,
            Retire,
            Err(ForbiddenTransition {
                from: Retired,
                step: Retire,
            }),
        ),
        (
            Retired,
            Discard,
            Err(ForbiddenTransition {
                from: Retired,
                step: Discard,
            }),
        ),
        (
            Retired,
            Edit,
            Err(ForbiddenTransition {
                from: Retired,
                step: Edit,
            }),
        ),
    ];
    for (from, step, expected) in cases {
        assert_eq!(from.apply(step), expected, "{from:?} with {step:?}");
    }
}

#[test]
fn forbidden_transition_keeps_its_state_and_step() {
    let forbidden = Retired.apply(Approve).unwrap_err();
    assert_eq!(forbidden.state(), Retired);
    assert_eq!(forbidden.step(), Approve);
}

#[test]
fn forbidden_transition_display_uses_words_not_debug() {
    let forbidden = Approved.apply(Approve).unwrap_err();
    assert_eq!(
        forbidden.to_string(),
        "Cannot approve a version that is approved."
    );
}

// domain-model.md section 3: the four allowed pairs have no error, so the other twelve pairs
// give twelve texts. Each text names the step and the state in words, not in Rust names.
#[test]
fn forbidden_transition_display_names_each_step_and_state_in_words() {
    let steps = [
        (Approve, "approve"),
        (Retire, "retire"),
        (Discard, "discard"),
        (Edit, "edit"),
    ];
    let states = [
        (LiveDraft, "a live draft"),
        (Discarded, "discarded"),
        (Approved, "approved"),
        (Retired, "retired"),
    ];
    let mut texts = Vec::new();
    for (state, state_words) in states {
        for (step, step_word) in steps {
            let Err(forbidden) = state.apply(step) else {
                continue;
            };
            let text = forbidden.to_string();
            assert!(
                text.starts_with(&format!("Cannot {step_word} ")),
                "{state:?} with {step:?}: {text}"
            );
            assert!(
                text.ends_with(&format!(" is {state_words}.")),
                "{state:?} with {step:?}: {text}"
            );
            for rust_name in [
                "LiveDraft",
                "Discarded",
                "Approved",
                "Retired",
                "Transition",
            ] {
                assert!(!text.contains(rust_name), "{text}");
            }
            texts.push(text);
        }
    }
    let distinct: std::collections::BTreeSet<&String> = texts.iter().collect();
    assert_eq!(texts.len(), 12);
    assert_eq!(distinct.len(), 12);
}

#[test]
fn status_of_each_state() {
    assert_eq!(LiveDraft.status(), VersionStatus::Draft);
    assert_eq!(Discarded.status(), VersionStatus::Draft);
    assert_eq!(Approved.status(), VersionStatus::Approved);
    assert_eq!(Retired.status(), VersionStatus::Retired);
}

#[test]
fn from_stored_maps_the_six_inputs() {
    assert_eq!(
        VersionState::from_stored(VersionStatus::Draft, false),
        Ok(LiveDraft)
    );
    assert_eq!(
        VersionState::from_stored(VersionStatus::Draft, true),
        Ok(Discarded)
    );
    assert_eq!(
        VersionState::from_stored(VersionStatus::Approved, false),
        Ok(Approved)
    );
    assert_eq!(
        VersionState::from_stored(VersionStatus::Retired, false),
        Ok(Retired)
    );
    assert_eq!(
        VersionState::from_stored(VersionStatus::Approved, true),
        Err(VersionStateReason::DeletedNotDraft)
    );
    assert_eq!(
        VersionState::from_stored(VersionStatus::Retired, true),
        Err(VersionStateReason::DeletedNotDraft)
    );
}

#[test]
fn deleted_not_draft_has_its_phrase() {
    assert_eq!(
        VersionStateReason::DeletedNotDraft.to_string(),
        "only a draft can be deleted"
    );
}

#[test]
fn as_sql_gives_the_three_column_words() {
    assert_eq!(VersionStatus::Draft.as_sql(), "draft");
    assert_eq!(VersionStatus::Approved.as_sql(), "approved");
    assert_eq!(VersionStatus::Retired.as_sql(), "retired");
}

#[test]
fn parse_accepts_the_three_column_words() {
    assert_eq!(VersionStatus::parse("draft"), Ok(VersionStatus::Draft));
    assert_eq!(
        VersionStatus::parse("approved"),
        Ok(VersionStatus::Approved)
    );
    assert_eq!(VersionStatus::parse("retired"), Ok(VersionStatus::Retired));
}

#[test]
fn parse_rejects_other_text() {
    assert_eq!(
        VersionStatus::parse("Draft"),
        Err(VersionStatusReason::Unknown)
    );
    assert_eq!(VersionStatus::parse(""), Err(VersionStatusReason::Unknown));
}

#[test]
fn display_of_status_is_the_column_word() {
    assert_eq!(VersionStatus::Draft.to_string(), "draft");
    assert_eq!(VersionStatus::Approved.to_string(), "approved");
    assert_eq!(VersionStatus::Retired.to_string(), "retired");
}

#[test]
fn unknown_status_reason_has_its_phrase() {
    assert_eq!(
        VersionStatusReason::Unknown.to_string(),
        "the status is not draft, approved, or retired"
    );
}
