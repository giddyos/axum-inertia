//! Actix form extraction and validation backed by the core form model.

use crate::{boundary, response::pending_response};
use actix_web::{
    Error, FromRequest, HttpRequest, HttpResponse, ResponseError, dev::Payload, http::StatusCode,
    web::Bytes,
};
use futures_util::future::LocalBoxFuture;
use serde::de::DeserializeOwned;
use std::{
    cell::RefCell,
    fmt,
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub use inertia_core::form::serialize_old_input;

/// Parsed Inertia form plus redirect metadata.
pub struct InertiaForm<T>(inertia_core::Form<T>);

impl<T> InertiaForm<T> {
    /// Returns the parsed value without validating it.
    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }

    /// Applies application-defined validation.
    pub fn validate_with<F>(self, validate: F) -> std::result::Result<T, FormError>
    where
        F: FnOnce(&T) -> std::result::Result<(), inertia_core::Errors>,
    {
        self.0.validate_with(validate).map_err(FormError::from)
    }
}

impl<T> Deref for InertiaForm<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for InertiaForm<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A typed form value that passed validation.
pub struct Validated<T>(pub T);

/// Actix rejection for form decoding or semantic validation.
pub struct FormError(FormErrorKind);

enum FormErrorKind {
    BadRequest(String),
    UnsupportedMediaType,
    Validation(Rc<RefCell<Option<inertia_core::PendingResponse>>>),
}

impl From<inertia_core::FormError> for FormError {
    fn from(error: inertia_core::FormError) -> Self {
        let kind = match error {
            inertia_core::FormError::BadRequest(error) => FormErrorKind::BadRequest(error),
            inertia_core::FormError::UnsupportedMediaType => FormErrorKind::UnsupportedMediaType,
            inertia_core::FormError::Validation(validation) => FormErrorKind::Validation(Rc::new(
                RefCell::new(Some(inertia_core::PendingResponse::InvalidForm(validation))),
            )),
        };
        Self(kind)
    }
}

impl fmt::Debug for FormError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("FormError")
            .field(&self.to_string())
            .finish()
    }
}

impl fmt::Display for FormError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            FormErrorKind::BadRequest(error) => {
                write!(formatter, "invalid Inertia form body: {error}")
            }
            FormErrorKind::UnsupportedMediaType => formatter.write_str(
                "InertiaForm supports application/json and application/x-www-form-urlencoded; use a separate multipart extractor for file uploads",
            ),
            FormErrorKind::Validation(_) => {
                formatter.write_str("Inertia form validation failed")
            }
        }
    }
}

impl std::error::Error for FormError {}

impl ResponseError for FormError {
    fn status_code(&self) -> StatusCode {
        match &self.0 {
            FormErrorKind::BadRequest(_) => StatusCode::BAD_REQUEST,
            FormErrorKind::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            FormErrorKind::Validation(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match &self.0 {
            FormErrorKind::BadRequest(error) => HttpResponse::BadRequest().body(error.clone()),
            FormErrorKind::UnsupportedMediaType => {
                HttpResponse::UnsupportedMediaType().body(
                    "InertiaForm supports JSON and URL-encoded bodies; multipart requires a separate extractor",
                )
            }
            FormErrorKind::Validation(pending) => pending.borrow_mut().take().map_or_else(
                || {
                    HttpResponse::InternalServerError()
                        .body("Actix Inertia validation response was already consumed")
                },
                pending_response,
            ),
        }
    }
}

impl<T> FromRequest for InertiaForm<T>
where
    T: DeserializeOwned + 'static,
{
    type Error = Error;
    type Future = LocalBoxFuture<'static, std::result::Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let parts = boundary::request_parts(request);
        let bytes = Bytes::from_request(request, payload);
        Box::pin(async move {
            let parts = parts.map_err(actix_web::error::ErrorBadRequest)?;
            let bytes = bytes.await?;
            inertia_core::Form::from_bytes(&parts, &bytes)
                .map(Self)
                .map_err(|error| FormError::from(error).into())
        })
    }
}

impl<T> FromRequest for Validated<T>
where
    T: DeserializeOwned + inertia_core::Validate + 'static,
{
    type Error = Error;
    type Future = LocalBoxFuture<'static, std::result::Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let form = InertiaForm::<T>::from_request(request, payload);
        Box::pin(async move {
            let form = form.await?;
            inertia_core::Validated::from_form(form.0)
                .map(|validated| Self(validated.0))
                .map_err(|error| FormError::from(error).into())
        })
    }
}
