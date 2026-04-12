use std::env;
use std::time::Duration;

use aws_lambda_events::apigw::{ApiGatewayV2httpRequest, ApiGatewayV2httpResponse};
use aws_sdk_s3::presigning::PresigningConfig;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Deserialize)]
struct UploadRequest {
    filename: String,
    content_type: String,
}

#[derive(Serialize)]
struct UploadResponse {
    upload_url: String,
    key: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let config = aws_config::load_from_env().await;
    let s3 = aws_sdk_s3::Client::new(&config);

    lambda_runtime::run(service_fn(|event| handle(event, &s3))).await
}

async fn handle(
    event: LambdaEvent<ApiGatewayV2httpRequest>,
    s3: &aws_sdk_s3::Client,
) -> Result<ApiGatewayV2httpResponse, Error> {
    let bucket = env::var("PHOTO_BUCKET").expect("PHOTO_BUCKET not set");
    let allowed_origin = env::var("ALLOWED_ORIGIN").expect("ALLOWED_ORIGIN not set");

    let body = event.payload.body.as_deref().unwrap_or("");
    let req: UploadRequest = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(e) => {
            return Ok(error_response(400, &format!("Invalid request body: {e}"), &allowed_origin));
        }
    };

    let key = format!(
        "{}/{}",
        uuid_prefix(),
        sanitize_filename(&req.filename)
    );

    info!(key = %key, "Generating presigned URL");

    let presigned = s3
        .put_object()
        .bucket(&bucket)
        .key(&key)
        .content_type(&req.content_type)
        .presigned(
            PresigningConfig::builder()
                .expires_in(Duration::from_secs(300))
                .build()?,
        )
        .await?;

    let response = UploadResponse {
        upload_url: presigned.uri().to_string(),
        key,
    };

    Ok(ApiGatewayV2httpResponse {
        status_code: 200,
        headers: cors_headers(&allowed_origin),
        body: Some(serde_json::to_string(&response)?.into()),
        is_base64_encoded: Some(false),
        ..Default::default()
    })
}

fn error_response(status: i64, message: &str, origin: &str) -> ApiGatewayV2httpResponse {
    ApiGatewayV2httpResponse {
        status_code: status,
        headers: cors_headers(origin),
        body: Some(format!(r#"{{"error":"{message}"}}"#).into()),
        is_base64_encoded: Some(false),
        ..Default::default()
    }
}

fn cors_headers(origin: &str) -> aws_lambda_events::http::HeaderMap {
    let mut headers = aws_lambda_events::http::HeaderMap::new();
    headers.insert("Access-Control-Allow-Origin", origin.parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers
}

/// Generate a short time-based prefix to avoid filename collisions.
fn uuid_prefix() -> String {``
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{ts:x}")
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect()
}
