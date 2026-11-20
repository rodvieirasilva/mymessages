use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct MediaEntry {
    pub path: PathBuf,
    pub filename: String,
    pub file_size: u64,
    pub is_sent: bool,
    pub modified_ts: i64,
}

#[derive(Debug, Default)]
pub struct UbackMediaIndex {
    by_dir_and_name: HashMap<(bool, String), MediaEntry>,
    by_name: HashMap<String, MediaEntry>,
    all_entries: Vec<MediaEntry>,
}

impl UbackMediaIndex {
    /// Walk UBack's `{received,sent}/{images,videos,audios,documents}/{YYYY}/{MM}/`
    /// directories once to build an O(1) lookup table without duplicating any files.
    pub fn scan(wa_root: &Path) -> Result<Self> {
        let mut index = Self::default();
        if !wa_root.is_dir() {
            return Ok(index);
        }

        for (dir_name, is_sent) in [("received", false), ("sent", true)] {
            let subdir = wa_root.join(dir_name);
            if subdir.is_dir() {
                walk_media_dir(&subdir, is_sent, &mut index)?;
            }
        }

        // Also support legacy flat WhatsApp folders if present directly under wa_root
        for legacy in [
            "WhatsApp Images",
            "WhatsApp Video",
            "WhatsApp Audio",
            "WhatsApp Voice Notes",
            "WhatsApp Documents",
        ] {
            let subdir = wa_root.join(legacy);
            if subdir.is_dir() {
                walk_media_dir(&subdir, false, &mut index)?;
            }
        }

        Ok(index)
    }

    pub fn len(&self) -> usize {
        self.all_entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.all_entries.is_empty()
    }

    pub fn all_entries(&self) -> &[MediaEntry] {
        &self.all_entries
    }

    /// Resolve a WhatsApp `message_media.file_path` or `media_name` to the exact
    /// on-disk UBack media file without copying or duplicating bytes.
    pub fn resolve(
        &self,
        wa_file_path: Option<&str>,
        media_name: Option<&str>,
        is_from_me: bool,
    ) -> Option<&MediaEntry> {
        let mut candidates: Vec<String> = Vec::new();
        let mut path_says_sent = is_from_me;

        if let Some(fp) = wa_file_path {
            let norm = fp.replace('\\', "/");
            if norm.to_lowercase().contains("/sent/") {
                path_says_sent = true;
            }
            if let Some(fname) = norm.rsplit('/').next() {
                if !fname.is_empty() {
                    candidates.push(fname.to_lowercase());
                }
            }
        }
        if let Some(name) = media_name {
            let clean = name.trim();
            if !clean.is_empty() {
                candidates.push(clean.to_lowercase());
            }
        }

        for name in &candidates {
            if let Some(hit) = self.by_dir_and_name.get(&(path_says_sent, name.clone())) {
                return Some(hit);
            }
        }
        for name in &candidates {
            if let Some(hit) = self.by_name.get(name) {
                return Some(hit);
            }
        }

        // Fallback: handle UBack conflict-disambiguated names (`<stem>_<shortHash>.<ext>`)
        for name in &candidates {
            if let Some((stem, ext)) = name.rsplit_once('.') {
                let prefix = format!("{}_", stem);
                let suffix = format!(".{}", ext);
                if let Some(hit) = self.all_entries.iter().find(|e| {
                    let lower = e.filename.to_lowercase();
                    lower.starts_with(&prefix) && lower.ends_with(&suffix)
                }) {
                    return Some(hit);
                }
            }
        }

        None
    }

    pub fn unlinked<'a>(&'a self, linked: &HashSet<PathBuf>) -> Vec<&'a MediaEntry> {
        self.all_entries
            .iter()
            .filter(|e| !linked.contains(&e.path))
            .collect()
    }
}

fn walk_media_dir(dir: &Path, default_is_sent: bool, index: &mut UbackMediaIndex) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            let is_sent = default_is_sent || name.eq_ignore_ascii_case("sent");
            walk_media_dir(&path, is_sent, index)?;
        } else if path.is_file() {
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified_ts = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let media = MediaEntry {
                path,
                filename: name.clone(),
                file_size: meta.len(),
                is_sent: default_is_sent,
                modified_ts,
            };
            let lower = name.to_lowercase();
            index.by_dir_and_name.entry((default_is_sent, lower.clone())).or_insert_with(|| media.clone());
            index.by_name.entry(lower).or_insert_with(|| media.clone());
            index.all_entries.push(media);
        }
    }
    Ok(())
}

pub fn guess_mimetype(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "heic" | "heif" => "image/heic",
        "mp4" => "video/mp4",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "3gp" => "video/3gpp",
        "opus" => "audio/opus",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "amr" => "audio/amr",
        "pdf" => "application/pdf",
        "txt" | "md" | "csv" => "text/plain",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }
}
