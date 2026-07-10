//! HTML response context and serialization helpers.

mod serializer;

use self::serializer::to_script_safe_json;
use serde::Serialize;

pub use crate::page::builder::HtmlResponseContext;

pub(crate) fn html_response_context<T>(page: &T) -> Result<HtmlResponseContext, serde_json::Error>
where
    T: Serialize + ?Sized,
{
    to_script_safe_json(page).map(HtmlResponseContext::new)
}
