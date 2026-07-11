#![allow(missing_docs)]

#[path = "../src/main.rs"]
#[allow(dead_code)]
mod example;

use axum::{
    body::{to_bytes, Body},
    http::Request,
    routing::get,
    Router,
};
use inertia_axum::{DynamicPage, RouterInertiaExt as _};
use tower::ServiceExt as _;

async fn require_node_22() {
    let output = tokio::process::Command::new("node")
        .arg("--version")
        .output()
        .await
        .expect("production SSR tests require Node 22 or newer on PATH");
    assert!(output.status.success(), "`node --version` failed");
    let version = String::from_utf8_lossy(&output.stdout);
    let major = version
        .trim()
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .expect("Node returned an invalid version");
    assert!(
        major >= 22,
        "production SSR tests require Node 22 or newer; found {}",
        version.trim()
    );
}

#[tokio::test]
#[ignore = "requires Node 22 or newer and built example frontend artifacts"]
async fn production_example_consumes_manifest_and_official_ssr_bundle() {
    require_node_22().await;
    let inertia = example::production_inertia().await.unwrap();
    let app = Router::new()
        .route(
            "/todos",
            get(|| async { DynamicPage::new("Todos/Index").prop("todos", Vec::<String>::new()) }),
        )
        .inertia(inertia);
    let response = app
        .oneshot(Request::get("/todos").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let html = String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("data-server-rendered=\"true\""));
    assert!(html.contains("<h1>Todos</h1>"));
    assert!(html.contains("/public/build/"));
}
