use crate::{TurnstileConfig, TurnstileMiddleware};
use tower_layer::Layer;

/// Layer that applies Turnstile verification middleware
#[derive(Clone)]
pub struct TurnstileLayer {
    config: TurnstileConfig,
}

impl TurnstileLayer {
    /// Create a new Turnstile layer with the given config
    pub fn new(config: TurnstileConfig) -> Self {
        Self { config }
    }

    /// Create a new Turnstile layer with just a secret key
    pub fn from_secret(secret: impl Into<String>) -> Self {
        Self::new(TurnstileConfig::new(secret))
    }
}

impl<S> Layer<S> for TurnstileLayer {
    type Service = TurnstileMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TurnstileMiddleware::new(inner, self.config.clone())
    }
}
