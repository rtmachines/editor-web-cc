//! Requirement "Static files" (`api.md` section 2.1).

use sha2::{Digest, Sha256};

use crate::support::{app, body_bytes, get, header, router_with};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[tokio::test]
async fn static_files_htmx_needs_no_session() {
    let response = get(&app(), "/static/htmx.min.js").await;
    assert_eq!(response.status(), 200);
    assert_eq!(header(&response, "content-type"), "application/javascript");
    assert_eq!(header(&response, "cache-control"), "public, max-age=86400");
    assert_eq!(header(&response, "set-cookie"), "");
}

#[tokio::test]
async fn static_files_htmx_body_is_the_pinned_file() {
    let response = get(&app(), "/static/htmx.min.js").await;
    let body = body_bytes(response).await;
    assert_eq!(
        hex(Sha256::digest(&body).as_slice()),
        recipe_web::HTMX_SHA256
    );
}

#[tokio::test]
async fn static_files_cache_max_age_follows_the_setting() {
    let settings = recipe_web::WebSettings {
        static_cache_max_age: std::time::Duration::from_secs(3600),
        ..recipe_web::WebSettings::default()
    };
    let response = get(&router_with(settings), "/static/htmx.min.js").await;
    assert_eq!(header(&response, "cache-control"), "public, max-age=3600");
}
