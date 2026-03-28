resource "aws_dynamodb_table" "rsvp" {
  name         = var.table_name
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "guest_id"
  range_key    = "guest_email"

  attribute {
    name = "guest_id"
    type = "S"
  }

  attribute {
    name = "guest_email"
    type = "S"
  }

  ttl {
    attribute_name = "ttl"
    enabled        = true
  }

  tags = {
    Environment = var.environment
    Name        = var.table_name
  }
}
