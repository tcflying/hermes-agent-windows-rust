use anyhow::Result;

pub struct GatewaySession;

impl GatewaySession {
    pub fn new() -> Self {
        Self
    }

    pub fn create(&self, _user_id: &str) -> Result<String> {
        Ok("session_id".to_string())
    }

    pub fn get_messages(&self, _session_id: &str) -> Result<Vec<GatewayMessage>> {
        Ok(vec![])
    }
}

pub struct GatewayMessage {
    pub role: String,
    pub content: String,
}

impl Default for GatewaySession {
    fn default() -> Self {
        Self::new()
    }
}
