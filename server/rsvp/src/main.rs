pub(crate) mod handler;
pub(crate) mod store;

use aws_lambda_events::{apigw::ApiGatewayProxyResponse, http::HeaderMap};
use lambda_runtime::{Error, LambdaEvent, service_fn};
use serde::Serialize;
use serde_json::Value;

use crate::handler::{HandlerError, HandlerImpl, PutRsvpInput};
use crate::store::DynamoRsvpStore;

fn json_response<V: Serialize>(val: V) -> ApiGatewayProxyResponse {
    match serde_json::to_string(&val) {
        Ok(body) => ApiGatewayProxyResponse {
            status_code: 200,
            multi_value_headers: HeaderMap::new(),
            is_base64_encoded: Some(false),
            body: Some(aws_lambda_events::encodings::Body::Text(body)),
            headers: {
                let mut h = HeaderMap::new();
                h.insert(
                    "content-type",
                    "application/json".parse().expect("valid header value"),
                );
                h
            },
        },
        Err(e) => error_response(500, &format!("serialization error: {e}")),
    }
}

fn error_response(status: i64, msg: &str) -> ApiGatewayProxyResponse {
    ApiGatewayProxyResponse {
        status_code: status,
        multi_value_headers: HeaderMap::new(),
        is_base64_encoded: Some(false),
        body: Some(aws_lambda_events::encodings::Body::Text(msg.to_string())),
        headers: HeaderMap::new(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let table_name =
        std::env::var("TABLE_NAME").unwrap_or_else(|_| "wedding-rsvp".to_string());
    let handler = HandlerImpl {
        store: DynamoRsvpStore {
            client: aws_sdk_dynamodb::Client::new(&config),
            table_name,
        },
    };

    lambda_runtime::run(service_fn(move |event: LambdaEvent<Value>| {
        let handler = handler.clone();
        async move { run_lambda(event, handler).await }
    }))
    .await
}

async fn run_lambda(
    event: LambdaEvent<Value>,
    handler: HandlerImpl<DynamoRsvpStore>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let payload = &event.payload;

    let route_key = payload
        .get("routeKey")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let query_param = |key: &str| {
        payload
            .get("queryStringParameters")
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_str())
    };

    let resp = match route_key {
        "GET /rsvp" => {
            if let Some(query) = query_param("search") {
                json_response(handler.search(query))
            } else if let Some(party_id) = query_param("party_id") {
                match handler.get_party(party_id).await {
                    Ok(r) => json_response(r),
                    Err(e) => ApiGatewayProxyResponse::from(e),
                }
            } else {
                error_response(400, "expected 'search' or 'party_id' query parameter")
            }
        }
        "PUT /rsvp" => {
            let body = payload
                .get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");
            match serde_json::from_str::<PutRsvpInput>(body) {
                Err(e) => error_response(400, &format!("invalid request body: {e}")),
                Ok(input) => {
                    let party_id = input.party_id.clone();
                    let responses_json =
                        serde_json::to_string(&input.responses).unwrap_or_default();
                    match handler.submit_rsvp(input).await {
                    Ok(()) => {
                        tracing::info!(
                            party_id = %party_id,
                            responses = %responses_json,
                            "RSVP submitted"
                        );
                        json_response(serde_json::json!({}))
                    }
                    Err(HandlerError::NotFound(m)) => error_response(404, &m),
                    Err(HandlerError::BadRequest(m)) => error_response(400, &m),
                    Err(HandlerError::Internal(m)) => error_response(500, &m),
                }}
            }
        }
        _ => error_response(400, "unknown route"),
    };

    Ok(resp)
}
