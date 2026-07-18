//! Actix application-data and asset-route configuration helpers.

use actix_web::{
    HttpRequest, HttpResponse,
    web::{self, Data, Path, ServiceConfig},
};
use inertia_core::{AssetRequest, AssetSource, InertiaApp};
use std::sync::Arc;

/// Registers Inertia application data and its optional framework-neutral asset source.
///
/// Actix middleware changes `App<T>`'s concrete service type, so install
/// [`crate::InertiaMiddleware`] separately with `.wrap(...)`.
pub fn configure(inertia: InertiaApp) -> impl FnOnce(&mut ServiceConfig) + Clone {
    move |config| {
        config.app_data(Data::new(inertia.clone()));
        assets(inertia)(config);
    }
}

/// Registers the configured asset source, if one exists.
pub fn assets(inertia: InertiaApp) -> impl FnOnce(&mut ServiceConfig) + Clone {
    move |config| {
        let Some(source) = inertia.asset_source().cloned() else {
            return;
        };
        let public_path = inertia.asset_public_path();
        let pattern = if public_path == "/" {
            "/{path:.*}".to_owned()
        } else {
            format!("{public_path}/{{path:.*}}")
        };
        config
            .app_data(Data::new(source))
            .route(&pattern, web::route().to(embedded_asset));
    }
}

async fn embedded_asset(
    request: HttpRequest,
    path: Path<String>,
    source: Data<Arc<dyn AssetSource>>,
) -> HttpResponse {
    let method = match request.method().as_str().parse() {
        Ok(method) => method,
        Err(error) => {
            return HttpResponse::InternalServerError()
                .body(format!("invalid asset method at Actix boundary: {error}"));
        }
    };
    let mut headers = http::HeaderMap::new();
    for (name, value) in request.headers() {
        let name = match http::HeaderName::from_bytes(name.as_str().as_bytes()) {
            Ok(name) => name,
            Err(error) => {
                return HttpResponse::InternalServerError().body(format!(
                    "invalid asset header name at Actix boundary: {error}"
                ));
            }
        };
        let value = match http::HeaderValue::from_bytes(value.as_bytes()) {
            Ok(value) => value,
            Err(error) => {
                return HttpResponse::InternalServerError().body(format!(
                    "invalid asset header value at Actix boundary: {error}"
                ));
            }
        };
        headers.append(name, value);
    }
    let request = AssetRequest {
        method: &method,
        path: &path,
        headers: &headers,
    };
    source.get_ref().as_ref().get(request).map_or_else(
        || HttpResponse::NotFound().finish(),
        crate::assets::asset_response,
    )
}
