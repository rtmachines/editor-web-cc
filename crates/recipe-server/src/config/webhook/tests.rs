//! Tests of the webhook file (ADR-0063 (a), (b), (f), (g), design D7).

use super::{WebhookFileFault, WebhookSource, parse_webhook_text, read_webhook_file};
use crate::config::test_support::Sandbox;

const KEY: &str = "alerts.webhook_url_file";
const URL: &str = "https://hooks.plant.example:8443/alert/tokenvalue0123456789?auth=querysecret";

#[test]
fn webhook_text_with_one_https_line_gives_the_url() {
    for text in [URL.to_owned(), format!("{URL}\n")] {
        let url = parse_webhook_text(&text).unwrap().unwrap();

        assert_eq!(url.uri().scheme_str(), Some("https"));
        assert_eq!(url.uri().host(), Some("hooks.plant.example"));
        assert_eq!(url.uri().port_u16(), Some(8443));
    }
}

#[test]
fn webhook_empty_text_means_no_webhook() {
    for text in ["", "\n"] {
        assert!(parse_webhook_text(text).unwrap().is_none(), "{text:?}");
    }
}

#[test]
fn webhook_refusals_are_typed() {
    let cases = [
        (format!("{URL}\r\n"), WebhookFileFault::CarriageReturn),
        (format!("{URL}\n{URL}\n"), WebhookFileFault::SeveralLines),
        (format!("\n{URL}"), WebhookFileFault::SeveralLines),
        (URL.replace("https", "http"), WebhookFileFault::NotHttps),
        (
            "ftp://hooks.plant.example/x".to_owned(),
            WebhookFileFault::NotHttps,
        ),
        (
            "hooks.plant.example/x".to_owned(),
            WebhookFileFault::NotAnAbsoluteUrl,
        ),
        ("https://".to_owned(), WebhookFileFault::NotAnAbsoluteUrl),
        (
            "https://bad host/x".to_owned(),
            WebhookFileFault::NotAnAbsoluteUrl,
        ),
        (" ".to_owned(), WebhookFileFault::NotAnAbsoluteUrl),
    ];
    for (text, fault) in cases {
        let error = parse_webhook_text(&text).unwrap_err();

        assert_eq!(error, fault, "{text:?}");
    }
}

#[test]
fn webhook_fault_and_url_text_never_hold_a_part_of_the_url() {
    let url = parse_webhook_text(URL).unwrap().unwrap();
    let mut shown = vec![format!("{url:?}")];
    for text in [
        format!("{URL}\r\n"),
        format!("{URL}\n{URL}"),
        URL.replace("https", "http"),
        "hooks.plant.example/tokenvalue0123456789".to_owned(),
    ] {
        let fault = parse_webhook_text(&text).unwrap_err();
        shown.push(format!("{fault} {fault:?}"));
    }
    for text in shown {
        for part in [
            "hooks",
            "plant",
            "tokenvalue",
            "querysecret",
            "8443",
            "alert",
        ] {
            assert!(!text.contains(part), "{part} in {text}");
        }
    }
}

#[test]
fn webhook_file_read_names_the_key_only() {
    let dir = Sandbox::new();
    let cases = [
        (dir.write("crlf", &format!("{URL}\r\n")), "carriage return"),
        (
            dir.write("two", &format!("{URL}\n{URL}\n")),
            "more than one line",
        ),
        (dir.write("http", &URL.replace("https", "http")), "https"),
        (
            dir.write("junk", "tokenvalue0123456789"),
            "not an absolute URL",
        ),
        (dir.path("absent_file"), "cannot be read"),
    ];
    for (path, words) in cases {
        let error = read_webhook_file(KEY, &path, WebhookSource::Set).unwrap_err();

        let text = format!("{error:#}");
        assert!(text.contains(KEY), "{text}");
        assert!(text.contains(words), "{text}");
        assert!(!text.contains("recipe-server-config"), "{text}");
        for part in ["hooks", "tokenvalue", "querysecret", "8443"] {
            assert!(!text.contains(part), "{part} in {text}");
        }
    }
}

#[test]
fn webhook_file_read_treats_an_empty_file_as_no_webhook() {
    let dir = Sandbox::new();
    let empty = dir.write("empty", "");
    let line = dir.write("line", "\n");

    for path in [empty, line] {
        for source in [WebhookSource::Set, WebhookSource::Default] {
            assert!(read_webhook_file(KEY, &path, source).unwrap().is_none());
        }
    }
}

#[test]
fn webhook_missing_file_stops_only_a_set_path() {
    let dir = Sandbox::new();
    let missing = dir.path("absent_file");

    let set = read_webhook_file(KEY, &missing, WebhookSource::Set);
    let default = read_webhook_file(KEY, &missing, WebhookSource::Default);

    assert!(set.is_err());
    assert!(default.unwrap().is_none());
}

#[test]
fn webhook_file_read_gives_the_url_of_a_good_file() {
    let dir = Sandbox::new();
    let path = dir.write("good", &format!("{URL}\n"));

    let url = read_webhook_file(KEY, &path, WebhookSource::Set)
        .unwrap()
        .unwrap();

    assert_eq!(url.uri().host(), Some("hooks.plant.example"));
}
