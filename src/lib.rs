use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const FASTMAIL_SESSION_URL: &str = "https://api.fastmail.com/jmap/session";
const FASTMAIL_API_URL: &str = "https://api.fastmail.com/jmap/api/";
const JMAP_CORE_CAPABILITY: &str = "urn:ietf:params:jmap:core";
const MASKED_EMAIL_CAPABILITY: &str = "https://www.fastmail.com/dev/maskedemail";

#[derive(Debug)]
pub enum FastmailError {
    Http(String),
    Auth(u16, String),
    Api(String),
    Parse(String),
    MissingCapability,
}

impl std::fmt::Display for FastmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FastmailError::Http(e) => write!(f, "HTTP error: {}", e),
            FastmailError::Auth(status, body) => write!(f, "Auth failed ({}): {}", status, body),
            FastmailError::Api(e) => write!(f, "API error: {}", e),
            FastmailError::Parse(e) => write!(f, "Parse error: {}", e),
            FastmailError::MissingCapability => write!(f, "Masked email capability not found"),
        }
    }
}

impl std::error::Error for FastmailError {}

#[derive(Deserialize, Debug)]
pub struct SessionResponse {
    #[serde(rename = "primaryAccounts")]
    pub primary_accounts: HashMap<String, String>,
}

#[derive(Serialize)]
struct JmapRequest {
    using: Vec<String>,
    #[serde(rename = "methodCalls")]
    method_calls: Vec<(String, serde_json::Value, String)>,
}

#[derive(Deserialize, Debug)]
pub struct JmapResponse {
    #[serde(rename = "methodResponses")]
    pub method_responses: Vec<(String, serde_json::Value, String)>,
}

#[derive(Deserialize, Debug)]
pub struct MaskedEmail {
    pub id: Option<String>,
    pub email: String,
}

pub struct FastmailClient {
    http: reqwest::blocking::Client,
    token: String,
}

impl FastmailClient {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            http: reqwest::blocking::Client::new(),
            token: token.into(),
        }
    }

    pub fn get_session(&self) -> Result<SessionResponse, FastmailError> {
        let response = self
            .http
            .get(FASTMAIL_SESSION_URL)
            .bearer_auth(&self.token)
            .send()
            .map_err(|e| FastmailError::Http(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(FastmailError::Auth(status.as_u16(), body));
        }

        response
            .json()
            .map_err(|e| FastmailError::Parse(e.to_string()))
    }

    pub fn get_account_id(&self) -> Result<String, FastmailError> {
        let session = self.get_session()?;
        session
            .primary_accounts
            .get(MASKED_EMAIL_CAPABILITY)
            .cloned()
            .ok_or(FastmailError::MissingCapability)
    }

    pub fn create_masked_email(
        &self,
        account_id: &str,
        description: Option<&str>,
    ) -> Result<MaskedEmail, FastmailError> {
        let request = JmapRequest {
            using: vec![JMAP_CORE_CAPABILITY.to_string(), MASKED_EMAIL_CAPABILITY.to_string()],
            method_calls: vec![(
                "MaskedEmail/set".to_string(),
                serde_json::json!({
                    "accountId": account_id,
                    "create": {
                        "new": {
                            "state": "enabled",
                            "description": description.unwrap_or_default(),
                            "forDomain": ""
                        }
                    }
                }),
                "0".to_string(),
            )],
        };

        let response = self
            .http
            .post(FASTMAIL_API_URL)
            .bearer_auth(&self.token)
            .json(&request)
            .send()
            .map_err(|e| FastmailError::Http(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(FastmailError::Auth(status.as_u16(), body));
        }

        let jmap: JmapResponse = response
            .json()
            .map_err(|e| FastmailError::Parse(e.to_string()))?;

        if let Some((method, result, _)) = jmap.method_responses.first() {
            if method == "MaskedEmail/set" {
                if let Some(created) = result.get("created") {
                    if let Some(new_email) = created.get("new") {
                        return serde_json::from_value(new_email.clone())
                            .map_err(|e| FastmailError::Parse(e.to_string()));
                    }
                }
                if let Some(not_created) = result.get("notCreated") {
                    return Err(FastmailError::Api(format!("{:?}", not_created)));
                }
            }
        }

        Err(FastmailError::Api(format!(
            "Unexpected response: {:?}",
            jmap
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_token() -> String {
        std::env::var("FASTMAIL_TOKEN").expect("FASTMAIL_TOKEN env var required for tests")
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_get_session() {
        let client = FastmailClient::new(get_test_token());
        let result = client.get_session();
        println!("Session result: {:#?}", result);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore]
    fn test_get_account_id() {
        let client = FastmailClient::new(get_test_token());
        let result = client.get_account_id();
        println!("Account ID result: {:#?}", result);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore]
    fn test_create_masked_email() {
        let client = FastmailClient::new(get_test_token());
        let account_id = client.get_account_id().expect("Failed to get account ID");
        let result = client.create_masked_email(&account_id, Some("test from tmail"));
        println!("Create masked email result: {:#?}", result);
        assert!(result.is_ok());
    }
}
