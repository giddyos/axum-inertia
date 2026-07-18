//! Axum integration coverage for a generated embedded frontend.

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{
        Method, Request, StatusCode,
        header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
    },
    routing::get,
};
use inertia_axum::{
    DynamicPage, InertiaApp, Redirect, RouterInertiaExt, X_INERTIA, X_INERTIA_LOCATION,
    X_INERTIA_PARTIAL_COMPONENT, X_INERTIA_PARTIAL_DATA, X_INERTIA_VERSION, page,
};
use inertia_embed::{EmbeddedFrontend, embed_frontend};
use serde_json::Value;
use tower::ServiceExt as _;

static FRONTEND: EmbeddedFrontend = embed_frontend! {
    root: "tests/fixtures/valid/dist",
    entry: "src/main.ts",
};

async fn home() -> DynamicPage {
    page!("Home", {
        message: "embedded",
        other: "retained",
    })
}

async fn redirect() -> Redirect {
    Redirect::to("/")
}

fn app() -> Router {
    Router::new()
        .route("/", get(home))
        .route("/redirect", get(redirect))
        .with_inertia(InertiaApp::embedded(&FRONTEND).build().unwrap())
}

#[tokio::test]
async fn initial_html_and_inertia_json_use_generated_tags_and_version() {
    let response = app()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains(FRONTEND.tags));

    let response = app()
        .oneshot(
            Request::builder()
                .uri("/")
                .header(X_INERTIA, "true")
                .header(X_INERTIA_VERSION, FRONTEND.version)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let page: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(page["component"], "Home");
    assert_eq!(page["version"], FRONTEND.version);
    assert_eq!(page["props"]["message"], "embedded");
}

#[tokio::test]
async fn version_mismatch_partial_reload_and_redirect_remain_protocol_correct() {
    let mismatch = app()
        .oneshot(
            Request::builder()
                .uri("/")
                .header(X_INERTIA, "true")
                .header(X_INERTIA_VERSION, "stale")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mismatch.status(), StatusCode::CONFLICT);
    assert_eq!(mismatch.headers()[X_INERTIA_LOCATION], "/");

    let partial = app()
        .oneshot(
            Request::builder()
                .uri("/")
                .header(X_INERTIA, "true")
                .header(X_INERTIA_VERSION, FRONTEND.version)
                .header(X_INERTIA_PARTIAL_COMPONENT, "Home")
                .header(X_INERTIA_PARTIAL_DATA, "message")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let page: Value =
        serde_json::from_slice(&to_bytes(partial.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(page["props"]["message"], "embedded");
    assert!(page["props"].get("other").is_none());

    let response = app()
        .oneshot(
            Request::builder()
                .uri("/redirect")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FOUND);
}

#[tokio::test]
async fn embedded_assets_support_get_head_304_missing_and_encoded_names() {
    let get = app()
        .oneshot(
            Request::builder()
                .uri("/build/assets/main-C6R2N8QK.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    assert_eq!(
        get.headers()[CONTENT_TYPE],
        "text/javascript; charset=utf-8"
    );
    let etag = get.headers()[ETAG].clone();
    let length = get.headers()[CONTENT_LENGTH].clone();
    assert!(
        !to_bytes(get.into_body(), usize::MAX)
            .await
            .unwrap()
            .is_empty()
    );

    let head = app()
        .oneshot(
            Request::builder()
                .method(Method::HEAD)
                .uri("/build/assets/main-C6R2N8QK.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(head.status(), StatusCode::OK);
    assert_eq!(head.headers()[ETAG], etag);
    assert_eq!(head.headers()[CONTENT_LENGTH], length);
    assert!(
        to_bytes(head.into_body(), usize::MAX)
            .await
            .unwrap()
            .is_empty()
    );

    let not_modified = app()
        .oneshot(
            Request::builder()
                .uri("/build/assets/main-C6R2N8QK.js")
                .header(IF_NONE_MATCH, etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);
    assert!(
        to_bytes(not_modified.into_body(), usize::MAX)
            .await
            .unwrap()
            .is_empty()
    );

    let encoded = app()
        .oneshot(
            Request::builder()
                .uri("/build/assets/file%20name.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(encoded.status(), StatusCode::OK);

    let css = app()
        .oneshot(
            Request::builder()
                .uri("/build/assets/main-30f2a8d9.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(css.status(), StatusCode::OK);
    assert_eq!(css.headers()[CONTENT_TYPE], "text/css; charset=utf-8");

    for path in [
        "/build/missing.js",
        "/build/assets/%2e%2e/main-C6R2N8QK.js",
        "/build/assets",
    ] {
        let response = app()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_ne!(response.status(), StatusCode::OK, "{path}");
    }
}
