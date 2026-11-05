use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;

use super::STATE_SETTING_KEYS;
use crate::ports::{ChainSeq, StateValue, TimeStateKey};

#[test]
fn the_state_keys_equal_the_keys_of_the_state_types() {
    let instant = Timestamp::from(OffsetDateTime::UNIX_EPOCH);
    let mut expected = vec![
        StateValue::AuditLastCheckSeq(ChainSeq::from(1)).key(),
        StateValue::AuditLastCheckAt(instant).key(),
        StateValue::HeartbeatLastAt(instant).key(),
        TimeStateKey::AuditLastCheckAt.key(),
        TimeStateKey::HeartbeatLastAt.key(),
    ];
    expected.sort_unstable();
    expected.dedup();
    let mut listed = STATE_SETTING_KEYS.to_vec();
    listed.sort_unstable();
    assert_eq!(listed, expected);
}
