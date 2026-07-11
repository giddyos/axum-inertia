use axum::{Router, body::Body, http::Request, routing::get};
use criterion::{Criterion, criterion_group, criterion_main};
use inertia_axum::{DynamicPage, InertiaApp, RouterInertiaExt, X_INERTIA, optional};
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tower::ServiceExt;

fn benchmark(c: &mut Criterion) {
    let calls = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route(
            "/",
            get({
                let calls = calls.clone();
                move || {
                    let calls = calls.clone();
                    async move {
                        DynamicPage::new("Bench").async_prop(
                            "optional",
                            optional(move || {
                                calls.fetch_add(1, Ordering::Relaxed);
                                async { Ok::<_, io::Error>(1) }
                            }),
                        )
                    }
                }
            }),
        )
        .inertia(InertiaApp::default_root().build().unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("unselected_async_prop", |b| {
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
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}
criterion_group!(benches, benchmark);
criterion_main!(benches);
