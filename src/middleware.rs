use crate::{verifier, TurnstileConfig, VerifiedTurnstile};
use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
};
use futures_util::future::BoxFuture;
use std::task::{Context, Poll};
use tower_service::Service;

/// Middleware that verifies Turnstile tokens
#[derive(Clone)]
pub struct TurnstileMiddleware<S> {
    inner: S,
    config: TurnstileConfig,
}

impl<S> TurnstileMiddleware<S> {
    pub fn new(inner: S, config: TurnstileConfig) -> Self {
        Self { inner, config }
    }
}

impl<S> Service<Request<Body>> for TurnstileMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let config = self.config.clone();
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);

        Box::pin(async move {
            // Extract token from header
            let token = req
                .headers()
                .get(&config.header_name)
                .and_then(|v| v.to_str().ok());

            let token = match token {
                Some(t) => t,
                None => {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from("Missing Turnstile token"))
                        .unwrap());
                }
            };

            // Verify token
            match verifier::verify_token(token, &config).await {
                Ok(true) => {
                    // Token is valid - add marker to extensions
                    req.extensions_mut().insert(VerifiedTurnstile);
                    inner.call(req).await
                }
                Ok(false) => Ok(Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Turnstile verification failed"))
                    .unwrap()),
                Err(e) => {
                    eprintln!("Turnstile verification error: {}", e);
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("Verification error"))
                        .unwrap())
                }
            }
        })
    }
}
