use std::path::PathBuf;

use local_ledger::LedgerDump;
use tokio::sync::{mpsc, oneshot};

use crate::local_ledger_worker::{
    run_llw, LocalLedgerMessage, LocalLedgerWorker, LocalLedgerWorkerErr,
};

pub struct LocalLedgerWorkerHandler {
    messenger: mpsc::Sender<LocalLedgerMessage>,
}

impl LocalLedgerWorkerHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(10);
        let llw = LocalLedgerWorker::new(rx);

        tokio::spawn(run_llw(llw));

        Self { messenger: tx }
    }

    pub async fn start_worker(
        &self,
        ledger_name: &str,
        master_pw: &str,
    ) -> Result<(), LocalLedgerWorkerErr> {
        let (tx, rx) = oneshot::channel();
        let msg = LocalLedgerMessage::Start {
            ledger_name: ledger_name.to_owned(),
            master_pw: master_pw.to_owned(),
            respond_to: tx,
        };
        let _ = self.messenger.send(msg).await;

        rx.await
            .map_err(|err| LocalLedgerWorkerErr::ResponseErr(err.to_string()))?
    }

    pub async fn add_entry(&self, entry_name: &str, pw: &str) -> Result<(), LocalLedgerWorkerErr> {
        let (respond_to, rx) = oneshot::channel();
        let msg = LocalLedgerMessage::AddEntry {
            entry_name: entry_name.to_owned(),
            pw: pw.to_owned(),
            respond_to,
        };
        let _ = self.messenger.send(msg).await;

        rx.await
            .map_err(|err| LocalLedgerWorkerErr::ResponseErr(err.to_string()))?
    }

    pub async fn update_entry(
        &self,
        entry_name: &str,
        pw: &str,
    ) -> Result<(), LocalLedgerWorkerErr> {
        let (respond_to, rx) = oneshot::channel();
        let msg = LocalLedgerMessage::UpdateEntry {
            entry_name: entry_name.to_owned(),
            pw: pw.to_owned(),
            respond_to,
        };
        let _ = self.messenger.send(msg).await;

        rx.await
            .map_err(|err| LocalLedgerWorkerErr::ResponseErr(err.to_string()))?
    }

    pub async fn remove_entry(&self, entry_name: &str) -> Result<(), LocalLedgerWorkerErr> {
        let (respond_to, rx) = oneshot::channel();
        let msg = LocalLedgerMessage::RemoveEntry {
            entry_name: entry_name.to_owned(),
            respond_to,
        };

        let _ = self.messenger.send(msg).await;

        rx.await
            .map_err(|err| LocalLedgerWorkerErr::ResponseErr(err.to_string()))?
    }

    pub async fn list_entries(&self) -> Result<Vec<String>, LocalLedgerWorkerErr> {
        let (respond_to, rx) = oneshot::channel();
        let msg = LocalLedgerMessage::List { respond_to };
        let _ = self.messenger.send(msg).await;

        rx.await
            .map_err(|err| LocalLedgerWorkerErr::ListEntriesErr(err.to_string()))?
    }

    pub async fn get_pw(&self, entry_name: &str) -> Result<String, LocalLedgerWorkerErr> {
        let (respond_to, rx) = oneshot::channel();
        let msg = LocalLedgerMessage::GetEntry {
            entry_name: entry_name.to_owned(),
            respond_to,
        };
        let _ = self.messenger.send(msg).await;

        rx.await
            .map_err(|err| LocalLedgerWorkerErr::GetEntryErr(err.to_string()))?
    }

    pub async fn get_ledger_dir(&self) -> Result<PathBuf, LocalLedgerWorkerErr> {
        let (respond_to, rx) = oneshot::channel();
        let msg = LocalLedgerMessage::GetLedgerDir { respond_to };
        let _ = self.messenger.send(msg).await;

        rx.await
            .map_err(|err| LocalLedgerWorkerErr::ResponseErr(err.to_string()))?
    }

    pub async fn get_doc_dump(&self) -> Result<LedgerDump, LocalLedgerWorkerErr> {
        let (respond_to, rx) = oneshot::channel();
        let msg = LocalLedgerMessage::GetLedgerContent { respond_to };
        let _ = self.messenger.send(msg).await;

        rx.await
            .map_err(|e| LocalLedgerWorkerErr::GetLedgerContent(e.to_string()))?
    }
}
