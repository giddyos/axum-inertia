//! Redirect-based form validation integration coverage.

#![cfg(feature = "macros")]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{
        Method, Request, StatusCode,
        header::{CONTENT_TYPE, LOCATION, REFERER},
    },
    routing::{get, post},
};
use inertia_axum::prelude::*;
use inertia_axum::{Errors, FormError};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;

fn validate_todo(form: &CreateTodo) -> Result<(), Errors> {
    let _ = &form.internal_token;
    if form.title.trim().is_empty() {
        Err(Errors::field("title", "The title is required"))
    } else {
        Ok(())
    }
}

#[derive(Deserialize, InertiaForm)]
#[inertia(
    validate_with = "validate_todo",
    error_bag = "createTodo",
    old_input,
    redact = "internal_token"
)]
struct CreateTodo {
    title: String,
    internal_token: Option<String>,
}

#[derive(Deserialize)]
struct LowerTodo {
    title: String,
}

async fn form_page() -> DynamicPage {
    page!("Todos/Form", { ready: true })
}

fn app(calls: Arc<AtomicUsize>) -> Router {
    let handler_calls = calls.clone();
    let lower_calls = calls;
    Router::new()
        .route("/form", get(form_page))
        .route(
            "/submit",
            post(move |Validated(input): Validated<CreateTodo>| {
                let calls = handler_calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Redirect::to("/form").flash("toast", format!("Created {}", input.title))
                }
            }),
        )
        .route(
            "/lower",
            post(move |form: InertiaForm<LowerTodo>| {
                let calls = lower_calls.clone();
                async move {
                    let input = form.validate_with(|input| {
                        if input.title.is_empty() {
                            Err(Errors::field("title", "required"))
                        } else {
                            Ok(())
                        }
                    })?;
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, FormError>(Redirect::to("/form").flash("title", input.title))
                }
            }),
        )
        .inertia(
            InertiaApp::default_root()
                .transient(MemoryTransient::new())
                .build()
                .unwrap(),
        )
}

fn request(uri: &str, content_type: &str, body: impl Into<Body>) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header("x-inertia", "true")
        .header(CONTENT_TYPE, content_type)
        .header(REFERER, "/form")
        .body(body.into())
        .unwrap()
}
async fn json_page(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[tokio::test]
async fn invalid_json_redirects_before_handler_and_request_bag_wins() {
    let calls = Arc::new(AtomicUsize::new(0));
    let app = app(calls.clone());
    let mut invalid = request(
        "/submit",
        "application/json",
        r#"{"title":"","internal_token":"secret"}"#,
    );
    invalid
        .headers_mut()
        .insert("x-inertia-error-bag", "requestBag".parse().unwrap());
    let response = app.clone().oneshot(invalid).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()[LOCATION], "/form");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let page = json_page(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri("/form")
                    .header("x-inertia", "true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        page["props"]["errors"]["requestBag"]["title"],
        "The title is required"
    );
    assert_eq!(page["props"]["oldInput"], json!({"title":""}));
    assert!(!page.to_string().contains("secret"));
    let consumed = json_page(
        app.oneshot(
            Request::builder()
                .uri("/form")
                .header("x-inertia", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(consumed["props"]["errors"], json!({}));
    assert!(consumed["props"].get("oldInput").is_none());
}

#[tokio::test]
async fn derive_bag_is_fallback() {
    let app = app(Arc::new(AtomicUsize::new(0)));
    app.clone()
        .oneshot(request(
            "/submit",
            "application/json",
            r#"{"title":"","internal_token":null}"#,
        ))
        .await
        .unwrap();
    let page = json_page(
        app.oneshot(
            Request::builder()
                .uri("/form")
                .header("x-inertia", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(
        page["props"]["errors"]["createTodo"]["title"],
        "The title is required"
    );
}

#[tokio::test]
async fn valid_json_and_urlencoded_forms_reach_handlers() {
    let calls = Arc::new(AtomicUsize::new(0));
    let app = app(calls.clone());
    assert_eq!(
        app.clone()
            .oneshot(request(
                "/submit",
                "application/json",
                r#"{"title":"Ship","internal_token":"secret"}"#
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::SEE_OTHER
    );
    let redirected = json_page(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri("/form")
                    .header("x-inertia", "true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(redirected["flash"]["toast"], "Created Ship");
    assert_eq!(
        app.oneshot(request(
            "/lower",
            "application/x-www-form-urlencoded",
            "title=Lower"
        ))
        .await
        .unwrap()
        .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn lower_level_validation_redirects_without_old_input_by_default() {
    let calls = Arc::new(AtomicUsize::new(0));
    let app = app(calls.clone());
    let response = app
        .clone()
        .oneshot(request(
            "/lower",
            "application/x-www-form-urlencoded",
            "title=",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let page = json_page(
        app.oneshot(
            Request::builder()
                .uri("/form")
                .header("x-inertia", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(page["props"]["errors"]["title"], "required");
    assert!(page["props"].get("oldInput").is_none());
}

#[tokio::test]
async fn multipart_is_not_folded_into_the_common_extractor() {
    let response = app(Arc::new(AtomicUsize::new(0)))
        .oneshot(request(
            "/submit",
            "multipart/form-data; boundary=x",
            "--x--",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn validation_without_transient_store_is_actionable() {
    let app = Router::new()
        .route(
            "/",
            post(|_value: Validated<CreateTodo>| async { "unreachable" }),
        )
        .inertia(InertiaApp::default_root().build().unwrap());
    let response = app
        .oneshot(request(
            "/",
            "application/json",
            r#"{"title":"","internal_token":null}"#,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(body.contains("transient"));
}

#[cfg(feature = "garde")]
#[derive(garde::Validate, InertiaForm)]
#[inertia(validator = "garde")]
struct GardeForm {
    #[garde(length(min = 1))]
    title: String,
}

#[cfg(feature = "validator")]
#[derive(validator::Validate, InertiaForm)]
#[inertia(validator = "validator")]
struct ValidatorForm {
    #[validate(length(min = 1))]
    title: String,
}

#[test]
fn feature_validation_adapters_produce_standard_errors() {
    #[cfg(feature = "garde")]
    assert!(
        inertia_axum::Validate::validate(&GardeForm {
            title: String::new()
        })
        .is_err()
    );
    #[cfg(feature = "validator")]
    assert!(
        inertia_axum::Validate::validate(&ValidatorForm {
            title: String::new()
        })
        .is_err()
    );
}
