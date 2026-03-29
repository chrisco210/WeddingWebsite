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

variable "table_name" {
  description = "DynamoDB table name for RSVP"
  type        = string
  default     = "wedding-rsvp"
}

variable "lambda_function_name" {
  description = "Lambda function name for RSVP"
  type        = string
  default     = "rsvp"
}

variable "api_name" {
  description = "API Gateway name"
  type        = string
  default     = "wedding-rsvp-api"
}
