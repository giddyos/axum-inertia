//! Convention-based Vite integration coverage.

#![cfg(feature = "vite")]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{
        HeaderMap, HeaderValue, Method, Request, StatusCode,
        header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
    },
    routing::get,
};
use inertia_axum::{
    AssetBody, AssetContext, AssetError, AssetProvider, AssetRequest, AssetResponse, AssetSource,
    AssetTags, AssetVersion, DynamicPage, InertiaApp, Page, RouterInertiaExt, X_INERTIA,
    X_INERTIA_VERSION, page,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn fixture() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "inertia-axum-vite-{}-{}",
        std::process::id(),
        NEXT_DIR.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(path.join("dist/.vite")).unwrap();
    path
}

fn manifest(root: &Path, source: &str) {
    fs::write(root.join("dist/.vite/manifest.json"), source).unwrap();
}

async fn home() -> DynamicPage {
    page!("Home", { message: "hello" })
}

#[derive(Clone)]
struct NumericAssets {
    version: AssetVersion,
}

impl AssetProvider for NumericAssets {
    fn version(&self) -> AssetVersion {
        self.version.clone()
    }
    fn render_tags(&self, _context: AssetContext<'_>) -> Result<AssetTags, AssetError> {
        Ok(AssetTags::new(
            "<script src=\"/custom.js\"></script>".to_owned(),
        ))
    }
}

#[derive(Debug)]
struct RecordingSource {
    paths: Arc<Mutex<Vec<String>>>,
}

impl AssetSource for RecordingSource {
    fn get(&self, request: AssetRequest<'_>) -> Option<AssetResponse> {
        self.paths.lock().unwrap().push(request.path.to_owned());
        (request.path == "nested/app.js").then(|| {
            let mut headers = HeaderMap::new();
            headers.insert("x-asset-source", HeaderValue::from_static("core"));
            AssetResponse {
                status: StatusCode::OK,
                headers,
                body: AssetBody::Static(b"adapter asset"),
            }
        })
    }
}

#[derive(Clone)]
struct SourceAssets {
    source: Arc<RecordingSource>,
}

impl AssetProvider for SourceAssets {
    fn version(&self) -> AssetVersion {
        AssetVersion::from("source-v1")
    }

    fn render_tags(&self, _context: AssetContext<'_>) -> Result<AssetTags, AssetError> {
        Ok(AssetTags::new(
            "<script src=\"/assets/nested/app.js\"></script>".to_owned(),
        ))
    }

    fn source(&self) -> Option<Arc<dyn AssetSource>> {
        Some(self.source.clone())
    }
}

