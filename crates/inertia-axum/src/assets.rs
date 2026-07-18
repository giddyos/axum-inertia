//! Axum conversion for framework-neutral static asset responses.

use axum::{body::Body, response::Response};
use inertia_core::{AssetBody, AssetResponse};

/// Converts a framework-neutral asset response into Axum's response type.
pub fn asset_response(asset: AssetResponse) -> Response {
    let body = match asset.body {
        AssetBody::Empty => Body::empty(),
        AssetBody::Bytes(bytes) => Body::from(bytes),
        AssetBody::Static(bytes) => Body::from(bytes),
    };
    let mut response = Response::new(body);
    *response.status_mut() = asset.status;
    *response.headers_mut() = asset.headers;
    response
}
