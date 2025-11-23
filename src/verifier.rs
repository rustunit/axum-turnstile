use crate::{TurnstileConfig, VerifyRequest, VerifyResponse};

/// Verify a Turnstile token with Cloudflare
pub async fn verify_token(
    token: &str,
    config: &TurnstileConfig,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();

    let response = client
        .post(&config.verify_url)
        .json(&VerifyRequest {
            secret: config.secret.clone(),
            response: token.to_string(),
        })
        .send()
        .await?;

    let result: VerifyResponse = response.json().await?;

    if !result.success
        && let Some(errors) = result.error_codes
    {
        eprintln!("Turnstile verification failed: {:?}", errors);
    }

    Ok(result.success)
}
