# Wedding RSVP Terraform Configuration

This directory contains Terraform configuration to deploy the Wedding RSVP infrastructure to AWS.

## Components

- **DynamoDB Table**: `wedding-rsvp` table to store RSVP responses
- **Lambda Functions**:
  - `rsvp-put`: Handles PUT requests to save RSVP responses
  - `rsvp-get`: Handles GET requests to retrieve RSVP responses
- **API Gateway**: HTTP API with two endpoints:
  - `PUT /rsvp`: Save an RSVP response
  - `GET /rsvp`: Retrieve RSVP response(s)
- **IAM Roles & Policies**: Permissions for Lambda to access DynamoDB
- **CloudWatch Logs**: Logging for Lambda functions and API Gateway

## Prerequisites

1. Install [Terraform](https://www.terraform.io/downloads.html)
2. Configure AWS credentials (via `~/.aws/credentials` or environment variables)
3. Ensure you have Node.js installed (for Lambda functions)

## Setup

1. Copy `terraform.tfvars.example` to `terraform.tfvars`:

   ```bash
   cp terraform.tfvars.example terraform.tfvars
   ```

2. Update `terraform.tfvars` with your desired values:

   ```bash
   nano terraform.tfvars
   ```

3. Initialize Terraform:
   ```bash
   terraform init
   ```

## Deployment

1. Plan the deployment:

   ```bash
   terraform plan
   ```

2. Apply the configuration:

   ```bash
   terraform apply
   ```

3. View the API endpoint:
   ```bash
   terraform output api_endpoint
   ```

## API Usage

### PUT /rsvp - Save RSVP Response

```bash
curl -X PUT https://<api-endpoint>/rsvp \
  -H "Content-Type: application/json" \
  -d '{
    "guest_id": "guest-123",
    "guest_email": "john@example.com",
    "status": "attending",
    "dietary_restrictions": "vegetarian",
    "plus_one": 1
  }'
```

**Request body:**

- `guest_id` (required): Unique guest identifier
- `guest_email` (required): Guest email address
- `status` (optional): attending, declined, or pending (default: pending)
- `dietary_restrictions` (optional): Dietary restrictions
- `plus_one` (optional): Number of additional guests (default: 0)

### GET /rsvp - Retrieve RSVP Response(s)

Get a specific RSVP:

```bash
curl https://<api-endpoint>/rsvp?guest_id=guest-123&guest_email=john@example.com
```

Get all RSVPs for a guest:

```bash
curl https://<api-endpoint>/rsvp?guest_id=guest-123
```

## DynamoDB Table Schema

| Attribute            | Type   | Key           | Description                                |
| -------------------- | ------ | ------------- | ------------------------------------------ |
| guest_id             | String | Partition Key | Unique guest identifier                    |
| guest_email          | String | Sort Key      | Guest email address                        |
| status               | String | -             | RSVP status (pending, attending, declined) |
| dietary_restrictions | String | -             | Dietary restrictions                       |
| plus_one             | Number | -             | Number of additional guests                |
| created_at           | String | -             | ISO timestamp of creation                  |
| ttl                  | Number | -             | TTL for automatic expiration               |

## Cleanup

To destroy all resources:

```bash
terraform destroy
```

## Environment Variables

Environment variables for customization:

- `AWS_REGION`: AWS region (default: us-east-1)
- `AWS_PROFILE`: AWS profile to use

## Troubleshooting

### Lambda function not found error

Ensure the Lambda source files exist at `../src/lambda_put_rsvp.js` and `../src/lambda_get_rsvp.js`.

### DynamoDB permission denied

Ensure the IAM role has the correct permissions to access DynamoDB.

### API Gateway CORS errors

CORS is enabled for all origins (\*). Adjust the `cors_configuration` in `api_gateway.tf` if needed.

## Files

- `main.tf`: Provider configuration
- `variables.tf`: Input variables
- `outputs.tf`: Output values
- `dynamodb.tf`: DynamoDB table configuration
- `lambda.tf`: Lambda functions
- `api_gateway.tf`: API Gateway configuration
- `iam.tf`: IAM roles and policies
- `terraform.tfvars.example`: Example variable values
- `.gitignore`: Git ignore rules for Terraform files
