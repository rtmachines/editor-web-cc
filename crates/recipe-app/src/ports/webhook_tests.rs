use std::collections::HashSet;

use super::{AlertCondition, WebhookFault};

fn faults() -> [WebhookFault; 4] {
    [
        WebhookFault::Connection,
        WebhookFault::Tls,
        WebhookFault::Timeout,
        WebhookFault::Status { code: 503 },
    ]
}

#[test]
fn there_are_ten_distinct_alert_conditions() {
    let names: HashSet<&str> = AlertCondition::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(AlertCondition::ALL.len(), 10);
    assert_eq!(names.len(), 10);
}

#[test]
fn a_condition_name_is_a_lower_case_snake_case_word() {
    for condition in AlertCondition::ALL {
        let name = condition.name();
        assert!(!name.is_empty());
        assert!(
            name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "bad condition name: {name}"
        );
    }
}

/// Maps each variant to its literal name. A new variant fails to compile here.
fn literal_name(condition: AlertCondition) -> &'static str {
    match condition {
        AlertCondition::Panics => "panics",
        AlertCondition::NotReady => "not_ready",
        AlertCondition::BackupAge => "backup_age",
        AlertCondition::Replication => "replication",
        AlertCondition::LowDisk => "low_disk",
        AlertCondition::FailedLogins => "failed_logins",
        AlertCondition::SecurityEvents => "security_events",
        AlertCondition::AuditChainBroken => "audit_chain_broken",
        AlertCondition::AuditCheckAge => "audit_check_age",
        AlertCondition::ExportHashMismatch => "export_hash_mismatch",
    }
}

#[test]
fn each_alert_condition_has_its_literal_name_and_appears_once_in_all() {
    for condition in AlertCondition::ALL {
        assert_eq!(condition.name(), literal_name(condition), "{condition:?}");
        let count = AlertCondition::ALL
            .iter()
            .filter(|c| **c == condition)
            .count();
        assert_eq!(count, 1, "{condition:?} must appear once in ALL");
    }
}

#[test]
fn the_display_text_of_a_webhook_fault_holds_no_part_of_a_url() {
    for fault in faults() {
        let text = fault.to_string();
        assert!(!text.is_empty());
        assert!(!text.contains("://"), "{text}");
        assert!(!text.to_ascii_lowercase().contains("http"), "{text}");
    }
}

#[test]
fn the_debug_text_of_a_webhook_fault_holds_no_part_of_a_url() {
    assert_eq!(format!("{:?}", WebhookFault::Connection), "Connection");
    assert_eq!(format!("{:?}", WebhookFault::Tls), "Tls");
    assert_eq!(format!("{:?}", WebhookFault::Timeout), "Timeout");
    assert_eq!(
        format!("{:?}", WebhookFault::Status { code: 503 }),
        "Status { code: 503 }"
    );
}

#[test]
fn each_webhook_fault_has_its_display_text() {
    let cases = [
        (
            WebhookFault::Connection,
            "The connection to the webhook host failed.",
        ),
        (
            WebhookFault::Tls,
            "The TLS handshake with the webhook host failed.",
        ),
        (WebhookFault::Timeout, "The webhook call took too long."),
        (
            WebhookFault::Status { code: 503 },
            "The webhook host answered with the status 503.",
        ),
    ];
    for (fault, text) in cases {
        assert_eq!(fault.to_string(), text, "{fault:?}");
    }
}

#[test]
fn a_status_fault_names_the_status_code_in_its_text() {
    let text = WebhookFault::Status { code: 404 }.to_string();
    assert!(text.contains("404"), "{text}");
}

#[test]
fn a_webhook_fault_crosses_threads() {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<WebhookFault>();
}
