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
    NotFound(String),
}

impl std::fmt::Display for FastmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FastmailError::Http(e) => write!(f, "HTTP error: {}", e),
            FastmailError::Auth(status, body) => write!(f, "Auth failed ({}): {}", status, body),
            FastmailError::Api(e) => write!(f, "API error: {}", e),
            FastmailError::Parse(e) => write!(f, "Parse error: {}", e),
            FastmailError::MissingCapability => write!(f, "Masked email capability not found"),
            FastmailError::NotFound(e) => write!(f, "Not found: {}", e),
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MaskedEmail {
    pub id: Option<String>,
    pub email: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(rename = "forDomain", default)]
    pub for_domain: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<String>,
    #[serde(rename = "lastMessageAt", default)]
    pub last_message_at: Option<String>,
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
        for_domain: Option<&str>,
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
                            "forDomain": for_domain.unwrap_or_default()
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

    pub fn list_masked_emails(&self, account_id: &str) -> Result<Vec<MaskedEmail>, FastmailError> {
        let request = JmapRequest {
            using: vec![JMAP_CORE_CAPABILITY.to_string(), MASKED_EMAIL_CAPABILITY.to_string()],
            method_calls: vec![(
                "MaskedEmail/get".to_string(),
                serde_json::json!({
                    "accountId": account_id,
                    "ids": null
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
            if method == "MaskedEmail/get" {
                if let Some(list) = result.get("list") {
                    return serde_json::from_value(list.clone())
                        .map_err(|e| FastmailError::Parse(e.to_string()));
                }
            }
        }

        Err(FastmailError::Api(format!(
            "Unexpected response: {:?}",
            jmap
        )))
    }

    pub fn delete_masked_email(&self, account_id: &str, id: &str) -> Result<(), FastmailError> {
        let request = JmapRequest {
            using: vec![JMAP_CORE_CAPABILITY.to_string(), MASKED_EMAIL_CAPABILITY.to_string()],
            method_calls: vec![(
                "MaskedEmail/set".to_string(),
                serde_json::json!({
                    "accountId": account_id,
                    "update": {
                        id: {
                            "state": "disabled"
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
                if result.get("updated").and_then(|u| u.get(id)).is_some() {
                    return Ok(());
                }
                if let Some(not_updated) = result.get("notUpdated") {
                    return Err(FastmailError::Api(format!("{:?}", not_updated)));
                }
            }
        }

        Err(FastmailError::Api(format!(
            "Unexpected response: {:?}",
            jmap
        )))
    }

    pub fn destroy_masked_email(&self, account_id: &str, id: &str) -> Result<(), FastmailError> {
        let request = JmapRequest {
            using: vec![JMAP_CORE_CAPABILITY.to_string(), MASKED_EMAIL_CAPABILITY.to_string()],
            method_calls: vec![(
                "MaskedEmail/set".to_string(),
                serde_json::json!({
                    "accountId": account_id,
                    "update": {
                        id: {
                            "state": "deleted"
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
                if result.get("updated").and_then(|u| u.get(id)).is_some() {
                    return Ok(());
                }
                if let Some(not_updated) = result.get("notUpdated") {
                    return Err(FastmailError::Api(format!("{:?}", not_updated)));
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
        let result = client.create_masked_email(&account_id, Some("test from tmail"), None);
        println!("Create masked email result: {:#?}", result);
        assert!(result.is_ok());

        // Cleanup
        let created = result.unwrap();
        let id = created.id.expect("Created email has no ID");
        client.destroy_masked_email(&account_id, &id).expect("Failed to cleanup");
    }

    #[test]
    #[ignore]
    fn test_list_masked_emails() {
        let client = FastmailClient::new(get_test_token());
        let account_id = client.get_account_id().expect("Failed to get account ID");
        let result = client.list_masked_emails(&account_id);
        println!("List masked emails result: {:#?}", result);
        assert!(result.is_ok());
        let emails = result.unwrap();
        assert!(!emails.is_empty());
    }

    #[test]
    #[ignore]
    fn test_delete_masked_email() {
        let client = FastmailClient::new(get_test_token());
        let account_id = client.get_account_id().expect("Failed to get account ID");

        // Create a test email first
        let created = client
            .create_masked_email(&account_id, Some("test delete"))
            .expect("Failed to create test email");
        println!("Created test email: {:#?}", created);

        let id = created.id.expect("Created email has no ID");

        // Archive it
        let result = client.delete_masked_email(&account_id, &id);
        println!("Delete result: {:#?}", result);
        assert!(result.is_ok());

        // Verify it's now disabled
        let emails = client.list_masked_emails(&account_id).expect("Failed to list");
        let archived = emails.iter().find(|e| e.id.as_deref() == Some(&id));
        assert!(archived.is_some());
        assert_eq!(archived.unwrap().state.as_deref(), Some("disabled"));

        // Cleanup
        client.destroy_masked_email(&account_id, &id).expect("Failed to cleanup");
    }
}
