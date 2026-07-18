#![allow(missing_docs)]

use axum::{
    Router,
    body::Body,
    http::{Method, Request},
    routing::get,
};
use criterion::{Criterion, criterion_group, criterion_main};
use inertia_axum::{InertiaApp, RouterInertiaExt, page};
use std::fs;
use tower::ServiceExt;

fn benchmark(c: &mut Criterion) {
    let root = std::env::temp_dir().join(format!("inertia-axum-vite-bench-{}", std::process::id()));
    fs::create_dir_all(root.join("dist/.vite")).unwrap();
    fs::write(
        root.join("dist/.vite/manifest.json"),
        r#"{"src/main.ts":{"file":"assets/main.js","css":["assets/main.css"]}}"#,
    )
    .unwrap();
    fs::create_dir_all(root.join("dist/assets")).unwrap();
    fs::write(root.join("dist/assets/main.js"), "export default 1").unwrap();
    fs::write(root.join("dist/assets/main.css"), "body{}").unwrap();
    let app = Router::new()
        .route("/", get(|| async { page!("Home", { message: "hello" }) }))
        .inertia(InertiaApp::vite(&root).build().unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("vite_initial_html", |b| {
        b.iter(|| {
            runtime.block_on(
                app.clone()
                    .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap()),
            )
        });
    });
    c.bench_function("vite_asset_get", |b| {
        b.iter(|| {
            runtime.block_on(
                app.clone().oneshot(
                    Request::builder()
                        .uri("/build/assets/main.js")
                        .body(Body::empty())
                        .unwrap(),
                ),
            )
        });
    });
    c.bench_function("vite_asset_head", |b| {
        b.iter(|| {
            runtime.block_on(
                app.clone().oneshot(
                    Request::builder()
                        .method(Method::HEAD)
                        .uri("/build/assets/main.js")
                        .body(Body::empty())
                        .unwrap(),
                ),
            )
        });
    });
    fs::remove_dir_all(root).unwrap();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
