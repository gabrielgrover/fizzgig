use std::{error::Error, fmt};
use uuid::Uuid;

pub fn generate_id() -> String {
    let mut buf = Uuid::encode_buffer();
    let id = Uuid::new_v4().to_simple().encode_upper(&mut buf);

    String::from(id)
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LocalLedgerError {
    pub message: String,
}

impl LocalLedgerError {
    pub fn new(m: &str) -> Self {
        LocalLedgerError {
            message: m.to_string(),
        }
    }
}

impl fmt::Display for LocalLedgerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for LocalLedgerError {
    fn description(&self) -> &str {
        &self.message
    }
}
