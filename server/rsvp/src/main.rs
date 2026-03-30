use aws_lambda_events::{apigw::ApiGatewayProxyResponse, http::HeaderMap};
use lambda_runtime::{Error, LambdaEvent, service_fn};
use serde_json::{Value, json};

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    lambda_runtime::run(service_fn(run_lambda)).await
}

async fn run_lambda(event: LambdaEvent<Value>) -> Result<ApiGatewayProxyResponse, Error> {
    let parsed_input = json!(event.payload);

    let resp = match parsed_input.get("routeKey").map(|v| v.as_str()).flatten() {
        Some("PUT /rsvp") => todo!("implement PUT /rsvp"),
        Some("GET /rsvp") => todo!("implement GET /rsvp"),
        _ => ApiGatewayProxyResponse {
            status_code: 400,
            multi_value_headers: HeaderMap::new(),
            is_base64_encoded: Some(false),
            body: None,
            headers: HeaderMap::new(),
        },
    };

    Ok(resp)
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test() {
        assert!(true);
    }
}
