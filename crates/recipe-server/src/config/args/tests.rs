//! Tests of the subcommand `healthcheck` in the parser of the command line (ADR-0059 (a), (f)).

use super::{Command, parse};
use crate::probe::ProbePath;

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn healthcheck_without_options_probes_the_readiness_path() {
    let command = parse(&words(&["healthcheck"])).unwrap();

    assert_eq!(
        command,
        Command::Healthcheck {
            path: ProbePath::readiness()
        }
    );
}

#[test]
fn healthcheck_takes_the_path_option() {
    let command = parse(&words(&["healthcheck", "--path", "/healthz"])).unwrap();

    assert_eq!(
        command,
        Command::Healthcheck {
            path: ProbePath::liveness()
        }
    );
}

#[test]
fn healthcheck_refuses_a_path_option_without_a_value() {
    let error = parse(&words(&["healthcheck", "--path"])).unwrap_err();

    assert!(
        error.to_string().contains("--path needs a value"),
        "{error}"
    );
}

#[test]
fn healthcheck_refuses_a_repeated_path_option() {
    let error = parse(&words(&["healthcheck", "--path", "/a", "--path", "/b"])).unwrap_err();

    assert!(error.to_string().contains("twice"), "{error}");
}

#[test]
fn healthcheck_refuses_a_path_that_is_not_safe() {
    for bad in ["readyz", "/with space", "/a\r\nb", ""] {
        let error = parse(&words(&["healthcheck", "--path", bad])).unwrap_err();

        assert!(error.to_string().contains("--path is not valid"), "{error}");
    }
}

#[test]
fn healthcheck_refuses_an_unknown_option_and_does_not_echo_it() {
    let error = parse(&words(&["healthcheck", "--port", "9000"])).unwrap_err();

    let text = error.to_string();
    assert!(text.contains("argument 2 is not known"), "{text}");
    assert!(!text.contains("--port"), "{text}");
}

#[test]
fn the_usage_text_names_healthcheck() {
    let error = parse(&[]).unwrap_err();

    assert!(error.to_string().contains("healthcheck"), "{error}");
}
