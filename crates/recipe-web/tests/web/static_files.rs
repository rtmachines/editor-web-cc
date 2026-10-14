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

/// Return the file names and the bytes of the font files on disk, sorted by name.
///
/// The unit test `static_files_each_embedded_font_matches_its_constant` pins the SHA-256 of
/// each embedded font once. This test then needs only the equality of the served body and the file.
fn fonts_on_disk() -> Vec<(String, Vec<u8>)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static/fonts");
    let mut fonts: Vec<(String, Vec<u8>)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|suffix| suffix == "woff2"))
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read(&path).unwrap())
        })
        .collect();
    fonts.sort();
    fonts
}

#[tokio::test]
async fn static_files_each_font_needs_no_session_and_has_the_body_of_its_file() {
    let router = app();
    let fonts = fonts_on_disk();
    assert_eq!(fonts.len(), 8);
    for (name, bytes) in fonts {
        let response = get(&router, &format!("/static/fonts/{name}")).await;
        assert_eq!(response.status(), 200, "{name}");
        assert_eq!(header(&response, "content-type"), "font/woff2", "{name}");
        assert_eq!(
            header(&response, "cache-control"),
            "public, max-age=86400",
            "{name}"
        );
        assert_eq!(
            header(&response, "x-content-type-options"),
            "nosniff",
            "{name}"
        );
        assert_eq!(header(&response, "set-cookie"), "", "{name}");
        assert_eq!(header(&response, "etag"), "", "{name}");
        let body = body_bytes(response).await;
        assert_eq!(body, bytes, "{name}");
    }
}

#[tokio::test]
async fn static_files_unknown_font_name_answers_not_found_without_cache() {
    let response = get(&app(), "/static/fonts/other.woff2").await;
    assert_eq!(response.status(), 404);
    assert_eq!(header(&response, "cache-control"), "no-store");
}

#[tokio::test]
async fn static_files_app_css_needs_no_session() {
    let response = get(&app(), "/static/app.css").await;
    assert_eq!(response.status(), 200);
    assert_eq!(header(&response, "content-type"), "text/css; charset=utf-8");
    assert_eq!(header(&response, "cache-control"), "public, max-age=86400");
    assert_eq!(header(&response, "x-content-type-options"), "nosniff");
    assert_eq!(header(&response, "set-cookie"), "");
    let body = body_bytes(response).await;
    assert!(String::from_utf8(body).unwrap().contains(":root"));
}

#[tokio::test]
async fn static_files_each_script_needs_no_session() {
    let router = app();
    for path in ["/static/row-version.js", "/static/tree.js"] {
        let response = get(&router, path).await;
        assert_eq!(response.status(), 200, "{path}");
        assert_eq!(
            header(&response, "content-type"),
            "application/javascript",
            "{path}"
        );
        assert_eq!(
            header(&response, "cache-control"),
            "public, max-age=86400",
            "{path}"
        );
        assert_eq!(
            header(&response, "x-content-type-options"),
            "nosniff",
            "{path}"
        );
        assert_eq!(header(&response, "set-cookie"), "", "{path}");
        let body = body_bytes(response).await;
        assert!(
            String::from_utf8(body)
                .unwrap()
                .starts_with("(function () {")
        );
    }
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
