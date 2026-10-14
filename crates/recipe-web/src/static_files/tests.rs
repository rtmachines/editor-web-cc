use sha2::{Digest, Sha256};

use super::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn static_files_embedded_htmx_matches_the_pinned_hash() {
    let digest = Sha256::digest(HTMX_BYTES);
    assert_eq!(hex(digest.as_slice()), HTMX_SHA256);
}

#[test]
fn static_files_pin_names_an_htmx_2_release() {
    assert!(HTMX_VERSION.starts_with("2."), "{HTMX_VERSION}");
    assert_eq!(HTMX_SHA256.len(), 64);
    assert!(
        HTMX_SHA256
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}

#[test]
fn static_files_embedded_file_is_the_upstream_script() {
    assert!(HTMX_BYTES.starts_with(b"var htmx="));
}
