resource "aws_apigatewayv2_api" "rsvp_api" {
  name          = var.api_name
  protocol_type = "HTTP"
  cors_configuration {
    allow_origins = ["*"]
    allow_methods = ["GET", "PUT", "OPTIONS"]
    allow_headers = ["Content-Type"]
  }
}

# GET /rsvp endpoint
resource "aws_apigatewayv2_integration" "get_rsvp_integration" {
  api_id                 = aws_apigatewayv2_api.rsvp_api.id
  integration_type       = "AWS_PROXY"
  integration_method     = "POST"
  integration_uri        = aws_lambda_function.get_rsvp.arn
  payload_format_version = "2.0"
}

# PUT /rsvp endpoint
resource "aws_apigatewayv2_integration" "put_rsvp_integration" {
  api_id                 = aws_apigatewayv2_api.rsvp_api.id
  integration_type       = "AWS_PROXY"
  integration_method     = "POST"
  integration_uri        = aws_lambda_function.put_rsvp.arn
  payload_format_version = "2.0"
}

# GET route
resource "aws_apigatewayv2_route" "get_rsvp_route" {
  api_id    = aws_apigatewayv2_api.rsvp_api.id
  route_key = "GET /rsvp"
  target    = "integrations/${aws_apigatewayv2_integration.get_rsvp_integration.id}"
}

# PUT route
resource "aws_apigatewayv2_route" "put_rsvp_route" {
  api_id    = aws_apigatewayv2_api.rsvp_api.id
  route_key = "PUT /rsvp"
  target    = "integrations/${aws_apigatewayv2_integration.put_rsvp_integration.id}"
}

# API Stage
resource "aws_apigatewayv2_stage" "rsvp_stage" {
  api_id      = aws_apigatewayv2_api.rsvp_api.id
  name        = "$default"
  auto_deploy = true
}

# Lambda permission for API Gateway to invoke GET function
resource "aws_lambda_permission" "get_rsvp_api_permission" {
  statement_id  = "AllowAPIGatewayInvoke"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.get_rsvp.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.rsvp_api.execution_arn}/*/*"
}

# Lambda permission for API Gateway to invoke PUT function
resource "aws_lambda_permission" "put_rsvp_api_permission" {
  statement_id  = "AllowAPIGatewayInvoke"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.put_rsvp.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.rsvp_api.execution_arn}/*/*"
}

# Data source for current region
data "aws_region" "current" {}
