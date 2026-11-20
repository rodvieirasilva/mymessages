use std::io::Write;
use aes_gcm::aead::consts::{U12, U16};
use aes_gcm::aead::Aead;
use aes_gcm::aes::Aes256;
use aes_gcm::{Aes256Gcm, AesGcm, KeyInit};
use flate2::write::ZlibEncoder;
use flate2::Compression;

use super::*;

type Aes256Gcm16 = AesGcm<Aes256, U16>;

pub fn encrypt_crypt15_for_test(
    sqlite_bytes: &[u8],
    hex_key: &str,
    iv: &[u8],
    with_trailer: bool,
    with_feature_flag: bool,
) -> Vec<u8> {
    let key = derive_crypt15_key(hex_key).unwrap();
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(sqlite_bytes).unwrap();
    let compressed = encoder.finish().unwrap();

    let ciphertext = match iv.len() {
        12 => {
            let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
            cipher.encrypt(aes_gcm::Nonce::<U12>::from_slice(iv), compressed.as_slice()).unwrap()
        }
        16 => {
            let cipher = Aes256Gcm16::new_from_slice(&key).unwrap();
            cipher.encrypt(aes_gcm::Nonce::<U16>::from_slice(iv), compressed.as_slice()).unwrap()
        }
        _ => panic!("test IV must be 12 or 16 bytes"),
    };

    let mut c15_iv = vec![0x0a, iv.len() as u8];
    c15_iv.extend_from_slice(iv);
    let mut prefix = vec![0x08, 0x01, 0x1a, c15_iv.len() as u8];
    prefix.extend_from_slice(&c15_iv);

    let mut out = vec![prefix.len() as u8];
    if with_feature_flag {
        out.push(0x01);
    }
    out.extend_from_slice(&prefix);
    out.extend_from_slice(&ciphertext);
    if with_trailer {
        out.extend_from_slice(&[0xAB; 16]);
    }
    out
}

#[test]
fn test_crypt15_roundtrip_both_framings() {
    let hex_key = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
    let mut fake_sqlite = SQLITE_MAGIC.to_vec();
    fake_sqlite.extend_from_slice(b"test sqlite body payload 1234567890");

    // Standard varint BackupPrefix (no extra 0x01 flag) + 16-byte IV + MD5 trailer
    let enc_varint = encrypt_crypt15_for_test(&fake_sqlite, hex_key, &[7u8; 16], true, false);
    assert_eq!(decrypt_crypt15(&enc_varint, hex_key).unwrap(), fake_sqlite);

    // Feature-flag 0x01 framing + 16-byte IV + trailer
    let enc_flag = encrypt_crypt15_for_test(&fake_sqlite, hex_key, &[8u8; 16], true, true);
    assert_eq!(decrypt_crypt15(&enc_flag, hex_key).unwrap(), fake_sqlite);

    // 12-byte IV without trailer
    let enc12 = encrypt_crypt15_for_test(&fake_sqlite, hex_key, &[9u8; 12], false, false);
    assert_eq!(decrypt_crypt15(&enc12, hex_key).unwrap(), fake_sqlite);

    let wrong_key = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";
    assert!(decrypt_crypt15(&enc_varint, wrong_key).is_err());
}
