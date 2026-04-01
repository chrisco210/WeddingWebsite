use aws_sdk_dynamodb::types::AttributeValue;

use crate::handler::{GuestRsvp, HandlerError};

// ── Backing store trait ───────────────────────────────────────────────────────

/// Abstracts persistence so that the handler can be tested without a real
/// DynamoDB table.
pub trait RsvpStore {
    /// Load previously submitted RSVP responses for a party, if any.
    async fn load(&self, party_id: &str) -> Result<Vec<GuestRsvp>, HandlerError>;

    /// Persist RSVP responses for a party, overwriting any previous submission.
    async fn save(&self, party_id: &str, responses: &[GuestRsvp]) -> Result<(), HandlerError>;
}

// ── DynamoDB store (production) ───────────────────────────────────────────────

#[derive(Clone)]
pub struct DynamoRsvpStore {
    pub client: aws_sdk_dynamodb::Client,
    pub table_name: String,
}

impl RsvpStore for DynamoRsvpStore {
    async fn load(&self, party_id: &str) -> Result<Vec<GuestRsvp>, HandlerError> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("party_id", AttributeValue::S(party_id.to_string()))
            .send()
            .await
            .map_err(|e| HandlerError::Internal(e.to_string()))?;

        let Some(item) = result.item else {
            return Ok(vec![]);
        };
        let Some(AttributeValue::S(data)) = item.get("rsvp_data") else {
            return Ok(vec![]);
        };

        serde_json::from_str(data).map_err(|e| HandlerError::Internal(e.to_string()))
    }

    async fn save(&self, party_id: &str, responses: &[GuestRsvp]) -> Result<(), HandlerError> {
        let rsvp_json = serde_json::to_string(responses)
            .map_err(|e| HandlerError::Internal(e.to_string()))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.client
            .put_item()
            .table_name(&self.table_name)
            .item("party_id", AttributeValue::S(party_id.to_string()))
            .item("rsvp_data", AttributeValue::S(rsvp_json))
            .item("submitted_at", AttributeValue::N(now.to_string()))
            .send()
            .await
            .map_err(|e| HandlerError::Internal(e.to_string()))?;

        Ok(())
    }
}
