//! HTTP type conversion across Actix Web's `http` 0.2 and core's `http` 1 boundary.

use actix_web::{HttpRequest, dev::ServiceRequest};
use inertia_core::RequestParts;

pub(crate) fn request_parts(request: &HttpRequest) -> Result<RequestParts, String> {
    convert(
        request.method().as_str(),
        &request.uri().to_string(),
        request.headers(),
    )
}

pub(crate) fn service_request_parts(request: &ServiceRequest) -> Result<RequestParts, String> {
    convert(
        request.method().as_str(),
        &request.uri().to_string(),
        request.headers(),
    )
}

fn convert(
    method: &str,
    uri: &str,
    source: &actix_web::http::header::HeaderMap,
) -> Result<RequestParts, String> {
    let method = method
        .parse()
        .map_err(|error| format!("invalid request method at Actix boundary: {error}"))?;
    let uri = uri
        .parse()
        .map_err(|error| format!("invalid request URI at Actix boundary: {error}"))?;
    let mut headers = http::HeaderMap::new();
    for (name, value) in source {
        let name = http::HeaderName::from_bytes(name.as_str().as_bytes())
            .map_err(|error| format!("invalid request header name at Actix boundary: {error}"))?;
        let value = http::HeaderValue::from_bytes(value.as_bytes())
            .map_err(|error| format!("invalid request header value at Actix boundary: {error}"))?;
        if is_inertia_protocol_header(&name) && value.to_str().is_err() {
            return Err(format!(
                "non-UTF-8 Inertia protocol header at Actix boundary: {name}"
            ));
        }
        headers.append(name, value);
    }
    Ok(RequestParts::new(method, uri, headers))
}

fn is_inertia_protocol_header(name: &http::HeaderName) -> bool {
    let name = name.as_str();
    name.starts_with("x-inertia") || matches!(name, "accept" | "purpose" | "x-requested-with")
}

#[cfg(test)]
mod tests {
    use super::request_parts;
    use actix_web::test::TestRequest;

    #[test]
    fn preserves_uri_and_repeated_header_values() {
        let request = TestRequest::get()
            .uri("/users?active=true")
            .append_header(("x-inertia-partial-data", "users"))
            .append_header(("x-inertia-partial-data", "teams"))
            .to_http_request();
        let parts = request_parts(&request).unwrap();
        assert_eq!(parts.uri().path_and_query().unwrap(), "/users?active=true");
        let values = parts
            .headers()
            .get_all("x-inertia-partial-data")
            .iter()
            .map(|value| value.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(values, ["users", "teams"]);
    }

    #[test]
    fn rejects_non_utf8_inertia_protocol_headers() {
        let request = TestRequest::get()
            .append_header((
                actix_web::http::header::HeaderName::from_static("x-inertia"),
                actix_web::http::header::HeaderValue::from_bytes(&[0x80]).unwrap(),
            ))
            .to_http_request();
        let error = request_parts(&request).unwrap_err();
        assert_eq!(
            error,
            "non-UTF-8 Inertia protocol header at Actix boundary: x-inertia"
        );
    }

    #[test]
    fn preserves_non_protocol_header_bytes() {
        let request = TestRequest::get()
            .append_header((
                actix_web::http::header::HeaderName::from_static("x-binary"),
                actix_web::http::header::HeaderValue::from_bytes(&[0x80]).unwrap(),
            ))
            .to_http_request();
        let parts = request_parts(&request).unwrap();
        assert_eq!(parts.headers().get("x-binary").unwrap().as_bytes(), &[0x80]);
    }
}
