use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};

#[derive(Debug, Clone)]
pub struct WaContact {
    pub display_name: Option<String>,
    pub phone_number: Option<String>,
}

pub fn read_wa_contacts(wa_db_path: &Path) -> Result<HashMap<String, WaContact>> {
    let conn = Connection::open_with_flags(wa_db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("Failed to open wa.db at {:?}", wa_db_path))?;

    let mut contacts = HashMap::new();
    let has_table: bool = conn
        .query_row(
            "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='wa_contacts'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if !has_table {
        return Ok(contacts);
    }

    let mut stmt = conn.prepare(
        "SELECT jid, display_name, number FROM wa_contacts WHERE jid IS NOT NULL",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;

    for row in rows.flatten() {
        let (jid, display_name, number) = row;
        contacts.insert(
            jid,
            WaContact {
                display_name: display_name.filter(|s| !s.trim().is_empty()),
                phone_number: number.filter(|s| !s.trim().is_empty()),
            },
        );
    }
    Ok(contacts)
}

/// Parse UBack `snapshots/contacts/**/*.vcf` files into a lookup map keyed by
/// normalized phone digits (full digits + last-8-digit suffix) so WhatsApp JIDs
/// (`5511999998888@s.whatsapp.net`) and SMS addresses (`+55 11 99999-8888`) resolve to `FN`.
pub fn read_vcf_contacts(contacts_root: &Path) -> HashMap<String, WaContact> {
    let mut map = HashMap::new();
    if !contacts_root.is_dir() {
        return map;
    }
    let mut vcf_files = Vec::new();
    collect_vcf_files(contacts_root, &mut vcf_files);
    vcf_files.sort();

    for path in vcf_files {
        let Ok(content) = fs::read_to_string(&path) else { continue };
        parse_vcf_into(&content, &mut map);
    }
    map
}

pub fn lookup_contact<'a>(contacts: &'a HashMap<String, WaContact>, jid_or_phone: &str) -> Option<&'a WaContact> {
    if let Some(c) = contacts.get(jid_or_phone) {
        return Some(c);
    }
    let raw_user = jid_or_phone.split('@').next().unwrap_or(jid_or_phone);
    let digits: String = raw_user.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    if let Some(c) = contacts.get(&digits) {
        return Some(c);
    }
    if digits.len() >= 8 {
        let tail = &digits[digits.len() - 8..];
        if let Some(c) = contacts.get(tail) {
            return Some(c);
        }
    }
    None
}

fn parse_vcf_into(content: &str, out: &mut HashMap<String, WaContact>) {
    let mut current_name: Option<String> = None;
    let mut current_phones: Vec<String> = Vec::new();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.eq_ignore_ascii_case("BEGIN:VCARD") {
            current_name = None;
            current_phones.clear();
        } else if line.eq_ignore_ascii_case("END:VCARD") {
            if let Some(ref name) = current_name {
                for phone in &current_phones {
                    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
                    if digits.is_empty() {
                        continue;
                    }
                    let contact = WaContact {
                        display_name: Some(name.clone()),
                        phone_number: Some(phone.clone()),
                    };
                    out.entry(digits.clone()).or_insert_with(|| contact.clone());
                    if let Some(without_55) = digits.strip_prefix("55") {
                        if without_55.len() >= 8 {
                            out.entry(without_55.to_string()).or_insert_with(|| contact.clone());
                        }
                    } else if digits.len() == 10 || digits.len() == 11 {
                        out.entry(format!("55{}", digits)).or_insert_with(|| contact.clone());
                    }
                    if digits.len() >= 8 {
                        let tail = digits[digits.len() - 8..].to_string();
                        out.entry(tail).or_insert(contact);
                    }
                }
            }
        } else if let Some(val) = line.strip_prefix("FN:").or_else(|| line.strip_prefix("fn:")) {
            let clean = val.trim().to_string();
            if !clean.is_empty() {
                current_name = Some(clean);
            }
        } else if let Some((key, val)) = line.split_once(':') {
            if key.to_ascii_uppercase().starts_with("TEL") {
                let clean = val.trim().to_string();
                if !clean.is_empty() {
                    current_phones.push(clean);
                }
            }
        }
    }
}

fn collect_vcf_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_vcf_files(&path, out);
        } else if path.is_file()
            && path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("vcf")).unwrap_or(false)
        {
            out.push(path);
        }
    }
}
