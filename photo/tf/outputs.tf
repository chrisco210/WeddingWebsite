output "api_endpoint" {
  description = "API Gateway endpoint URL"
  value       = aws_apigatewayv2_stage.default.invoke_url
}

output "photo_bucket_name" {
  description = "S3 bucket for photo uploads"
  value       = aws_s3_bucket.photos.bucket
}
