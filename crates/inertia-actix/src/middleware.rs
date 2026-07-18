//! Actix middleware delegating preparation and finalization to `inertia-core`.

use crate::{
    boundary::service_request_parts,
    extract::PreparedHandle,
    response::{PendingResponseHandle, core_response},
};
use actix_web::{
    Error, HttpMessage, HttpResponse,
    body::{BoxBody, EitherBody, MessageBody},
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::{LocalBoxFuture, Ready, ready};
use std::rc::Rc;

/// Installs a framework-neutral Inertia application in the Actix request lifecycle.
#[derive(Clone)]
pub struct InertiaMiddleware {
    app: inertia_core::InertiaApp,
}

impl InertiaMiddleware {
    /// Creates middleware for `app`.
    pub fn new(app: inertia_core::InertiaApp) -> Self {
        Self { app }
    }
}

impl<S, B> Transform<S, ServiceRequest> for InertiaMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type InitError = ();
    type Transform = InertiaMiddlewareService<S>;
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(InertiaMiddlewareService {
            service: Rc::new(service),
            app: self.app.clone(),
        }))
    }
}

/// Service produced by [`InertiaMiddleware`].
pub struct InertiaMiddlewareService<S> {
    service: Rc<S>,
    app: inertia_core::InertiaApp,
}

impl<S, B> Service<ServiceRequest> for InertiaMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let app = self.app.clone();
        Box::pin(async move {
            let parts = match service_request_parts(&request) {
                Ok(parts) => parts,
                Err(error) => {
                    return Ok(request
                        .into_response(HttpResponse::InternalServerError().body(error))
                        .map_into_right_body());
                }
            };
            let prepared = match app.prepare_request(parts, None).await {
                Ok(inertia_core::VersionCheck::Proceed(prepared)) => *prepared,
                Ok(inertia_core::VersionCheck::Mismatch(response)) => {
                    return Ok(request
                        .into_response(core_response(response))
                        .map_into_right_body());
                }
                Err(error) => {
                    return Ok(request
                        .into_response(core_response(error.into_response()))
                        .map_into_right_body());
                }
            };
            let visit = prepared.visit().clone();
            let prepared = PreparedHandle::new(prepared);
            request.extensions_mut().insert(visit);
            request.extensions_mut().insert(prepared.clone());

            let mut response = service.call(request).await?;
            let pending = response
                .response_mut()
                .extensions_mut()
                .remove::<PendingResponseHandle>()
                .and_then(|handle| handle.take());
            let Some(pending) = pending else {
                return Ok(response.map_into_left_body());
            };
            let Some(prepared) = prepared.take() else {
                return Ok(response
                    .into_response(
                        HttpResponse::InternalServerError()
                            .body("Actix Inertia request was already finalized"),
                    )
                    .map_into_right_body());
            };
            #[cfg(feature = "ssr")]
            let finalized = prepared.finalize_with_ssr(pending, None).await;
            #[cfg(not(feature = "ssr"))]
            let finalized = prepared.finalize(pending).await;
            Ok(response
                .into_response(core_response(finalized))
                .map_into_right_body())
        })
    }
}
