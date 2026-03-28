const AWS = require("aws-sdk");
const dynamodb = new AWS.DynamoDB.DocumentClient();

const TABLE_NAME = process.env.TABLE_NAME;

exports.handler = async (event) => {
  console.log("Received event:", JSON.stringify(event, null, 2));

  try {
    const body =
      typeof event.body === "string" ? JSON.parse(event.body) : event.body;

    const { guest_id, guest_email, status, dietary_restrictions, plus_one } =
      body;

    // Validate required fields
    if (!guest_id || !guest_email) {
      return {
        statusCode: 400,
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          error: "Missing required fields: guest_id and guest_email",
        }),
      };
    }

    // Prepare item for DynamoDB
    const item = {
      guest_id,
      guest_email,
      status: status || "pending",
      dietary_restrictions: dietary_restrictions || "",
      plus_one: plus_one || 0,
      created_at: new Date().toISOString(),
    };

    // Put item in DynamoDB
    await dynamodb
      .put({
        TableName: TABLE_NAME,
        Item: item,
      })
      .promise();

    return {
      statusCode: 200,
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        message: "RSVP saved successfully",
        data: item,
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
