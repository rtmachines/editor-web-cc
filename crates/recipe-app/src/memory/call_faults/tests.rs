use super::CallFaults;

#[test]
fn a_new_plan_has_no_fault_and_no_call() {
    let mut faults: CallFaults<&str, u8> = CallFaults::default();
    assert_eq!(faults.calls("read"), 0);
    assert_eq!(faults.next("read"), None);
    assert_eq!(faults.calls("read"), 1);
}

#[test]
fn fail_on_call_fires_on_that_call_only() {
    let mut faults = CallFaults::default();
    faults.fail_on_call("read", 2, 7_u8);
    assert_eq!(faults.next("read"), None);
    assert_eq!(faults.next("read"), Some(7));
    assert_eq!(faults.next("read"), None);
}

#[test]
fn fail_from_call_fires_on_that_call_and_every_later_call() {
    let mut faults = CallFaults::default();
    faults.fail_from_call("read", 2, 7_u8);
    assert_eq!(faults.next("read"), None);
    assert_eq!(faults.next("read"), Some(7));
    assert_eq!(faults.next("read"), Some(7));
}

#[test]
fn each_method_counts_its_own_calls() {
    let mut faults = CallFaults::default();
    faults.fail_on_call("write", 1, 9_u8);
    assert_eq!(faults.next("read"), None);
    assert_eq!(faults.next("read"), None);
    assert_eq!(faults.next("write"), Some(9));
    assert_eq!(faults.calls("read"), 2);
    assert_eq!(faults.calls("write"), 1);
}

#[test]
fn a_call_number_of_zero_never_fires() {
    let mut faults = CallFaults::default();
    faults.fail_on_call("read", 0, 1_u8);
    assert_eq!(faults.next("read"), None);
}

#[test]
fn the_first_plan_that_matches_wins() {
    let mut faults = CallFaults::default();
    faults.fail_on_call("read", 1, 1_u8);
    faults.fail_from_call("read", 1, 2_u8);
    assert_eq!(faults.next("read"), Some(1));
    assert_eq!(faults.next("read"), Some(2));
}

#[test]
fn clear_drops_the_plans_and_keeps_the_counters() {
    let mut faults = CallFaults::default();
    faults.fail_from_call("read", 1, 1_u8);
    assert_eq!(faults.next("read"), Some(1));
    faults.clear();
    assert_eq!(faults.next("read"), None);
    assert_eq!(faults.calls("read"), 2);
}
