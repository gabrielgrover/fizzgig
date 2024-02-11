use std::path::PathBuf;

use crate::commands::SavedPassword;
use local_ledger::{LedgerDump, LocalLedger};
use serde_json::Value;
use tokio_stream::Stream;

const PASSWORD_LEDGER_NAME: &str = "Password_Ledger";

#[derive(Debug)]
pub struct PasswordLedgerHandler {
    ledger: Option<LocalLedger<SavedPassword>>,
}

impl PasswordLedgerHandler {
    pub fn new() -> Self {
        Self { ledger: None }
    }

    pub fn start(&mut self, master_pw: &str) -> Result<(), String> {
        if self.ledger.is_some() {
            return Ok(());
        }

        let password_ledger =
            LocalLedger::<SavedPassword>::new(PASSWORD_LEDGER_NAME, master_pw.to_string())
                .map_err(|e| e.to_string())?;

        self.ledger = Some(password_ledger);

        Ok(())
    }

    pub fn add_entry(&mut self, entry_name: &str, pw: &str) -> Result<(), String> {
        let password_ledger = self
            .ledger
            .as_mut()
            .ok_or("Ledger has not been started".to_string())?;

        let saved_password = SavedPassword {
            name: entry_name.to_string(),
            pw: pw.to_string(),
        };

        let _ = password_ledger
            .create(saved_password, entry_name)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn update_entry(&mut self, entry_name: &str, pw: &str) -> Result<(), String> {
        let password_ledger = self
            .ledger
            .as_mut()
            .ok_or("Ledger has not been started".to_string())?;

        let saved_password = SavedPassword {
            name: entry_name.to_string(),
            pw: pw.to_string(),
        };

        password_ledger
            .update(entry_name, saved_password)
            .map_err(|e| e.to_string())
    }

    pub fn remove_entry(&mut self, entry_name: &str) -> Result<(), String> {
        let password_ledger = self
            .ledger
            .as_mut()
            .ok_or("Ledger has not been started".to_string())?;

        password_ledger
            .remove(entry_name)
            .map_err(|e| e.to_string())
    }

    pub fn list_entries(&self) -> Result<Vec<String>, String> {
        let password_ledger = self
            .ledger
            .as_ref()
            .ok_or("Ledger has not been started".to_string())?;

        password_ledger
            .list_entry_labels()
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|entry_name| entry_name.to_string())
                    .collect()
            })
            .map_err(|e| e.to_string())
    }

    pub fn get_pw(&mut self, entry_name: &str) -> Result<String, String> {
        let password_ledger = self
            .ledger
            .as_mut()
            .ok_or("Ledger has not been started".to_string())?;

        tracing::info!("entry_name: {}", entry_name);

        password_ledger
            .read_by_entry_name(entry_name)
            .map(|saved_pw| saved_pw.pw.clone())
            .map_err(|e| e.to_string())
    }

    pub fn get_ledger_dir(&self) -> Result<PathBuf, String> {
        let password_ledger = self
            .ledger
            .as_ref()
            .ok_or("Ledger has not been started".to_string())?;

        password_ledger.get_ledger_dir().map_err(|e| e.to_string())
    }

    pub fn get_doc_dump(&self) -> Result<LedgerDump, String> {
        let password_ledger = self
            .ledger
            .as_ref()
            .ok_or("Ledger has not been started".to_string())?;

        password_ledger.doc_dump().map_err(|e| e.to_string())
    }

    pub async fn merge<S>(&mut self, s: S) -> Result<(), String>
    where
        S: Stream<Item = Result<Value, Box<dyn std::error::Error>>> + Unpin + Send,
    {
        let password_ledger = self
            .ledger
            .as_mut()
            .ok_or("Ledger has not been started".to_string())?;

        password_ledger.merge(s).await.map_err(|e| e.to_string())?;

        Ok(())
    }
}
