//! Axum router installation and framework-neutral asset mounting.

use crate::{InertiaLayer, assets::asset_response};
use axum::{Router, body::Body, extract::Request, response::Response, routing::any};
use inertia_core::AssetRequest;
use std::sync::Arc;

/// Installs an Inertia application on an Axum router.
pub trait RouterInertiaExt<S> {
    /// Adds request preparation and response finalization.
    fn inertia(self, app: inertia_core::InertiaApp) -> Self;

    /// Alias matching the installation vocabulary used by other adapters.
    fn with_inertia(self, app: inertia_core::InertiaApp) -> Self;
}

impl<S> RouterInertiaExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn inertia(self, app: inertia_core::InertiaApp) -> Self {
        let public_path: std::sync::Arc<str> = Arc::from(app.asset_public_path());
        let router = if let Some(source) = app.asset_source().cloned() {
            let wildcard = if public_path.as_ref() == "/" {
                "/{*path}".to_owned()
            } else {
                format!("{public_path}/{{*path}}")
            };
            let handler_public_path = public_path.clone();
            self.route(
                &wildcard,
                any(move |request: Request| {
                    let source = source.clone();
                    let public_path = handler_public_path.clone();
                    async move {
                        let request_path = request.uri().path();
                        let relative = if public_path.as_ref() == "/" {
                            request_path.strip_prefix('/')
                        } else {
                            request_path
                                .strip_prefix(public_path.as_ref())
                                .and_then(|path| path.strip_prefix('/'))
                        };
                        let Some(path) = relative else {
                            return not_found();
                        };
                        let asset_request = AssetRequest {
                            method: request.method(),
                            path,
                            headers: request.headers(),
                        };
                        source
                            .get(asset_request)
                            .map_or_else(not_found, asset_response)
                    }
                }),
            )
        } else {
            self
        };
        router.layer(InertiaLayer::new(app))
    }

    fn with_inertia(self, app: inertia_core::InertiaApp) -> Self {
        self.inertia(app)
    }
}

fn not_found() -> Response {
    Response::builder()
        .status(http::StatusCode::NOT_FOUND)
        .body(Body::empty())
        .expect("static response parts are valid")
}
