use std::path::PathBuf;

use crate::commands::SavedPassword;
//use bytes::Bytes;
use local_ledger::{LedgerDump, LocalLedger};
use tokio::sync::{mpsc, oneshot};
//use tokio_stream::Stream;

pub enum LocalLedgerMessage {
    Start {
        ledger_name: String,
        master_pw: String,
        respond_to: oneshot::Sender<Result<(), LocalLedgerWorkerErr>>,
    },
    AddEntry {
        entry_name: String,
        pw: String,
        respond_to: oneshot::Sender<Result<(), LocalLedgerWorkerErr>>,
    },
    List {
        respond_to: oneshot::Sender<Result<Vec<String>, LocalLedgerWorkerErr>>,
    },
    GetEntry {
        entry_name: String,
        respond_to: oneshot::Sender<Result<String, LocalLedgerWorkerErr>>,
    },
    UpdateEntry {
        entry_name: String,
        pw: String,
        respond_to: oneshot::Sender<Result<(), LocalLedgerWorkerErr>>,
    },
    RemoveEntry {
        entry_name: String,
        respond_to: oneshot::Sender<Result<(), LocalLedgerWorkerErr>>,
    },
    GetLedgerDir {
        respond_to: oneshot::Sender<Result<PathBuf, LocalLedgerWorkerErr>>,
    },
    GetLedgerContent {
        respond_to: oneshot::Sender<Result<LedgerDump, LocalLedgerWorkerErr>>,
    },
    // Merge {
    //     byte_stream:
    //         Pin<Box<dyn Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send>>> + Send>>,
    //     respond_to: oneshot::Sender<Result<(), LocalLedgerWorkerErr>>,
    // },
}

#[derive(Debug, serde::Serialize)]
pub enum LocalLedgerWorkerErr {
    StartErr(String),
    ResponseErr(String),
    AddEntryErr(String),
    ListEntriesErr(String),
    GetEntryErr(String),
    GetLedgerContent(String),
}

impl ToString for LocalLedgerWorkerErr {
    fn to_string(&self) -> String {
        match self {
            LocalLedgerWorkerErr::StartErr(msg) => msg.to_owned(),
            LocalLedgerWorkerErr::ResponseErr(msg) => msg.to_owned(),
            LocalLedgerWorkerErr::AddEntryErr(msg) => msg.to_owned(),
            LocalLedgerWorkerErr::ListEntriesErr(msg) => msg.to_owned(),
            LocalLedgerWorkerErr::GetEntryErr(msg) => msg.to_owned(),
            LocalLedgerWorkerErr::GetLedgerContent(msg) => msg.to_owned(),
        }
    }
}

pub struct LocalLedgerWorker {
    receiver: mpsc::Receiver<LocalLedgerMessage>,
    local_ledger: Option<LocalLedger<SavedPassword>>,
}

impl LocalLedgerWorker {
    pub fn new(recvr: mpsc::Receiver<LocalLedgerMessage>) -> Self {
        Self {
            receiver: recvr,
            local_ledger: None,
        }
    }

