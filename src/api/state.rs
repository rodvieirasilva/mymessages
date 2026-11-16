use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::index::Database;
use crate::storage::MaildirStorage;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub storage: Arc<MaildirStorage>,
    pub data_dir: PathBuf,
    pub config: AppConfig,
}

impl AppState {
    pub fn new(db: Database, data_dir: PathBuf, config: AppConfig) -> Self {
        let storage = MaildirStorage::new(&data_dir);
        Self {
            db: Arc::new(Mutex::new(db)),
            storage: Arc::new(storage),
            data_dir,
            config,
        }
    }
}
