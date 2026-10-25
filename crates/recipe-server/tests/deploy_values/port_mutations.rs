//! Proof that a port or a bare integer of the application environment keeps the syntax that the application reads.
//!
//! The application reads the two port keys as raw text and accepts ASCII digits only. A text such
//! as `9000.0` equals 9000 as a number, but the application refuses it at startup.
//! The session timeouts and the database port are bare integers. A text such as `30min` equals
//! 1800 seconds for a unit-aware comparison, but the application refuses it at startup.

use crate::inputs::{Inputs, check_all};

/// The environment variable and the port digits that `compose.yaml` holds for it.
const PORTS: [(&str, &str); 2] = [
    ("RECIPE_HTTP__PUBLIC_PORT", "8080"),
    ("RECIPE_HTTP__INTERNAL_PORT", "9000"),
];

/// Texts that equal the port as a number but break the rule of `parse_port`.
fn bad_texts(digits: &str) -> [String; 4] {
    [
        format!("{digits}.0"),
        format!("+{digits}"),
        format!(" {digits}"),
        format!("0{digits}.00"),
    ]
}

/// A numeric key that the application reads as a bare integer: the variable, its digits, and
/// texts that a unit-aware comparison accepts but the integer reader refuses.
const INTEGERS: [(&str, &str, &[&str]); 3] = [
    ("RECIPE_SESSION__IDLE_TIMEOUT", "1800", &["1800s", "30min"]),
    ("RECIPE_SESSION__ABSOLUTE_TIMEOUT", "43200", &["12h"]),
    ("RECIPE_DB__PORT", "5432", &["5432s"]),
];

/// Assert that the text `bad` in place of `digits` gives a syntax fault that names `name`.
fn assert_syntax_fault(base: &Inputs, name: &str, digits: &str, bad: &str) {
    let from = format!("{name}: \"{digits}\"");
    let to = format!("{name}: \"{bad}\"");
    let report = check_all(&base.with_replaced("compose.yaml", &from, &to));
    let named = report
        .faults
        .iter()
        .any(|fault| fault.key.contains(name) && fault.expected.contains("ASCII digits"));
    assert!(
        named,
        "no syntax fault for {name} after {from:?} -> {to:?}\nfaults:\n{}",
        report.message()
    );
}

#[test]
fn a_port_text_that_the_application_refuses_is_a_fault_that_names_the_key() {
    let base = Inputs::load();
    for (name, digits) in PORTS {
        for bad in bad_texts(digits) {
            assert_syntax_fault(&base, name, digits, &bad);
        }
    }
}

#[test]
fn an_integer_text_with_a_unit_is_a_fault_that_names_the_key() {
    let base = Inputs::load();
    for (name, digits, texts) in INTEGERS {
        for bad in texts {
            assert_syntax_fault(&base, name, digits, bad);
        }
    }
}
