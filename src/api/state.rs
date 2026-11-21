use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::index::Database;
use crate::storage::{MaildirStorage, StoragePipeline};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub storage: Arc<MaildirStorage>,
    pub pipeline: Arc<StoragePipeline>,
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub config: AppConfig,
}

impl AppState {
    pub fn new(db: Database, data_dir: PathBuf, config: AppConfig) -> Self {
        let storage = Arc::new(MaildirStorage::new(&data_dir));
        let db_arc = Arc::new(Mutex::new(db));
        let pipeline = Arc::new(StoragePipeline::from_config(&config, db_arc.clone(), storage.clone()));
        let db_path = config
            .get_storage_path("sqlite")
            .unwrap_or_else(|| data_dir.join("messages.sqlite3"));

        Self {
            db: db_arc,
            storage,
            pipeline,
            data_dir,
            db_path,
            config,
        }
    }
}
