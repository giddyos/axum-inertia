use http::{HeaderMap, header::IF_NONE_MATCH};

pub(crate) const IMMUTABLE: &str = "public, max-age=31536000, immutable";
pub(crate) const REVALIDATE: &str = "public, max-age=0, must-revalidate";

pub(crate) fn etag_matches(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get_all(IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .any(|candidate| candidate == "*" || candidate.trim_start_matches("W/") == etag)
}