    fn handle_msg(&mut self, msg: LocalLedgerMessage) {
        match msg {
            LocalLedgerMessage::Start {
                ledger_name,
                master_pw,
                respond_to,
            } => {
                let start_result = LocalLedger::<SavedPassword>::new(&ledger_name, master_pw)
                    .map(|ll| {
                        self.local_ledger = Some(ll);
                    })
                    .map_err(|err| LocalLedgerWorkerErr::StartErr(err.to_string()));

                let _ = respond_to.send(start_result).map_err(|_err| {
                    // TODO: retry
                    println!("Failed to start LocalLedgerWorker");
                });
            }

            LocalLedgerMessage::AddEntry {
                entry_name,
                pw,
                respond_to,
            } => {
                let add_entry_result = self
                    .local_ledger
                    .as_mut()
                    .map_or(
                        Err(LocalLedgerWorkerErr::AddEntryErr(
                            "LocalLedgerWorker has not been started.".to_string(),
                        )),
                        |ll| Ok(ll),
                    )
                    .and_then(|ll| {
                        let saved_password = SavedPassword {
                            pw,
                            name: entry_name.clone(),
                        };

                        let _ = ll
                            .create(saved_password, &entry_name)
                            .map_err(|err| LocalLedgerWorkerErr::AddEntryErr(err.to_string()))?;

                        Ok(())
                    });

                let _ = respond_to.send(add_entry_result).map_err(|_err| {
                    // TODO: retry
                    println!("Failed to add entry");
                });
            }

            LocalLedgerMessage::List { respond_to } => {
                let list_result = self
                    .local_ledger
                    .as_mut()
                    .map_or(
                        Err(LocalLedgerWorkerErr::ListEntriesErr(
                            "LocalLedgerWorker has not been started".to_string(),
                        )),
                        |ll| Ok(ll),
                    )
                    .and_then(|ll| {
                        let labels = ll
                            .list_entry_labels()
                            .map(|ls| {
                                let owned_ls: Vec<_> =
                                    ls.into_iter().map(|l| l.to_owned()).collect();

                                owned_ls
                            })
                            .map_err(|err| LocalLedgerWorkerErr::ListEntriesErr(err.to_string()));

                        labels
                    });

                let _ = respond_to.send(list_result).map_err(|_err| {
                    // TODO: retry
                    println!("Failed to list entries");
                });
            }

            LocalLedgerMessage::GetEntry {
                entry_name,
                respond_to,
            } => {
                let pw_result = self
                    .local_ledger
                    .as_mut()
                    .map_or(
                        Err(LocalLedgerWorkerErr::GetEntryErr(
                            "LocalLedgerWorker has not been started".to_string(),
                        )),
                        |ll| Ok(ll),
                    )
                    .and_then(|ll| {
                        let pw = ll
                            .read_by_entry_name(&entry_name)
                            .map(|saved_password| saved_password.pw.to_string())
                            .map_err(|err| LocalLedgerWorkerErr::GetEntryErr(err.to_string()));

                        pw
                    });

                let _ = respond_to.send(pw_result).map_err(|_err| {
                    // TODO: retry
                    println!("Failed to get entry");
                });
            }

            LocalLedgerMessage::UpdateEntry {
                entry_name,
                pw,
                respond_to,
            } => {
                let pw_result = self
                    .local_ledger
                    .as_mut()
                    .ok_or(LocalLedgerWorkerErr::GetEntryErr(
                        "LocalLedgerWorker has not been started".to_string(),
                    ))
                    .and_then(|ll| {
                        let saved_password = SavedPassword {
                            pw,
                            name: entry_name.clone(),
                        };

                        let pw = ll
                            .update(&entry_name, saved_password)
                            .map_err(|err| LocalLedgerWorkerErr::GetEntryErr(err.to_string()));

                        pw
                    });

                let _ = respond_to.send(pw_result).map_err(|_err| {
                    // TODO: retry
                    println!("Failed to get entry");
                });
            }

            LocalLedgerMessage::RemoveEntry {
                entry_name,
                respond_to,
            } => {
                let pw_result = self
                    .local_ledger
                    .as_mut()
                    .map_or(
                        Err(LocalLedgerWorkerErr::GetEntryErr(
                            "LocalLedgerWorker has not been started".to_string(),
                        )),
                        |ll| Ok(ll),
                    )
                    .and_then(|ll| {
                        ll.remove(&entry_name)
                            .map_err(|err| LocalLedgerWorkerErr::GetEntryErr(err.to_string()))
                    });

                let _ = respond_to.send(pw_result).map_err(|_err| {
                    // TODO: retry
                    println!("Failed to get entry");
                });
            }

            LocalLedgerMessage::GetLedgerDir { respond_to } => {
                let dir_result = self
                    .local_ledger
                    .as_mut()
                    .map_or(
                        Err(LocalLedgerWorkerErr::GetEntryErr(
                            "LocalLedgerWorker has not been started".to_string(),
                        )),
                        |ll| Ok(ll),
                    )
                    .and_then(|ll| {
                        ll.get_ledger_dir()
                            .map_err(|err| LocalLedgerWorkerErr::GetEntryErr(err.to_string()))
                    });

                let _ = respond_to.send(dir_result).map_err(|_err| {
                    // TODO: retry
                    println!("Failed to get entry");
                });
            }

            LocalLedgerMessage::GetLedgerContent { respond_to } => {
                let ledger_dump = self
                    .local_ledger
                    .as_mut()
                    .ok_or(LocalLedgerWorkerErr::GetLedgerContent(
                        "LocalLedgerWorker has not been started".to_string(),
                    ))
                    .and_then(|ll| {
                        ll.doc_dump()
                            .map_err(|e| LocalLedgerWorkerErr::GetLedgerContent(e.to_string()))
                    });

                let _ = respond_to.send(ledger_dump).map_err(|_| {
                    println!("Failed to get ledger dump");
                });
                //unimplemented!()
            } // LocalLedgerMessage::Merge { respond_to } => {
              //     let ledger_dump = self
              //         .local_ledger
              //         .as_mut()
              //         .ok_or(LocalLedgerWorkerErr::GetLedgerContent(
              //             "LocalLedgerWorker has not been started".to_string(),
              //         ))
              //         .and_then(|ll| {
              //             ll.merge()
              //                 .await
              //                 .map_err(|e| LocalLedgerWorkerErr::GetLedgerContent(e.to_string()))
              //         });

              //     let _ = respond_to.send(ledger_dump).map_err(|_| {
              //         println!("Failed to get ledger dump");
              //     });
              // }
        }
    }
}

pub async fn run_llw(mut llw: LocalLedgerWorker) {
    while let Some(msg) = llw.receiver.recv().await {
        llw.handle_msg(msg);
    }
}
