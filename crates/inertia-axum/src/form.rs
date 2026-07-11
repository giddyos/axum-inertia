//! Redirect-based Inertia form extraction and validation.

use crate::{PendingResponse, RequestContext};
use axum::{
    body::to_bytes,
    extract::{FromRequest, Request},
    http::{
        StatusCode,
        header::{CONTENT_TYPE, REFERER},
    },
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use std::{
    fmt,
    ops::{Deref, DerefMut},
};

/// Standard field-to-message validation errors.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Errors(Map<String, Value>);
impl Errors {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.0.insert(field.into(), Value::String(message.into()));
    }
    pub fn field(field: impl Into<String>, message: impl Into<String>) -> Self {
        let mut errors = Self::new();
        errors.add(field, message);
        errors
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn into_value(self) -> Value {
        Value::Object(self.0)
    }
}

/// Local validation contract implemented by derives or application code.
pub trait Validate {
    fn validate(&self) -> Result<(), Errors>;
    fn error_bag() -> Option<&'static str> {
        None
    }
    fn old_input(&self) -> Option<Value> {
        None
    }
}

/// Parsed form plus request metadata for lower-level custom validation.
pub struct InertiaForm<T> {
    input: T,
    bag: Option<Box<str>>,
    back: Box<str>,
}
impl<T> InertiaForm<T> {
    pub fn into_inner(self) -> T {
        self.input
    }
    pub fn validate_with<F>(self, validate: F) -> Result<T, FormError>
    where
        F: FnOnce(&T) -> Result<(), Errors>,
    {
        match validate(&self.input) {
            Ok(()) => Ok(self.input),
            Err(errors) => Err(FormError::validation(errors, self.bag, self.back, None)),
        }
    }
}
impl<T> Deref for InertiaForm<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.input
    }
}
impl<T> DerefMut for InertiaForm<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.input
    }
}

/// A typed form value that passed validation.
pub struct Validated<T>(pub T);

#[derive(Debug)]
pub enum FormError {
    BadRequest(String),
    UnsupportedMediaType,
    Validation(crate::response::PendingValidation),
}
impl FormError {
    fn validation(
        errors: Errors,
        bag: Option<Box<str>>,
        back: Box<str>,
        old_input: Option<Value>,
    ) -> Self {
        let errors = if let Some(bag) = bag {
            let mut scoped = Map::new();
            scoped.insert(bag.into(), errors.into_value());
            Value::Object(scoped)
        } else {
            errors.into_value()
        };
        Self::Validation(crate::response::PendingValidation {
            errors,
            old_input,
            back,
        })
    }
}
impl fmt::Display for FormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRequest(error) => write!(f, "invalid Inertia form body: {error}"),
            Self::UnsupportedMediaType => f.write_str("InertiaForm supports application/json and application/x-www-form-urlencoded; use a separate multipart extractor for file uploads"),
            Self::Validation(_) => f.write_str("Inertia form validation failed"),
        }
    }
}
impl IntoResponse for FormError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(error) => (StatusCode::BAD_REQUEST, error).into_response(),
            Self::UnsupportedMediaType => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "InertiaForm supports JSON and URL-encoded bodies; multipart requires a separate extractor").into_response(),
            Self::Validation(validation) => crate::response::pending_response(PendingResponse::InvalidForm(validation)),
        }
    }
}

impl<S, T> FromRequest<S> for InertiaForm<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send + 'static,
{
    type Rejection = FormError;
    async fn from_request(request: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = request.headers();
        let context = RequestContext::from_header_fn(|name| {
            headers.get(name).and_then(|value| value.to_str().ok())
        });
        let bag = context.error_bag().map(Into::into);
        let back = headers
            .get(REFERER)
            .and_then(|value| value.to_str().ok())
            .map_or_else(
                || request.uri().path().to_owned().into_boxed_str(),
                Into::into,
            );
        let content_type = headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_owned();
        let bytes = to_bytes(request.into_body(), 2 * 1024 * 1024)
            .await
            .map_err(|error| FormError::BadRequest(error.to_string()))?;
        let input: T = match content_type.as_str() {
            "application/json" => serde_json::from_slice(&bytes)
                .map_err(|error| FormError::BadRequest(error.to_string()))?,
            "application/x-www-form-urlencoded" => serde_urlencoded::from_bytes(&bytes)
                .map_err(|error| FormError::BadRequest(error.to_string()))?,
            _ => return Err(FormError::UnsupportedMediaType),
        };
        Ok(Self { input, bag, back })
    }
}

impl<S, T> FromRequest<S> for Validated<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate + Send + 'static,
{
    type Rejection = FormError;
    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let form = InertiaForm::<T>::from_request(request, state).await?;
        match form.input.validate() {
            Ok(()) => Ok(Self(form.input)),
            Err(errors) => {
                let old_input = form.input.old_input();
                let bag = form.bag.or_else(|| T::error_bag().map(Into::into));
                Err(FormError::validation(errors, bag, form.back, old_input))
            }
        }
    }
}

#[doc(hidden)]
pub fn serialize_old_input(
    fields: impl IntoIterator<Item = (&'static str, Result<Value, serde_json::Error>)>,
) -> Value {
    let mut values = Map::new();
    for (name, value) in fields {
        if let Ok(value) = value {
            values.insert(name.to_owned(), value);
        }
    }
    Value::Object(values)
}
