use lambda_runtime::{Error, LambdaEvent, service_fn};
use serde_json::{Value, json};

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_runtime::run(service_fn(run_lambda)).await
}

async fn run_lambda(event: LambdaEvent<Value>) -> Result<Value, Error> {
    Ok(json!(event.payload))
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn test() {
        assert!(true);
    }
}
