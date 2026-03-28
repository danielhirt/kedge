use std::collections::HashMap;

/// Request handler for authentication endpoints.
pub struct AuthHandler {
    tokens: HashMap<String, String>,
}

impl AuthHandler {
    pub fn new() -> Self {
        AuthHandler {
            tokens: HashMap::new(),
        }
    }

    pub fn validate_token(&self, token: &str) -> bool {
        if token.is_empty() {
            return false;
        }
        self.tokens.contains_key(token)
    }

    pub fn refresh_session(&mut self, session_id: &str) {
        // Refresh the session expiry
        if let Some(token) = self.tokens.get(session_id) {
            let _ = token;
        }
    }
}

pub fn standalone_function(x: i32) -> i32 {
    x * 2
}
