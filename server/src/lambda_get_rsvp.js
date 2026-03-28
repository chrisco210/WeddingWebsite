const AWS = require("aws-sdk");
const dynamodb = new AWS.DynamoDB.DocumentClient();

const TABLE_NAME = process.env.TABLE_NAME;

exports.handler = async (event) => {
  console.log("Received event:", JSON.stringify(event, null, 2));

  try {
    const queryParams = event.queryStringParameters || {};
    const { guest_id, guest_email } = queryParams;

    // If both guest_id and guest_email are provided, get specific RSVP
    if (guest_id && guest_email) {
      const params = {
        TableName: TABLE_NAME,
        Key: {
          guest_id,
          guest_email,
        },
      };

      const result = await dynamodb.get(params).promise();

      if (!result.Item) {
        return {
          statusCode: 404,
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            error: "RSVP not found",
          }),
        };
      }

      return {
        statusCode: 200,
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          data: result.Item,
        }),
      };
    }

    // If only guest_id is provided, query all RSVPs for that guest
    if (guest_id) {
      const params = {
        TableName: TABLE_NAME,
        KeyConditionExpression: "guest_id = :guest_id",
        ExpressionAttributeValues: {
          ":guest_id": guest_id,
        },
      };

      const result = await dynamodb.query(params).promise();

      return {
        statusCode: 200,
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          count: result.Items.length,
          data: result.Items,
        }),
      };
    }

    // If no parameters, return error
    return {
      statusCode: 400,
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        error:
          "Missing required query parameters: guest_id and guest_email OR guest_id",
      }),
    };
  } catch (error) {
    console.error("Error:", error);
    return {
      statusCode: 500,
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        error: "Internal server error",
        message: error.message,
      }),
    };
  }
};
