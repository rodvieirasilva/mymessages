use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

pub struct MaildirStorage {
    pub root_dir: PathBuf,
}

impl MaildirStorage {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }

    pub fn sanitize_name(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' || c == '@' { c } else { '_' })
            .collect()
    }

    pub fn get_chat_dir(&self, source: &str, chat_id: &str) -> PathBuf {
        let clean_source = Self::sanitize_name(source);
        let clean_chat = Self::sanitize_name(chat_id);
        if self.root_dir.ends_with("mail") {
            self.root_dir.join(clean_source).join(clean_chat)
        } else {
            self.root_dir.join("mail").join(clean_source).join(clean_chat)
        }
    }

    pub fn init_chat_folders(&self, source: &str, chat_id: &str) -> Result<PathBuf> {
        let chat_dir = self.get_chat_dir(source, chat_id);
        fs::create_dir_all(chat_dir.join("tmp"))
            .with_context(|| format!("Failed to create maildir tmp in {:?}", chat_dir))?;
        fs::create_dir_all(chat_dir.join("new"))
            .with_context(|| format!("Failed to create maildir new in {:?}", chat_dir))?;
        fs::create_dir_all(chat_dir.join("cur"))
            .with_context(|| format!("Failed to create maildir cur in {:?}", chat_dir))?;
        Ok(chat_dir)
    }

    pub fn write_message(
        &self,
        source: &str,
        chat_id: &str,
        message_id: &str,
        timestamp: i64,
        content_hash: &str,
        eml_content: &str,
    ) -> Result<PathBuf> {
        let chat_dir = self.init_chat_folders(source, chat_id)?;
        let clean_msg_id = Self::sanitize_name(message_id);
        let hash_prefix = if content_hash.len() >= 8 { &content_hash[..8] } else { content_hash };
        let filename = format!("{}.{}.{}.eml", timestamp, clean_msg_id, hash_prefix);

        let tmp_path = chat_dir.join("tmp").join(&filename);
        let cur_path = chat_dir.join("cur").join(&filename);

        fs::write(&tmp_path, eml_content.as_bytes())
            .with_context(|| format!("Failed to write tmp EML to {:?}", tmp_path))?;
        fs::rename(&tmp_path, &cur_path)
            .with_context(|| format!("Failed to move EML to {:?}", cur_path))?;

        // Return path relative to base directory
        let base_dir = if self.root_dir.ends_with("mail") {
            self.root_dir.parent().unwrap_or(&self.root_dir)
        } else {
            &self.root_dir
        };

        cur_path
            .strip_prefix(base_dir)
            .map(|p| p.to_path_buf())
            .with_context(|| "Failed to strip prefix from eml path")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maildir_atomic_write() {
        let temp_dir = std::env::temp_dir().join("mymessages_test_maildir");
        let _ = fs::remove_dir_all(&temp_dir);

        let storage = MaildirStorage::new(&temp_dir);
        let rel_path = storage
            .write_message(
                "whatsapp",
                "chat:123/abc",
                "msg_999",
                1700000000,
                "abcdef12345678",
                "Hello EML Content",
            )
            .unwrap();

        let full_path = temp_dir.join(&rel_path);
        assert!(full_path.exists());
        let read_content = fs::read_to_string(&full_path).unwrap();
        assert_eq!(read_content, "Hello EML Content");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
