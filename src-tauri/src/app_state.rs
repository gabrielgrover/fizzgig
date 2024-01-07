use land_strider_sdk::LandStrider;
use tokio::sync::Mutex;

use crate::password_ledger_handler::PasswordLedgerHandler;

pub struct AppState {
    pub land_strider: LandStrider,
    pub pw_ledger: Mutex<PasswordLedgerHandler>,
}
