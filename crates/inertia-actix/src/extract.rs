//! Actix extractors backed by core-prepared request state.

use crate::{DynamicPage, Location, PendingPage, Redirect, Response, Result};
use actix_web::{
    Error, FromRequest, HttpMessage, HttpRequest, dev::Payload, error::ErrorInternalServerError,
    web::Data,
};
use futures_util::future::{Ready, ready};
use serde::Serialize;
use std::{cell::RefCell, ops::Deref, rc::Rc};

/// Request-local one-shot storage for core-owned preparation state.
#[derive(Clone)]
pub(crate) struct PreparedHandle(Rc<RefCell<Option<inertia_core::PreparedRequest>>>);

impl PreparedHandle {
    pub(crate) fn new(prepared: inertia_core::PreparedRequest) -> Self {
        Self(Rc::new(RefCell::new(Some(prepared))))
    }

    pub(crate) fn take(&self) -> Option<inertia_core::PreparedRequest> {
        self.0.borrow_mut().take()
    }
}

/// Request-aware asynchronous Inertia rendering façade.
pub struct Inertia {
    prepared: PreparedHandle,
    visit: inertia_core::Visit,
}

impl Inertia {
    /// Constructs a legacy framework-neutral page value.
    pub fn response<C: Into<String>, T>(component: C, props: T) -> inertia_core::Inertia<T> {
        inertia_core::Inertia::response(component, props)
    }

    /// Starts the legacy advanced page builder.
    pub fn page(component: impl Into<String>) -> inertia_core::InertiaPageBuilder {
        inertia_core::Inertia::page(component)
    }

    /// Builds and asynchronously finalizes a page.
    pub async fn render(self, component: impl Into<String>, props: impl Serialize) -> Result {
        let value = serde_json::to_value(props).map_err(ErrorInternalServerError)?;
        let mut page = DynamicPage::new(component);
        match value {
            serde_json::Value::Object(values) => {
                for (key, value) in values {
                    page = page.prop(key, value);
                }
            }
            value => page = page.prop("value", value),
        }
        self.finalize(inertia_core::PendingResponse::Page(Box::new(
            page.into_pending_page().into_core(),
        )))
        .await
    }

    /// Asynchronously finalizes a derived typed page.
    pub async fn render_typed(self, page: impl inertia_core::InertiaPage) -> Result {
        self.finalize(inertia_core::PendingResponse::Page(Box::new(
            PendingPage::typed(page).into_core(),
        )))
        .await
    }

    /// Asynchronously finalizes a method-aware redirect.
    pub async fn to(self, url: impl Into<String>) -> Result {
        self.finalize(inertia_core::PendingResponse::Redirect(
            Redirect::to(url).into_core(),
        ))
        .await
    }

    /// Asynchronously finalizes an external location visit.
    pub async fn external(self, url: impl Into<String>) -> Result {
        self.finalize(inertia_core::PendingResponse::Location(
            Location::external(url).into_core(),
        ))
        .await
    }

    /// Returns the parsed framework-neutral visit.
    pub fn visit(&self) -> &inertia_core::Visit {
        &self.visit
    }

    async fn finalize(self, pending: inertia_core::PendingResponse) -> Result {
        let prepared = self.prepared.take().ok_or_else(|| {
            ErrorInternalServerError("this Actix Inertia request was already finalized")
        })?;
        #[cfg(feature = "ssr")]
        let response = prepared.finalize_with_ssr(pending, None).await;
        #[cfg(not(feature = "ssr"))]
        let response = prepared.finalize(pending).await;
        Ok(Response(response))
    }
}

impl Deref for Inertia {
    type Target = inertia_core::Visit;

    fn deref(&self) -> &Self::Target {
        &self.visit
    }
}

impl FromRequest for Inertia {
    type Error = Error;
    type Future = Ready<std::result::Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        if request
            .app_data::<Data<inertia_core::InertiaApp>>()
            .is_none()
        {
            return ready(Err(ErrorInternalServerError(
                "inertia-actix app data is not installed; register inertia_actix::configure or Data<InertiaApp>",
            )));
        }
        let extensions = request.extensions();
        let prepared = extensions.get::<PreparedHandle>().cloned();
        let visit = extensions.get::<inertia_core::Visit>().cloned();
        ready(match (prepared, visit) {
            (Some(prepared), Some(visit)) => Ok(Self { prepared, visit }),
            _ => Err(ErrorInternalServerError(
                "inertia-actix is not installed; register InertiaMiddleware on the Actix App",
            )),
        })
    }
}

/// Parsed visit extractor that remains available without an installed app.
#[derive(Clone, Debug)]
pub struct Visit(inertia_core::Visit);

impl Deref for Visit {
    type Target = inertia_core::Visit;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for Visit {
    type Error = Error;
    type Future = Ready<std::result::Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        if let Some(visit) = request.extensions().get::<inertia_core::Visit>().cloned() {
            return ready(Ok(Self(visit)));
        }
        ready(
            crate::boundary::request_parts(request)
                .map(inertia_core::Visit::from)
                .map(Self)
                .map_err(ErrorInternalServerError),
        )
    }
}
