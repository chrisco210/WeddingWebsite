variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "us-east-2"
}

variable "environment" {
  description = "Environment name"
  type        = string
  default     = "production"
}

variable "lambda_function_name" {
  description = "Lambda function name"
  type        = string
  default     = "photo"
}

variable "api_name" {
  description = "API Gateway name"
  type        = string
  default     = "photo-api"
}

variable "photo_bucket_name" {
  description = "S3 bucket name for photo uploads"
  type        = string
  default     = "wedding-photos-uploads"
}

variable "allowed_origin" {
  description = "Allowed CORS origin (GitHub Pages URL)"
  type        = string
}
