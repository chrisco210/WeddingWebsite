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
  description = "Lambda function name for RSVP"
  type        = string
  default     = "photo"
}

variable "api_name" {
  description = "API Gateway name"
  type        = string
  default     = "photo-api"
}
