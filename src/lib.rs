//! # axum-turnstile
//!
//! Cloudflare Turnstile verification middleware for [Axum](https://github.com/tokio-rs/axum).
//!
//! This crate provides middleware for verifying [Cloudflare Turnstile](https://www.cloudflare.com/products/turnstile/)
//! tokens in Axum web applications. Turnstile is Cloudflare's privacy-first CAPTCHA alternative
//! that helps protect your application from bots and abuse.
//!
//! ## Features
//!
//! - 🔒 Easy integration with Axum applications
//! - 🎯 Tower middleware layer for flexible composition
//! - ⚙️ Configurable header names and verification endpoints
//! - 🧪 Support for Cloudflare's test keys
//! - 📦 Minimal dependencies
//!
//! ## Quick Start
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! axum-turnstile = "0.1"
//! axum = "0.8"
//! tokio = { version = "1", features = ["full"] }
//! ```
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use axum::{routing::post, Router};
//! use axum_turnstile::{TurnstileLayer, VerifiedTurnstile};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create a protected endpoint
//!     let app = Router::new()
//!         .route("/api/protected", post(protected_handler))
//!         .layer(TurnstileLayer::from_secret("your-secret-key"));
//!
//!     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
//!         .await
//!         .unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//!
//! // Handler that requires Turnstile verification
//! async fn protected_handler(_verified: VerifiedTurnstile) -> &'static str {
//!     "Success! Turnstile token verified."
//! }
//! ```
//!
//! ## How It Works
//!
//! 1. Client includes the Turnstile token in the `CF-Turnstile-Token` header
//! 2. Middleware extracts and verifies the token with Cloudflare's API
//! 3. If valid, the request proceeds and handlers can extract [`VerifiedTurnstile`]
//! 4. If invalid or missing, the request is rejected with an appropriate status code
//!
//! ## Advanced Configuration
//!
//! ```rust
//! use axum_turnstile::{TurnstileConfig, TurnstileLayer};
//!
//! let config = TurnstileConfig::new("your-secret-key")
//!     .with_header_name("X-Custom-Turnstile-Token")
//!     .with_verify_url("https://custom-endpoint.example.com/verify");
//!
//! let layer = TurnstileLayer::new(config);
//! ```
//!
//! ## Testing
//!
//! Cloudflare provides test keys that always pass or fail verification:
//!
//! - **Always passes**: `1x0000000000000000000000000000000AA`
//! - **Always fails**: `2x0000000000000000000000000000000AA`
//!
//! ```rust,no_run
//! use axum_turnstile::TurnstileLayer;
//!
//! // Use the test key that always passes
//! let layer = TurnstileLayer::from_secret("1x0000000000000000000000000000000AA");
//! ```
//!
//! ## Response Codes
//!
//! - `400 Bad Request`: Turnstile token header is missing
//! - `403 Forbidden`: Token verification failed
//! - `500 Internal Server Error`: Error communicating with Cloudflare's API
//!
//! ## Extracting the Verified Marker
//!
//! The [`VerifiedTurnstile`] type implements [`FromRequestParts`],
//! so you can use it as an extractor in your handlers:
//!
//! ```rust
//! use axum_turnstile::VerifiedTurnstile;
//!
//! async fn handler(_verified: VerifiedTurnstile) -> &'static str {
//!     "Only reached if Turnstile verification succeeded"
//! }
//! ```

mod layer;
mod middleware;
mod verifier;

pub use layer::TurnstileLayer;
pub use middleware::TurnstileMiddleware;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use serde::{Deserialize, Serialize};

/// Configuration for Turnstile verification
#[derive(Clone, Debug)]
pub struct TurnstileConfig {
    /// Cloudflare Turnstile secret key
    pub secret: String,
    /// Custom header name (default: "CF-Turnstile-Token")
    pub header_name: String,
    /// Verification endpoint (default: Cloudflare's endpoint)
    pub verify_url: String,
}

impl TurnstileConfig {
    /// Create a new config with the given secret
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            header_name: "CF-Turnstile-Token".to_string(),
            verify_url: "https://challenges.cloudflare.com/turnstile/v0/siteverify".to_string(),
        }
    }

    /// Set a custom header name
    pub fn with_header_name(mut self, name: impl Into<String>) -> Self {
        self.header_name = name.into();
        self
    }

    /// Set a custom verification URL (for testing)
    pub fn with_verify_url(mut self, url: impl Into<String>) -> Self {
        self.verify_url = url.into();
        self
    }
}

#[derive(Serialize)]
struct VerifyRequest {
    secret: String,
    response: String,
}

#[derive(Deserialize, Debug)]
struct VerifyResponse {
    success: bool,
    #[serde(rename = "error-codes")]
    error_codes: Option<Vec<String>>,
}

/// Marker type that can be extracted in handlers after successful verification
#[derive(Clone, Debug)]
pub struct VerifiedTurnstile;

impl<S> FromRequestParts<S> for VerifiedTurnstile
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<VerifiedTurnstile>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_missing_token() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(TurnstileLayer::from_secret("test-secret"));

        let response = app
            .oneshot(Request::get("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_with_test_key() {
        // Using Cloudflare's test key
        let app = Router::new().route("/test", get(|| async { "OK" })).layer(
            TurnstileLayer::from_secret("1x0000000000000000000000000000000AA"),
        );

        let response = app
            .oneshot(
                Request::get("/test")
                    .header("CF-Turnstile-Token", "test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
