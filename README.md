# WeddingWebsite

Wedding website

## Server

It is a Lambda/ApiGateway based service. Infra is managed with terraform in
`server/tf`. Service code is built using the rust lambda integration.

Build and deploy:

```
cargo lambda build --release --arm64 --output-format zip
terraform apply
```
