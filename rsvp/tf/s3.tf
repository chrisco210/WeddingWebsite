resource "aws_s3_bucket" "guest_list" {
}

resource "aws_s3_bucket_versioning" "guest_list" {
  bucket = aws_s3_bucket.guest_list.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "guest_list" {
  bucket = aws_s3_bucket.guest_list.id

  depends_on = [aws_s3_bucket_versioning.guest_list]

  rule {
    id     = "expire-noncurrent-versions"
    status = "Enabled"

    noncurrent_version_expiration {
      noncurrent_days = 7
    }
  }
}

resource "aws_s3_object" "guests_csv" {
  bucket = aws_s3_bucket.guest_list.bucket
  key    = var.guest_list_object_key
  source = "${path.module}/guests.csv"
  etag   = filemd5("${path.module}/guests.csv")
}

resource "aws_s3_object" "welcome_party" {
  bucket = aws_s3_bucket.guest_list.bucket
  key    = var.welcome_party_object_key
  source = "${path.module}/welcome_party.txt"
  etag   = filemd5("${path.module}/welcome_party.txt")
}

resource "aws_s3_bucket_public_access_block" "guest_list" {
  bucket = aws_s3_bucket.guest_list.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}
