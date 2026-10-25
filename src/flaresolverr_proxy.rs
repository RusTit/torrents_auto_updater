use std::error::Error;

use serde_json::{Value, json};
use tracing::debug;

pub struct FlareSolverrProxy {
    url: String,
    proxy_url: Option<String>,
}

impl FlareSolverrProxy {
    pub fn new(url: String, proxy_url: Option<String>) -> Self {
        Self { url, proxy_url }
    }

    pub async fn destroy_session(
        &self,
        session_id: &String,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let body = json!({
            "cmd": "sessions.destroy",
            "session": session_id
        });

        client
            .post(&self.url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn create_session(&self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let body = json!({
            "cmd": "sessions.create",
            "proxy": &self.proxy_url,
        });

        let response = client
            .post(&self.url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let body: Value = response.json().await?;

        debug!("{}", body);
        let session = body["session"]
            .as_str()
            .ok_or_else(|| std::io::Error::other("Session not found"))?;

        Ok(session.to_string())
    }

    pub async fn get_or_create_session_id(&self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let existing_session_id = self.get_existing_sessions().await?.into_iter().next();
        if let Some(value) = existing_session_id {
            return Ok(value);
        }
        let new_session_id = self.create_session().await?;

        Ok(new_session_id)
    }

    pub async fn get_existing_sessions(&self) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let body = json!({
            "cmd": "sessions.list"
        });

        let response = client
            .post(&self.url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let body: Value = response.json().await?;

        let sessions: Vec<String> = body["sessions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str().map(String::from))
            .collect();

        debug!("FlareSolverr sessions: {:?}", sessions);

        Ok(sessions)
    }
}
