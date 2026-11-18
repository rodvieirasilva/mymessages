use super::*;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_config_storage_list() {
    let cfg = AppConfig::default();
    assert_eq!(cfg.storages.len(), 2);
    assert!(cfg.is_storage_enabled("maildir"));
    assert!(cfg.is_storage_enabled("sqlite"));

    let maildir_path = cfg.get_storage_path("maildir");
    assert_eq!(maildir_path, Some(PathBuf::from("./data/mail")));

    let toml_str = toml::to_string_pretty(&cfg).unwrap();
    assert!(toml_str.contains("[[storages]]"));
    assert!(toml_str.contains("name = \"maildir\""));

    let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
    assert_eq!(cfg, parsed);
}

#[test]
fn test_config_save_and_load() {
    let temp_file = std::env::temp_dir().join(format!("test_cfg_list_{}.toml", std::process::id()));
    let mut cfg = AppConfig::default();
    cfg.storages.push(StorageEntry {
        name: "backup_maildir".into(),
        storage_type: StorageType::Maildir,
        enabled: false,
        path: "./backup/mail".into(),
    });
    cfg.save_to_file(&temp_file).unwrap();

    let loaded = AppConfig::load_from_file(&temp_file).unwrap();
    assert_eq!(loaded.storages.len(), 3);
    assert!(!loaded.is_storage_enabled("backup_maildir"));

    let _ = fs::remove_file(temp_file);
}
