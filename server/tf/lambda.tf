# Archive the PUT Lambda function
data "archive_file" "lambda_put_zip" {
  type        = "zip"
  source_file = "${path.module}/../src/lambda_put_rsvp.js"
  output_path = "${path.module}/../dist/lambda_put_rsvp.zip"
}

# Archive the GET Lambda function
data "archive_file" "lambda_get_zip" {
  type        = "zip"
  source_file = "${path.module}/../src/lambda_get_rsvp.js"
  output_path = "${path.module}/../dist/lambda_get_rsvp.zip"
}

# PUT RSVP Lambda function
resource "aws_lambda_function" "put_rsvp" {
  filename         = data.archive_file.lambda_put_zip.output_path
  function_name    = var.lambda_function_name_put
  role             = aws_iam_role.lambda_role.arn
  handler          = "lambda_put_rsvp.handler"
  source_code_hash = data.archive_file.lambda_put_zip.output_base64sha256
  runtime          = "nodejs18.x"

  environment {
    variables = {
      TABLE_NAME = aws_dynamodb_table.rsvp.name
    }
  }
}

# GET RSVP Lambda function
resource "aws_lambda_function" "get_rsvp" {
  filename         = data.archive_file.lambda_get_zip.output_path
  function_name    = var.lambda_function_name_get
  role             = aws_iam_role.lambda_role.arn
  handler          = "lambda_get_rsvp.handler"
  source_code_hash = data.archive_file.lambda_get_zip.output_base64sha256
  runtime          = "nodejs18.x"

  environment {
    variables = {
      TABLE_NAME = aws_dynamodb_table.rsvp.name
    }
  }
}

# CloudWatch Log Group for PUT Lambda
resource "aws_cloudwatch_log_group" "put_rsvp_logs" {
  name              = "/aws/lambda/${aws_lambda_function.put_rsvp.function_name}"
  retention_in_days = 14
}

# CloudWatch Log Group for GET Lambda
resource "aws_cloudwatch_log_group" "get_rsvp_logs" {
  name              = "/aws/lambda/${aws_lambda_function.get_rsvp.function_name}"
  retention_in_days = 14
}
