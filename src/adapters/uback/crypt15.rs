use std::io::Read;
use aes_gcm::aead::consts::{U12, U16};
use aes_gcm::aead::Aead;
use aes_gcm::aes::Aes256;
use aes_gcm::{Aes256Gcm, AesGcm, KeyInit};
use anyhow::{anyhow, bail, Context, Result};
use flate2::read::ZlibDecoder;
use sha2::{Digest, Sha256};

type Aes256Gcm16 = AesGcm<Aes256, U16>;
pub(crate) const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";

/// RFC 2104 HMAC-SHA256 implemented directly over `sha2::Sha256`.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut k_block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let hashed = Sha256::digest(key);
        k_block[..32].copy_from_slice(&hashed);
    } else {
        k_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= k_block[i];
        opad[i] ^= k_block[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    let mut out = [0u8; 32];
    out.copy_from_slice(&outer.finalize());
    out
}

/// Derive the 32-byte AES-256-GCM decryption key from a WhatsApp 64-digit hex key.
pub fn derive_crypt15_key(hex_key: &str) -> Result<[u8; 32]> {
    let clean: String = hex_key.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    let root_bytes = hex::decode(&clean).context("Invalid 64-digit hex WhatsApp key")?;
    if root_bytes.len() != 32 {
        bail!("WhatsApp 64-digit key must decode to 32 bytes (got {})", root_bytes.len());
    }
    let intermediate = hmac_sha256(&[0u8; 32], &root_bytes);
    let mut info = b"backup encryption".to_vec();
    info.push(0x01);
    Ok(hmac_sha256(&intermediate, &info))
}

/// Parse the `.crypt15` protobuf header (`BackupPrefix`) to extract the IV and payload offset.
/// Supports both standard protobuf varint framing (UBack/WhatsApp) and 1-byte size + `0x01` flag.
pub fn parse_crypt15_header(data: &[u8]) -> Result<(Vec<u8>, usize)> {
    if data.len() < 4 {
        bail!("Crypt15 file is too small ({} bytes)", data.len());
    }

    // 1. Standard varint-prefixed BackupPrefix (matches WhatsAppCrypt15Decryptor.kt)
    if let Some((proto_len, proto_start)) = read_varint(data, 0) {
        let proto_end = proto_start.saturating_add(proto_len as usize);
        if proto_len > 0 && proto_end <= data.len() {
            if let Some(iv) = extract_c15_iv(&data[proto_start..proto_end]) {
                return Ok((iv, proto_end));
            }
        }
    }

    // 2. Fallback: 1-byte length + optional 0x01 feature flag byte
    if data[1] == 0x01 {
        let proto_end = 2 + (data[0] as usize);
        if proto_end <= data.len() {
            if let Some(iv) = extract_c15_iv(&data[2..proto_end]) {
                return Ok((iv, proto_end));
            }
        }
    }

    bail!("Missing C15_IV field in Crypt15 protobuf header")
}

/// Decrypt a WhatsApp `.crypt15` file (msgstore.db.crypt15 or wa.db.crypt15) into raw SQLite bytes.
pub fn decrypt_crypt15(encrypted: &[u8], hex_key: &str) -> Result<Vec<u8>> {
    let key = derive_crypt15_key(hex_key)?;
    let (iv, payload_offset) = parse_crypt15_header(encrypted)?;
    let payload = &encrypted[payload_offset..];
    if payload.len() < 16 {
        bail!("Crypt15 ciphertext payload too short");
    }

    let decrypted = decrypt_gcm(&key, &iv, payload).or_else(|first_err| {
        if payload.len() > 32 {
            decrypt_gcm(&key, &iv, &payload[..payload.len() - 16]).map_err(|_| first_err)
        } else {
            Err(first_err)
        }
    })?;

    if decrypted.starts_with(SQLITE_MAGIC) {
        return Ok(decrypted);
    }

    let mut decoder = ZlibDecoder::new(decrypted.as_slice());
    let mut plain = Vec::new();
    decoder
        .read_to_end(&mut plain)
        .context("Failed to zlib-decompress decrypted Crypt15 database")?;
    if !plain.starts_with(SQLITE_MAGIC) {
        bail!("Decrypted Crypt15 payload is not a valid SQLite database");
    }
    Ok(plain)
}

fn decrypt_gcm(key: &[u8; 32], iv: &[u8], ciphertext_and_tag: &[u8]) -> Result<Vec<u8>> {
    match iv.len() {
        12 => {
            let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow!("AES key error: {}", e))?;
            cipher
                .decrypt(aes_gcm::Nonce::<U12>::from_slice(iv), ciphertext_and_tag)
                .map_err(|_| anyhow!("AES-256-GCM decryption failed (wrong key or corrupted file)"))
        }
        16 => {
            let cipher = Aes256Gcm16::new_from_slice(key).map_err(|e| anyhow!("AES key error: {}", e))?;
            cipher
                .decrypt(aes_gcm::Nonce::<U16>::from_slice(iv), ciphertext_and_tag)
                .map_err(|_| anyhow!("AES-256-GCM(16) decryption failed (wrong key or corrupted file)"))
        }
        other => bail!("Unsupported Crypt15 IV length: {}", other),
    }
}

fn extract_c15_iv(buf: &[u8]) -> Option<Vec<u8>> {
    let mut i = 0;
    while i < buf.len() {
        let (tag, next) = read_varint(buf, i)?;
        i = next;
        match tag & 0x07 {
            0 => i = read_varint(buf, i)?.1,
            1 => i += 8,
            2 => {
                let (len, next) = read_varint(buf, i)?;
                let end = next + (len as usize);
                if end > buf.len() { return None; }
                if (tag >> 3) == 3 {
                    if let Some(iv) = extract_nested_bytes_field1(&buf[next..end]) {
                        return Some(iv);
                    }
                }
                i = end;
            }
            5 => i += 4,
            _ => return None,
        }
    }
    None
}

fn extract_nested_bytes_field1(buf: &[u8]) -> Option<Vec<u8>> {
    let mut i = 0;
    while i < buf.len() {
        let (tag, next) = read_varint(buf, i)?;
        i = next;
        match tag & 0x07 {
            0 => i = read_varint(buf, i)?.1,
            2 => {
                let (len, next) = read_varint(buf, i)?;
                let end = next + (len as usize);
                if end > buf.len() { return None; }
                if (tag >> 3) == 1 { return Some(buf[next..end].to_vec()); }
                i = end;
            }
            _ => return None,
        }
    }
    None
}

fn read_varint(buf: &[u8], mut idx: usize) -> Option<(u64, usize)> {
    let mut val = 0u64;
    let mut shift = 0u32;
    while idx < buf.len() && shift < 64 {
        let b = buf[idx];
        idx += 1;
        val |= ((b & 0x7f) as u64) << shift;
        if (b & 0x80) == 0 { return Some((val, idx)); }
        shift += 7;
    }
    None
}

#[path = "crypt15_test.rs"]
#[cfg(test)]
pub mod tests;
