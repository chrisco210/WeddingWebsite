resource "aws_dynamodb_table" "rsvp" {
  name         = var.table_name
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "party_id"

  attribute {
    name = "party_id"
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
