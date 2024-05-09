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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct EntryMetaData {
    pub label: String,
    pub has_conflict: bool,
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

    pub fn list_entry_meta_data(&self) -> Result<Vec<EntryMetaData>, String> {
        let password_ledger = self
            .ledger
            .as_ref()
            .ok_or("Ledger has not been started".to_string())?;
        let entries_with_conflict: Vec<_> = password_ledger
            .list_entries_with_conflicts()
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|label| EntryMetaData {
                        label,
                        has_conflict: true,
                    })
                    .collect()
            })
            .map_err(|e| e.to_string())?;
        let entries: Vec<_> = password_ledger
            .list_entry_labels()
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|label| EntryMetaData {
                        label,
                        has_conflict: false,
                    })
                    .collect()
            })
            .map_err(|e| e.to_string())?;

        Ok(entries
            .into_iter()
            .chain(entries_with_conflict.into_iter())
            .collect())
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

    /// Get conf tuple (original_password, remote_password)
    pub fn get_conf_pair(&mut self, entry_name: &str) -> Result<(String, String), String> {
        let password_ledger = self
            .ledger
            .as_mut()
            .ok_or("Ledger has not been started".to_string())?;
        let conf_doc = password_ledger
            .get_conf(entry_name)
            .map_err(|e| e.to_string())?;
        let conf_data = conf_doc.read_data().map_err(|e| e.to_string())?;
        let original_doc = password_ledger
            .read_by_entry_name(entry_name)
            .map_err(|e| e.to_string())?;
        let pair = (original_doc.pw.clone(), conf_data.pw.to_string());

        Ok(pair)
    }

    pub fn resolve(&mut self, entry_name: &str, keep_original: bool) -> Result<(), String> {
        let password_ledger = self
            .ledger
            .as_mut()
            .ok_or("Ledger has not been started".to_string())?;

        password_ledger
            .resolve(entry_name, keep_original)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LandStrider, LandStriderConfig};
    use land_strider::startup::land_strider_app;
    use utility::generate_id;

    #[tokio::test]
    async fn should_be_able_to_sync_with_server() {
        let test_server_config = axum_test::TestServerConfig::builder()
            .transport(axum_test::Transport::HttpRandomPort)
            .build();
        let server =
            axum_test::TestServer::new_with_config(land_strider_app(), test_server_config).unwrap();
        let addr_url = server.server_address().unwrap();
        let host = addr_url.host().unwrap().to_string();
        let port = addr_url.port().unwrap();
        let ls_config = LandStriderConfig::new(&host, port as u32);
        let land_strider = LandStrider::new(ls_config);

        let master_pw = "password".to_string();
        let mut pl = PasswordLedgerHandler::new();
        let entry_name = generate_id();

        pl.start(&master_pw).unwrap();
        pl.add_entry(&entry_name, "test1234").unwrap();
        let dump = pl.get_doc_dump().unwrap();

        let temp_pw = "1234".to_string();
        let pin = land_strider
            .push_s(dump, temp_pw.clone())
            .await
            .unwrap()
            .pin;
        let ps = land_strider.pull_s(&pin, &temp_pw).await.unwrap();

        pl.merge(ps).await.unwrap();
    }
}
