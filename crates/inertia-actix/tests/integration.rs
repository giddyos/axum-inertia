#![allow(missing_docs)]

use actix_web::{App, body::to_bytes, http::StatusCode, test as actix_test, web};
use inertia_actix::{
    DynamicPage, Inertia, InertiaApp, InertiaMiddleware, Result as InertiaResult, configure,
};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

async fn awaited(inertia: Inertia) -> InertiaResult {
    inertia
        .render(
            "Awaited",
            json!({
                "message": "resolved asynchronously",
            }),
        )
        .await
}

#[actix_web::test]
async fn configure_installs_app_data_for_awaited_rendering() {
    let inertia = InertiaApp::default_root().build().unwrap();
    let app = actix_test::init_service(
        App::new()
            .route("/", web::get().to(awaited))
            .wrap(InertiaMiddleware::new(inertia.clone()))
            .configure(configure(inertia)),
    )
    .await;
    let response = actix_test::call_service(
        &app,
        actix_test::TestRequest::get()
            .uri("/")
            .insert_header(("x-inertia", "true"))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body()).await.unwrap()).unwrap();
    assert_eq!(page["component"], "Awaited");
    assert_eq!(page["props"]["message"], "resolved asynchronously");
}

#[actix_web::test]
async fn missing_installation_returns_an_actionable_extractor_error() {
    let app = actix_test::init_service(App::new().route("/", web::get().to(awaited))).await;
    let response =
        actix_test::call_service(&app, actix_test::TestRequest::get().uri("/").to_request()).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body()).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("app data is not installed"));
}

#[actix_web::test]
async fn missing_middleware_returns_an_actionable_extractor_error() {
    let inertia = InertiaApp::default_root().build().unwrap();
    let app = actix_test::init_service(
        App::new()
            .route("/", web::get().to(awaited))
            .app_data(web::Data::new(inertia)),
    )
    .await;
    let response =
        actix_test::call_service(&app, actix_test::TestRequest::get().uri("/").to_request()).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body()).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("register InertiaMiddleware"));
}

#[actix_web::test]
async fn version_mismatch_short_circuits_the_handler() {
    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = Arc::clone(&calls);
    let inertia = InertiaApp::default_root()
        .version("current")
        .build()
        .unwrap();
    let app = actix_test::init_service(
        App::new()
            .route(
                "/page",
                web::get().to(move || {
                    let calls = Arc::clone(&handler_calls);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        DynamicPage::new("Page")
                    }
                }),
            )
            .app_data(web::Data::new(inertia.clone()))
            .wrap(InertiaMiddleware::new(inertia)),
    )
    .await;
    let response = actix_test::call_service(
        &app,
        actix_test::TestRequest::get()
            .uri("/page")
            .insert_header(("x-inertia", "true"))
            .insert_header(("x-inertia-version", "stale"))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response
            .headers()
            .get("x-inertia-location")
            .and_then(|value| value.to_str().ok()),
        Some("/page")
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[actix_web::test]
async fn malformed_protocol_header_short_circuits_the_handler() {
    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = Arc::clone(&calls);
    let inertia = InertiaApp::default_root().build().unwrap();
    let app = actix_test::init_service(
        App::new()
            .route(
                "/page",
                web::get().to(move || {
                    let calls = Arc::clone(&handler_calls);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        DynamicPage::new("Page")
                    }
                }),
            )
            .app_data(web::Data::new(inertia.clone()))
            .wrap(InertiaMiddleware::new(inertia)),
    )
    .await;
    let response = actix_test::call_service(
        &app,
        actix_test::TestRequest::get()
            .uri("/page")
            .append_header((
                actix_web::http::header::HeaderName::from_static("x-inertia"),
                actix_web::http::header::HeaderValue::from_bytes(&[0x80]).unwrap(),
            ))
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body()).await.unwrap();
    assert!(String::from_utf8_lossy(&body).contains("non-UTF-8 Inertia protocol header"));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[actix_web::test]
async fn ordinary_responses_remain_untouched() {
    let inertia = InertiaApp::default_root().build().unwrap();
    let app = actix_test::init_service(
        App::new()
            .route("/health", web::get().to(|| async { "healthy" }))
            .app_data(web::Data::new(inertia.clone()))
            .wrap(InertiaMiddleware::new(inertia)),
    )
    .await;
    let response = actix_test::call_service(
        &app,
        actix_test::TestRequest::get().uri("/health").to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body()).await.unwrap(),
        web::Bytes::from_static(b"healthy")
    );
}
