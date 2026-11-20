use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

const WHATSAPP_KEY_PREFIX: &str = "WHATSAPP_64DIGI_";

#[derive(Debug, Clone, Default)]
pub struct CredentialsStore {
    values: HashMap<String, String>,
}

impl CredentialsStore {
    pub fn load(credentials_file: Option<&Path>) -> Self {
        let mut values = HashMap::new();

        if let Some(path) = credentials_file {
            if let Ok(file_map) = parse_kv_file(path) {
                values.extend(file_map);
            }
        } else if let Some(default_path) = default_credentials_path() {
            if let Ok(file_map) = parse_kv_file(&default_path) {
                values.extend(file_map);
            }
        }

        // Process environment variables always take precedence over file values
        for (k, v) in std::env::vars() {
            values.insert(k, v);
        }

        Self { values }
    }

    pub fn from_map(values: HashMap<String, String>) -> Self {
        Self { values }
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let values = parse_kv_file(path)?;
        Ok(Self { values })
    }

    /// Resolve a 64-digit hex WhatsApp backup key for the given account name.
    /// Matches `WHATSAPP_64DIGI_<account>` (exact or case-insensitive suffix).
    pub fn whatsapp_key_for(&self, account: &str) -> Option<String> {
        let exact_key = format!("{}{}", WHATSAPP_KEY_PREFIX, account);
        if let Some(val) = self.values.get(&exact_key) {
            return normalize_hex_key(val);
        }

        let target_lower = account.to_lowercase();
        let target_norm = normalize_account_slug(&target_lower);
        for (k, v) in &self.values {
            if let Some(suffix) = k.strip_prefix(WHATSAPP_KEY_PREFIX) {
                let suffix_lower = suffix.to_lowercase();
                if suffix_lower == target_lower || normalize_account_slug(&suffix_lower) == target_norm {
                    if let Some(hex) = normalize_hex_key(v) {
                        return Some(hex);
                    }
                }
            }
        }
        None
    }

    /// List all account names discovered from `WHATSAPP_64DIGI_*` entries.
    pub fn whatsapp_accounts(&self) -> Vec<String> {
        let mut accounts: Vec<String> = self
            .values
            .iter()
            .filter_map(|(k, v)| {
                let suffix = k.strip_prefix(WHATSAPP_KEY_PREFIX)?;
                if !suffix.is_empty() && normalize_hex_key(v).is_some() {
                    Some(suffix.to_string())
                } else {
                    None
                }
            })
            .collect();
        accounts.sort();
        accounts.dedup();
        accounts
    }
}

pub fn default_credentials_path() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("MYMESSAGES_CREDENTIALS_FILE") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Some(p);
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let creds = home.join(".credentials");
    if creds.exists() {
        Some(creds)
    } else {
        None
    }
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    if let Some(s) = path.to_str() {
        if let Some(rest) = s.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home).join(rest);
            }
        }
    }
    path.to_path_buf()
}

pub fn parse_kv_file(path: &Path) -> Result<HashMap<String, String>> {
    let expanded = expand_tilde(path);
    let content = fs::read_to_string(&expanded)
        .with_context(|| format!("Failed to read credentials file {:?}", expanded))?;
    let mut map = HashMap::new();
    for raw in content.lines() {
        let mut line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(stripped) = line.strip_prefix("export ") {
            line = stripped.trim();
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().to_string();
            let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
            if !key.is_empty() {
                map.insert(key, val);
            }
        }
    }
    Ok(map)
}

fn normalize_hex_key(raw: &str) -> Option<String> {
    let clean: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    if clean.len() == 64 && clean.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(clean.to_lowercase())
    } else {
        None
    }
}

fn normalize_account_slug(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kv_file_and_lookup() {
        let tmp = std::env::temp_dir().join(format!("mymessages_creds_test_{}.env", std::process::id()));
        let key_a = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let key_b = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
        let content = format!(
            "# comment line\nexport WHATSAPP_64DIGI_alice=\"{}\"\nWHATSAPP_64DIGI_bob_phone='{}'\nINVALID_KEY=short\n",
            key_a, key_b
        );
        fs::write(&tmp, content).unwrap();

        let store = CredentialsStore::from_file(&tmp).unwrap();
        assert_eq!(store.whatsapp_key_for("alice").as_deref(), Some(key_a));
        assert_eq!(store.whatsapp_key_for("ALICE").as_deref(), Some(key_a));
        assert_eq!(store.whatsapp_key_for("bob-phone").as_deref(), Some(key_b));
        assert_eq!(store.whatsapp_key_for("unknown"), None);
        assert_eq!(store.whatsapp_accounts(), vec!["alice", "bob_phone"]);

        let _ = fs::remove_file(&tmp);
    }
}
