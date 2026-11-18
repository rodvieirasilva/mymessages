use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use rusqlite::Connection;

use crate::adapters::FolderAdapter;
use crate::index::hasher::is_hash_indexed;
use crate::index::writer::index_message;
use crate::storage::{extract_header, parse_eml_to_canonical, StoragePipeline};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReindexSource {
    Maildir,
    Folder(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReindexTarget {
    Db,
    Maildir,
    All,
}

pub struct ReindexReport {
    pub total_found: usize,
    pub newly_indexed: usize,
    pub duplicates_skipped: usize,
}

pub fn reindex_from_maildir(
    conn: &mut Connection,
    mail_dir: &Path,
) -> Result<ReindexReport> {
    let mut report = ReindexReport {
        total_found: 0,
        newly_indexed: 0,
        duplicates_skipped: 0,
    };

    if !mail_dir.exists() {
        return Ok(report);
    }

    let mut eml_files = Vec::new();
    find_eml_files(mail_dir, &mut eml_files)?;
    report.total_found = eml_files.len();

    for eml_path in eml_files {
        let content = fs::read_to_string(&eml_path)?;
        if let Some(msg) = parse_eml_to_canonical(&content) {
            let hash = extract_header(&content, "X-MyMessages-SHA256").unwrap_or_else(|| msg.compute_content_hash());
            if is_hash_indexed(conn, &hash)? {
                report.duplicates_skipped += 1;
                continue;
            }

            let stripped = eml_path.strip_prefix(mail_dir).unwrap_or(&eml_path);
            let rel_str = stripped.to_string_lossy();
            let final_rel_path = if rel_str.starts_with("mail/") {
                rel_str.to_string()
            } else {
                format!("mail/{}", rel_str)
            };

            index_message(conn, &msg, &hash, &final_rel_path)?;
            report.newly_indexed += 1;
        }
    }

    Ok(report)
}

pub fn reindex_from_folder(
    pipeline: &StoragePipeline,
    folder: &Path,
) -> Result<ReindexReport> {
    let adapter = FolderAdapter::new(folder);
    let messages = adapter.read_all_messages()?;
    let mut report = ReindexReport {
        total_found: messages.len(),
        newly_indexed: 0,
        duplicates_skipped: 0,
    };

    for msg in messages {
        let hash = msg.compute_content_hash();
        let res = pipeline.write_all(&msg, &hash)?;
        if res.db_indexed || res.eml_path.is_some() {
            report.newly_indexed += 1;
        } else {
            report.duplicates_skipped += 1;
        }
    }

    Ok(report)
}

fn find_eml_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            find_eml_files(&path, files)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("eml") {
            files.push(path);
        }
    }
    Ok(())
}

#[path = "reindex_test.rs"]
#[cfg(test)]
mod tests;
