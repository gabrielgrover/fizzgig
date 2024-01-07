use rand::Rng;
use std::{error::Error, fmt};
use uuid::Uuid;

pub fn generate_id() -> String {
    let mut buf = Uuid::encode_buffer();
    let id = Uuid::new_v4().to_simple().encode_upper(&mut buf);

    String::from(id)
}

pub fn generate_pin() -> String {
    let mut rng = rand::thread_rng();
    let number: u32 = rng.gen_range(100_000..=999_999);
    let pin = number.to_string();

    pin
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum LocalLedgerErrorType {
    Default,
    Confict,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LocalLedgerError {
    pub message: String,
    pub err_type: LocalLedgerErrorType,
}

impl LocalLedgerError {
    pub fn new(m: &str) -> Self {
        LocalLedgerError {
            message: m.to_string(),
            err_type: LocalLedgerErrorType::Default,
        }
    }

    pub fn conflict(m: &str) -> Self {
        LocalLedgerError {
            message: m.to_string(),
            err_type: LocalLedgerErrorType::Confict,
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
