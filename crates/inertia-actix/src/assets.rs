//! Actix conversion for framework-neutral asset responses.

use actix_web::{HttpResponse, body::BoxBody};
use inertia_core::{AssetBody, AssetResponse};

pub(crate) fn asset_response(asset: AssetResponse) -> HttpResponse<BoxBody> {
    let status = actix_web::http::StatusCode::from_u16(asset.status.as_u16())
        .expect("asset source emitted a valid HTTP status");
    let mut builder = HttpResponse::build(status);
    for (name, value) in &asset.headers {
        let name = actix_web::http::header::HeaderName::from_bytes(name.as_str().as_bytes())
            .expect("asset source emitted a valid HTTP header name");
        let value = actix_web::http::header::HeaderValue::from_bytes(value.as_bytes())
            .expect("asset source emitted a valid HTTP header value");
        builder.append_header((name, value));
    }
    match asset.body {
        AssetBody::Empty => builder.finish(),
        AssetBody::Bytes(bytes) => builder.body(bytes),
        AssetBody::Static(bytes) => builder.body(bytes),
    }
}
