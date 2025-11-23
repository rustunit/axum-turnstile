# axum-turnstile

[![Crates.io](https://img.shields.io/crates/v/axum-turnstile.svg)](https://crates.io/crates/axum-turnstile)
[![Documentation](https://docs.rs/axum-turnstile/badge.svg)](https://docs.rs/axum-turnstile)
[![License](https://img.shields.io/crates/l/axum-turnstile.svg)](https://github.com/rustunit/axum-turnstile#license)

**Cloudflare Turnstile verification middleware for Axum**

Protect your Axum web applications from bots and abuse with [Cloudflare Turnstile](https://www.cloudflare.com/products/turnstile/) - a privacy-first, user-friendly CAPTCHA alternative. This crate provides a seamless integration as Tower middleware.

## Features

- ✨ Drop-in middleware for Axum routes
- 🎯 Type-safe verification with extractors
- ⚙️ Customizable headers and endpoints
- 🧪 Built-in support for test keys
- 📦 Minimal dependencies

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
axum-turnstile = "0.1"
```

## Quick Start

### 1. Get Your Turnstile Keys

Sign up at [Cloudflare Dashboard](https://dash.cloudflare.com/) and create a Turnstile site to get your:
- **Site Key** (public, used in your frontend)
- **Secret Key** (private, used in this middleware)

### 2. Add the Middleware

```rust
use axum::{routing::post, Router};
use axum_turnstile::{TurnstileLayer, VerifiedTurnstile};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/submit", post(submit_handler))
        // Protect this route with Turnstile
        .layer(TurnstileLayer::from_secret("your-secret-key"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    axum::serve(listener, app).await.unwrap();
}

// This handler will only be called if Turnstile verification succeeds
async fn submit_handler(_verified: VerifiedTurnstile) -> &'static str {
    "Form submitted successfully!"
}
```

### 3. Frontend Integration

Include the Turnstile widget in your HTML and send the token with your request:

```html
<!DOCTYPE html>
<html>
<head>
    <script src="https://challenges.cloudflare.com/turnstile/v0/api.js" async defer></script>
</head>
<body>
    <form id="myForm">
        <!-- Turnstile widget -->
        <div class="cf-turnstile" data-sitekey="your-site-key"></div>
        <button type="submit">Submit</button>
    </form>

    <script>
        document.getElementById('myForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            
            // Get the Turnstile token
            const token = document.querySelector('[name="cf-turnstile-response"]').value;
            
            // Send it to your protected endpoint
            const response = await fetch('/api/submit', {
                method: 'POST',
                headers: {
                    'CF-Turnstile-Token': token,
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({ /* your data */ })
            });
            
            if (response.ok) {
                alert('Success!');
            } else {
                alert('Verification failed');
            }
        });
    </script>
</body>
</html>
```

## Advanced Usage

### Custom Configuration

```rust
use axum_turnstile::{TurnstileConfig, TurnstileLayer};

let config = TurnstileConfig::new("your-secret-key")
    .with_header_name("X-Custom-Turnstile-Token")
    .with_verify_url("https://custom-endpoint.example.com/verify");

let layer = TurnstileLayer::new(config);
```

### Selective Route Protection

You can apply the middleware to specific routes by using nested routers:

```rust
use axum::{routing::{get, post}, Router};
use axum_turnstile::TurnstileLayer;

// Create a router with protected routes
let protected = Router::new()
    .route("/api/submit", post(submit))
    .route("/api/comment", post(comment))
    .layer(TurnstileLayer::from_secret("your-secret-key"));

// Merge with public routes
let app = Router::new()
    .route("/", get(home))
    .route("/about", get(about))
    .merge(protected);
```

Alternatively, you can nest protected routes under a common path:

```rust
use axum::{routing::{get, post}, Router};
use axum_turnstile::TurnstileLayer;

let app = Router::new()
    // Public routes
    .route("/", get(home))
    .route("/about", get(about))
    // Nest protected routes under /api
    .nest("/api", Router::new()
        .route("/submit", post(submit))
        .route("/comment", post(comment))
        .layer(TurnstileLayer::from_secret("your-secret-key"))
    );
```

### Using the Extractor

The `VerifiedTurnstile` type can be used as an extractor in any handler:

```rust
use axum::Json;
use axum_turnstile::VerifiedTurnstile;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct FormData {
    name: String,
    email: String,
}

#[derive(Serialize)]
struct Response {
    message: String,
}

async fn submit_form(
    _verified: VerifiedTurnstile,  // Ensures Turnstile was verified
    Json(data): Json<FormData>,
) -> Json<Response> {
    // Process the form data
    Json(Response {
        message: format!("Thanks for submitting, {}!", data.name)
    })
}
```

## Testing

Cloudflare provides test keys that always pass or fail verification:

### Always Passes
```rust
use axum_turnstile::TurnstileLayer;

// Secret key that always passes
let layer = TurnstileLayer::from_secret("1x0000000000000000000000000000000AA");
```

**Site key (frontend):** `1x00000000000000000000AA`

### Always Fails
```rust
// Secret key that always fails
let layer = TurnstileLayer::from_secret("2x0000000000000000000000000000000AA");
```

**Site key (frontend):** `2x00000000000000000000AA`

### Writing Tests

```rust
use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::post,
    Router,
};
use axum_turnstile::TurnstileLayer;
use tower::ServiceExt;

#[tokio::test]
async fn test_turnstile_verification() {
    let app = Router::new()
        .route("/submit", post(|| async { "OK" }))
        .layer(TurnstileLayer::from_secret("1x0000000000000000000000000000000AA"));

    let response = app
        .oneshot(
            Request::post("/submit")
                .header("CF-Turnstile-Token", "test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

## Response Status Codes

| Status Code | Reason |
|-------------|--------|
| `400 Bad Request` | The `CF-Turnstile-Token` header is missing from the request |
| `403 Forbidden` | The Turnstile token verification failed |
| `500 Internal Server Error` | Error communicating with Cloudflare's verification API |

## How It Works

1. **Client Request**: The client includes the Turnstile token in the request header
2. **Middleware Intercept**: The middleware extracts the token from the header
3. **Verification**: The token is verified with Cloudflare's API
4. **Success Path**: If valid, a `VerifiedTurnstile` marker is added to request extensions
5. **Handler Execution**: Your handler can extract the marker to ensure verification
6. **Failure Path**: If invalid or missing, an error response is returned immediately

```
┌─────────┐          ┌──────────────┐          ┌────────────┐          ┌─────────┐
│ Client  │─────────▶│  Turnstile   │─────────▶│ Cloudflare │─────────▶│ Handler │
│         │  Token   │  Middleware  │  Verify  │    API     │  Success │         │
└─────────┘          └──────────────┘          └────────────┘          └─────────┘
                            │
                            │ Invalid/Missing
                            ▼
                     ┌───────────────┐
                     │ Error Response│
                     └───────────────┘
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Resources

- [Cloudflare Turnstile Documentation](https://developers.cloudflare.com/turnstile/)
- [Axum Documentation](https://docs.rs/axum)
- [API Documentation](https://docs.rs/axum-turnstile)
