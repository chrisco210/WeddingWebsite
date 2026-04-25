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
  retention_in_days = 120
}

resource "aws_sns_topic" "lambda_alerts" {
  name = "${var.lambda_function_name}-alerts"
}

resource "aws_sns_topic_subscription" "lambda_alerts_email" {
  topic_arn = aws_sns_topic.lambda_alerts.arn
  protocol  = "email"
  endpoint  = "chrisrachlinski+rsvp@gmail.com"
}

resource "aws_cloudwatch_metric_alarm" "lambda_errors" {
  alarm_name          = "${var.lambda_function_name}-errors"
  comparison_operator = "GreaterThanOrEqualToThreshold"
  evaluation_periods  = 1
  metric_name         = "Errors"
  namespace           = "AWS/Lambda"
  period              = 600
  statistic           = "Sum"
  threshold           = 1
  alarm_description   = "Triggers when the RSVP Lambda has any errors"
  treat_missing_data  = "notBreaching"

  dimensions = {
    FunctionName = aws_lambda_function.rsvp.function_name
  }

  alarm_actions = [aws_sns_topic.lambda_alerts.arn]
}
