resource "aws_lambda_function" "rsvp" {
  filename         = "../target/lambda/rsvp/bootstrap.zip"
  function_name    = var.lambda_function_name
  role             = aws_iam_role.lambda_role.arn
  handler          = "rust.handler"
  runtime          = "provided.al2023"
  architectures    = ["arm64"]
  source_code_hash = filebase64sha256("../target/lambda/rsvp/bootstrap.zip")


  environment {
    variables = {
      TABLE_NAME = aws_dynamodb_table.rsvp.name
    }
  }
}

# CloudWatch Log Group for Lambda
resource "aws_cloudwatch_log_group" "rsvp_logs" {
  name              = "/aws/lambda/${aws_lambda_function.rsvp.function_name}"
  retention_in_days = 14
}