#[tokio::test]
async fn custom_provider_keeps_numeric_page_version() {
    let app = Router::new().route("/", get(home)).inertia(
        InertiaApp::default_root()
            .assets(NumericAssets {
                version: 42_u64.into(),
            })
            .build()
            .unwrap(),
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header(X_INERTIA, "true")
                .header(X_INERTIA_VERSION, "42")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let page: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(page["version"], 42);
}

#[tokio::test]
async fn production_manifest_resolves_imports_css_version_and_static_files() {
    let root = fixture();
    manifest(
        &root,
        r#"{
      "src/main.ts":{"file":"assets/main-123.js","css":["assets/main.css"],"imports":["_shared.js"]},
      "_shared.js":{"file":"assets/shared-456.js","css":["assets/shared.css"]}
    }"#,
    );
    fs::create_dir_all(root.join("dist/assets")).unwrap();
    fs::write(root.join("dist/assets/main-123.js"), "export default 1").unwrap();
    let inertia = InertiaApp::vite(&root).build().unwrap();
    let app = Router::new().route("/", get(home)).inertia(inertia);
    fs::remove_file(root.join("dist/.vite/manifest.json")).unwrap();
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let html = String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("/build/assets/main.css"));
    assert!(html.contains("/build/assets/shared.css"));
    assert!(html.contains("modulepreload"));
    assert!(html.contains("/build/assets/main-123.js"));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/build/assets/main-123.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[CONTENT_TYPE], "text/javascript");
    assert_eq!(response.headers()[CONTENT_LENGTH], "16");
    let etag = response.headers()[ETAG].clone();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(body, "export default 1");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::HEAD)
                .uri("/build/assets/main-123.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[ETAG], etag);
    assert!(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .is_empty()
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/build/assets/main-123.js")
                .header(IF_NONE_MATCH, &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert!(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .is_empty()
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/build/assets/main-123.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/build/assets/missing.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn custom_source_is_mounted_with_exact_public_path_stripping() {
    let paths = Arc::new(Mutex::new(Vec::new()));
    let source = Arc::new(RecordingSource {
        paths: paths.clone(),
    });
    let app = Router::new().inertia(
        InertiaApp::default_root()
            .assets(SourceAssets { source })
            .public_path("/assets/")
            .build()
            .unwrap(),
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/assets/nested/app.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["x-asset-source"], "core");
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        "adapter asset"
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/assets2/nested/app.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(paths.lock().unwrap().as_slice(), ["nested/app.js"]);
}

#[tokio::test]
async fn no_asset_route_is_registered_without_a_source() {
    let app = Router::new()
        .route("/build/{*path}", get(|| async { "application fallback" }))
        .inertia(
            InertiaApp::default_root()
                .assets(NumericAssets {
                    version: 1_u64.into(),
                })
                .build()
                .unwrap(),
        );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/build/app.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        "application fallback"
    );
}

#[tokio::test]
async fn development_mode_needs_no_manifest_and_renders_both_scripts() {
    let root = fixture();
    fs::remove_file(root.join("dist/.vite/manifest.json")).ok();
    let app = Router::new().route("/", get(home)).inertia(
        InertiaApp::vite(&root)
            .entry("src/app.ts")
            .dev_server("http://localhost:5173/")
            .build()
            .unwrap(),
    );
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let html = String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(html.contains("http://localhost:5173/@vite/client"));
    assert!(html.contains("http://localhost:5173/src/app.ts"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn startup_errors_are_actionable() {
    let root = fixture();
    let missing = InertiaApp::vite(&root).build().err().unwrap().to_string();
    assert!(missing.contains("Could not read manifest"));
    manifest(&root, r#"{"src/app.ts":{"file":"app.js"}}"#);
    let entry = InertiaApp::vite(&root).build().err().unwrap().to_string();
    assert!(entry.contains("Entry \"src/main.ts\" was not found"));
    assert!(entry.contains("src/app.ts"));
    let malformed = InertiaApp::vite(&root)
        .dev_server("not a URL")
        .build()
        .err()
        .unwrap()
        .to_string();
    assert!(malformed.contains("VITE_DEV_SERVER_URL"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn asset_version_retains_scalar_json_and_normalizes_headers() {
    let string = AssetVersion::from("release-7");
    let number = AssetVersion::from(42_u64);
    assert_eq!(serde_json::to_value(&string).unwrap(), json!("release-7"));
    assert_eq!(serde_json::to_value(&number).unwrap(), json!(42));
    assert_eq!(number.header_value(), "42");
    let page = Page::new("Home", Value::Object(Default::default()), "/").version(number);
    assert_eq!(serde_json::to_value(page).unwrap()["version"], 42);
}

#[tokio::test]
async fn configured_overrides_change_manifest_and_public_paths() {
    let root = fixture();
    fs::create_dir_all(root.join("public/build/.vite")).unwrap();
    fs::write(
        root.join("public/build/.vite/manifest.json"),
        r#"{"src/app.ts":{"file":"app.js"}}"#,
    )
    .unwrap();
    fs::write(root.join("public/build/app.js"), "ok").unwrap();
    let app = Router::new().route("/", get(home)).inertia(
        InertiaApp::vite(&root)
            .entry("src/app.ts")
            .build_dir("public/build")
            .public_path("/assets")
            .build()
            .unwrap(),
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/assets/app.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    fs::remove_dir_all(root).unwrap();
}
