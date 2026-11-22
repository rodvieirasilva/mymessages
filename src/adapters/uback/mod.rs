pub mod crypt15;
pub mod db_loader;
pub mod media_index;
pub mod msgstore;
pub mod sms;
pub mod telegram;
pub mod wa_contacts;

use std::fs;
use std::path::PathBuf;
use anyhow::Result;

use crate::config::CredentialsStore;
use crate::models::CanonicalMessage;
use db_loader::{find_all_delta_dbs, find_latest_db, media_entries_to_messages, open_or_decrypt_db};
use media_index::UbackMediaIndex;

#[derive(Debug, Clone)]
pub struct UbackAccountTarget {
    pub account: String,
    pub source: String,
    pub wa_dir: PathBuf,
}

#[derive(Debug, Default)]
pub struct UbackScanSummary {
    pub accounts_scanned: Vec<String>,
    pub messages: Vec<CanonicalMessage>,
    pub warnings: Vec<String>,
}

pub struct UbackAdapter {
    backup_root: PathBuf,
    credentials: CredentialsStore,
    account_filter: Vec<String>,
    index_media_without_db: bool,
}

impl UbackAdapter {
    pub fn new(
        backup_root: PathBuf,
        credentials: CredentialsStore,
        account_filter: Vec<String>,
        index_media_without_db: bool,
    ) -> Self {
        Self {
            backup_root,
            credentials,
            account_filter,
            index_media_without_db,
        }
    }

    /// Discover all UBack WhatsApp & WhatsApp Business folders under `backup_root`.
    pub fn discover_targets(&self) -> Result<Vec<UbackAccountTarget>> {
        let mut targets = Vec::new();
        if !self.backup_root.is_dir() {
            return Ok(targets);
        }

        let direct_name = self
            .backup_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("default")
            .to_string();
        let mut found_direct = false;
        for (folder, source) in [
            ("whatsapp", "whatsapp"),
            ("whatsapp-business", "whatsapp_business"),
            ("snapshots/sms", "sms"),
            ("telegram", "telegram"),
        ] {
            let wa_dir = self.backup_root.join(folder);
            if wa_dir.is_dir() && self.allows_account(&direct_name) {
                targets.push(UbackAccountTarget {
                    account: direct_name.clone(),
                    source: source.to_string(),
                    wa_dir,
                });
                found_direct = true;
            }
        }
        if found_direct {
            return Ok(targets);
        }

        let mut entries: Vec<_> = fs::read_dir(&self.backup_root)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let acct_path = entry.path();
            if !acct_path.is_dir() {
                continue;
            }
            let account = entry.file_name().to_string_lossy().to_string();
            if account.starts_with('.') || !self.allows_account(&account) {
                continue;
            }
            for (folder, source) in [
                ("whatsapp", "whatsapp"),
                ("whatsapp-business", "whatsapp_business"),
                ("snapshots/sms", "sms"),
                ("telegram", "telegram"),
            ] {
                let wa_dir = acct_path.join(folder);
                if wa_dir.is_dir() {
                    targets.push(UbackAccountTarget {
                        account: account.clone(),
                        source: source.to_string(),
                        wa_dir,
                    });
                }
            }
        }

        Ok(targets)
    }

    pub fn scan_all(&self) -> Result<UbackScanSummary> {
        let targets = self.discover_targets()?;
        let mut summary = UbackScanSummary::default();

        for target in targets {
            if !summary.accounts_scanned.contains(&target.account) {
                summary.accounts_scanned.push(target.account.clone());
            }
            match self.scan_target(&target) {
                Ok((msgs, warns)) => {
                    summary.messages.extend(msgs);
                    summary.warnings.extend(warns);
                }
                Err(e) => {
                    summary.warnings.push(format!("Account '{}' ({}): {}", target.account, target.source, e));
                }
            }
        }

        Ok(summary)
    }

    fn allows_account(&self, account: &str) -> bool {
        self.account_filter.is_empty()
            || self.account_filter.iter().any(|f| f.eq_ignore_ascii_case(account))
    }

    fn scan_target(&self, target: &UbackAccountTarget) -> Result<(Vec<CanonicalMessage>, Vec<String>)> {
        let mut warnings = Vec::new();
        let account_root = if target.source == "sms" {
            target.wa_dir.parent().and_then(|p| p.parent())
        } else {
            target.wa_dir.parent()
        };
        let vcf_contacts = account_root
            .map(|r| wa_contacts::read_vcf_contacts(&r.join("snapshots/contacts")))
            .unwrap_or_default();

        if target.source == "sms" {
            let msgs = sms::read_sms_snapshots(&target.wa_dir, &target.account, &vcf_contacts)?;
            return Ok((msgs, warnings));
        }
        if target.source == "telegram" {
            let msgs = telegram::read_telegram_dir(&target.wa_dir, &target.account)?;
            return Ok((msgs, warnings));
        }

        let media_index = UbackMediaIndex::scan(&target.wa_dir)?;

        let delta_dbs = find_all_delta_dbs(&target.wa_dir.join("deltas"));
        let msgstore_file = find_latest_db(&target.wa_dir.join("databases"), "msgstore")
            .or_else(|| find_latest_db(&target.wa_dir.join("Databases"), "msgstore"));
        let wa_db_file = find_latest_db(&target.wa_dir.join("backups"), "wa")
            .or_else(|| find_latest_db(&target.wa_dir.join("Backups"), "wa"));

        if delta_dbs.is_empty() && msgstore_file.is_none() {
            if self.index_media_without_db && !media_index.is_empty() {
                return Ok((media_entries_to_messages(&target.account, &target.source, &media_index), warnings));
            }
            warnings.push(format!(
                "No msgstore deltas or databases found in {:?} ({} media files indexed ready to link)",
                target.wa_dir,
                media_index.len()
            ));
            return Ok((Vec::new(), warnings));
        }

        let mut contacts = vcf_contacts;
        if let Some(ref wa_path) = wa_db_file {
            match open_or_decrypt_db(wa_path, &target.account, &self.credentials) {
                Ok(temp_wa) => {
                    if let Ok(wa_map) = wa_contacts::read_wa_contacts(temp_wa.path()) {
                        contacts.extend(wa_map);
                    }
                }
                Err(e) => {
                    warnings.push(format!("Could not read wa.db for '{}': {}", target.account, e));
                }
            }
        }

        let mut all_messages = Vec::new();
        for delta_path in &delta_dbs {
            match msgstore::read_msgstore(delta_path, &target.account, &target.source, &contacts, &media_index) {
                Ok(msgs) => all_messages.extend(msgs),
                Err(e) => warnings.push(format!("Failed reading delta {:?}: {}", delta_path, e)),
            }
        }

        if let Some(db_file) = msgstore_file {
            match open_or_decrypt_db(&db_file, &target.account, &self.credentials) {
                Ok(temp_msgstore) => {
                    let msgs = msgstore::read_msgstore(
                        temp_msgstore.path(),
                        &target.account,
                        &target.source,
                        &contacts,
                        &media_index,
                    )?;
                    all_messages.extend(msgs);
                }
                Err(e) if !all_messages.is_empty() => {
                    warnings.push(format!("Skipped encrypted DB {:?} (already loaded {} delta msgs): {}", db_file, all_messages.len(), e));
                }
                Err(e) => return Err(e),
            }
        }

        Ok((all_messages, warnings))
    }
}

#[path = "uback_test.rs"]
#[cfg(test)]
mod tests;
