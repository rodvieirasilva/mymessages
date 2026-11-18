use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::config::AppConfig;
use crate::index::Database;
use crate::index::reindex::{reindex_from_folder, reindex_from_maildir};
use crate::storage::{MaildirStorage, StoragePipeline};

#[test]
fn test_reindex_from_folder_and_maildir() {
    let temp_dir = std::env::temp_dir().join(format!("test_reindex_pipe_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let db = Arc::new(Mutex::new(Database::in_memory().unwrap()));
    let storage = Arc::new(MaildirStorage::new(&temp_dir));
    let mut cfg = AppConfig::default();
    cfg.storages = vec![
        crate::config::StorageEntry::new_maildir("maildir", &temp_dir),
        crate::config::StorageEntry::new_sqlite("sqlite", ":memory:"),
    ];
    let pipeline = StoragePipeline::from_config(&cfg, db.clone(), storage);

    // 1. Ingest from test_messages folder via pipeline
    let report = reindex_from_folder(
        &pipeline,
        Path::new("./tests/test_messages"),
    ).unwrap();

    assert!(report.total_found >= 4);
    assert_eq!(report.newly_indexed, report.total_found);

    // 2. Clear database to simulate complete DB loss and restore from Maildir
    let mut fresh_db = Database::in_memory().unwrap();
    let restore_report = reindex_from_maildir(fresh_db.conn_mut(), &temp_dir).unwrap();
    assert!(restore_report.newly_indexed >= 4);

    // Verify attachments are restored
    let att_count: i64 = fresh_db.conn().query_row(
        "SELECT count(*) FROM attachments",
        [],
        |r| r.get(0),
    ).unwrap();
    assert!(att_count >= 1, "Reindexed database must contain attachments");

    let _ = fs::remove_dir_all(&temp_dir);
}
