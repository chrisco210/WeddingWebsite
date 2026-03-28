output "api_endpoint" {
  description = "API Gateway endpoint URL"
  value       = aws_apigatewayv2_stage.rsvp_stage.invoke_url
}

output "dynamodb_table_name" {
  description = "DynamoDB table name"
  value       = aws_dynamodb_table.rsvp.name
}

output "dynamodb_table_arn" {
  description = "DynamoDB table ARN"
  value       = aws_dynamodb_table.rsvp.arn
}

output "lambda_put_function_name" {
  description = "PUT RSVP Lambda function name"
  value       = aws_lambda_function.put_rsvp.function_name
}

output "lambda_get_function_name" {
  description = "GET RSVP Lambda function name"
  value       = aws_lambda_function.get_rsvp.function_name
}

output "api_id" {
  description = "API Gateway ID"
  value       = aws_apigatewayv2_api.rsvp_api.id
}
