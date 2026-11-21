//! Tests of `CreateVersion` for the rule "WSTRING characters on CODESYS and TwinCAT"
//! (ADR-0201 (h)): the copy of a base keeps a stored value of the earlier rule set.

use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use serde_json::json;

use super::tests::{parameters_of, write_journal};
use super::{CreateVersion, CreateVersionInput};
use crate::memory::fixtures::{Fixture, actor, long_text, meta, reference, tid};
use crate::recipe::test_support::{NOTE_TYPE, note_parameter, wstring_row};

#[tokio::test]
async fn adr_0201_a_base_with_a_character_outside_the_basic_plane_is_copied_unchanged() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let fixture = Fixture::new();
        let machine = fixture.seed_machine_on(platform, 1, "M1");
        fixture
            .store
            .seed_type(&wstring_row(NOTE_TYPE, platform, 10));
        let note = note_parameter(platform, 10, json!("A😀"));
        let (_, _, approved) = fixture.seed_retired(&machine, 1, "R-1");
        fixture.store.seed_parameter(approved, &note);
        let output = CreateVersion::new(fixture.context())
            .execute(
                &actor(Role::ProcessEngineer),
                &meta(),
                CreateVersionInput {
                    reference: reference("R-1"),
                    based_on: Some(2),
                    change_note: long_text("Next version"),
                },
            )
            .await
            .unwrap();
        assert_eq!(output.number, 3, "{platform:?}");
        assert_eq!(
            fixture.store.journal(),
            write_journal(&[102]),
            "{platform:?}"
        );
        let copy = parameters_of(&fixture, output.version).await;
        assert_eq!(copy.len(), 1, "{platform:?}");
        let copied = copy.first().unwrap();
        assert_eq!(copied.plc_type, tid(NOTE_TYPE), "{platform:?}");
        assert_eq!(copied.value, note.value, "{platform:?}");
        assert_eq!(copied.value.as_json(), &json!("A😀"), "{platform:?}");
        assert_ne!(copied.id, note.id, "{platform:?}");
        assert_eq!(fixture.store.audit_entries().len(), 1, "{platform:?}");
    }
}
