#![cfg(feature = "ssr")]
#![allow(missing_docs)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    routing::{get, post},
};
use inertia_axum::{DynamicPage, InertiaApp, RouterInertiaExt as _, Ssr};
use std::sync::atomic::{AtomicUsize, Ordering};
use tower::ServiceExt as _;

fn unique_fixture() -> std::path::PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    std::env::temp_dir().join(format!(
        "inertia-ssr-config-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn synchronous_build_rejects_configured_ssr() {
    let Err(error) = InertiaApp::default_root().ssr("dist/ssr/ssr.js").build() else {
        panic!("SSR must require asynchronous startup");
    };

    assert!(error.to_string().contains(".start()"));
    assert!(error.to_string().contains(".await"));
}

#[tokio::test]
async fn production_vite_ssr_requires_client_manifest_before_node_startup() {
    let root = unique_fixture();
    let bundle = root.join("dist/ssr/app.js");
    std::fs::create_dir_all(bundle.parent().unwrap()).unwrap();
    std::fs::write(&bundle, "process.exit(0)").unwrap();

    let result = InertiaApp::vite(&root)
        .entry("src/app.js")
        .build_dir("public/build")
        .ssr("dist/ssr/app.js")
        .start()
        .await;
    let Err(error) = result else {
        panic!("production Vite startup must reject a missing client manifest");
    };
    assert!(matches!(error, inertia_axum::StartError::Config(_)));
    assert!(error.to_string().contains(".vite/manifest.json"));
    assert!(error.to_string().contains("Run the Vite production build"));
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn ssr_with_default_root_does_not_require_a_vite_manifest() {
    let service = Router::new()
        .route("/health", get(|| async { StatusCode::OK }))
        .route(
            "/render",
            post(|| async {
                r#"{"head":[],"body":"<div id=\"app\" data-server-rendered=\"true\">external</div>"}"#
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
    let inertia = InertiaApp::default_root()
        .ssr(Ssr::external(endpoint))
        .start()
        .await
        .unwrap();
    let app = Router::new()
        .route("/", get(|| async { DynamicPage::new("Home") }))
        .inertia(inertia);
    let response = app
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let html = String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("data-server-rendered=\"true\">external"));
}
