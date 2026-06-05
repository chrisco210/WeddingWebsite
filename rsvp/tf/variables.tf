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

variable "guest_list_object_key" {
  description = "S3 object key for the guest list CSV"
  type        = string
  default     = "guests.csv"
}

variable "welcome_party_object_key" {
  description = "S3 object key for the welcome party guest list"
  type        = string
  default     = "welcome_party.txt"
}
