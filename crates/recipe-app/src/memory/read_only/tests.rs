//! Tests of the in-memory `ReadOnlySwitch` (task 3.10, design D6, test row T14).

use super::MemoryReadOnly;
use crate::ports::ReadOnlySwitch;

#[test]
fn a_new_switch_is_off() {
    assert!(!MemoryReadOnly::default().is_on());
}

#[test]
fn a_switch_can_start_on() {
    assert!(MemoryReadOnly::new(true).is_on());
}

#[test]
fn set_turns_the_mode_on_and_off() {
    let switch = MemoryReadOnly::default();
    switch.set(true);
    assert!(switch.is_on());
    switch.set(false);
    assert!(!switch.is_on());
}

#[test]
fn the_switch_records_every_call_of_set_even_without_a_change() {
    let switch = MemoryReadOnly::default();
    switch.set(false);
    switch.set(true);
    switch.set(true);
    assert_eq!(switch.history(), vec![false, true, true]);
}

#[test]
fn a_clone_shares_the_flag_and_the_history() {
    let switch = MemoryReadOnly::default();
    let copy = switch.clone();
    copy.set(true);
    assert!(switch.is_on());
    assert_eq!(switch.history(), vec![true]);
}

#[test]
fn the_switch_works_as_a_trait_object() {
    let switch: Box<dyn ReadOnlySwitch> = Box::new(MemoryReadOnly::default());
    switch.set(true);
    assert!(switch.is_on());
}
