#![allow(missing_docs)]

use axum::{Router, body::Body, http::Request, routing::get};
use criterion::{Criterion, criterion_group, criterion_main};
use inertia_axum::{DynamicPage, InertiaApp, RouterInertiaExt, X_INERTIA, lazy};
use std::io;
use tower::ServiceExt;

fn benchmark(c: &mut Criterion) {
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                (0..16).fold(DynamicPage::new("Bench"), |page, index| {
                    page.async_prop(
                        format!("prop{index}"),
                        lazy(move || async move {
                            tokio::task::yield_now().await;
                            Ok::<_, io::Error>(index)
                        }),
                    )
                })
            }),
        )
        .inertia(InertiaApp::default_root().build().unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("selected_async_prop_concurrency", |b| {
        b.iter(|| {
            runtime.block_on(
                app.clone().oneshot(
                    Request::builder()
                        .uri("/")
                        .header(X_INERTIA, "true")
                        .body(Body::empty())
                        .unwrap(),
                ),
            )
        })
    });
}
criterion_group!(benches, benchmark);
criterion_main!(benches);
