//! Authenticated encryption for sensitive SQLite fields and the complete conversation cache.
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
    Aes256Gcm, Nonce,
};
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::path::Path;
use zeroize::Zeroizing;

const PREFIX: &str = "ctdoenc:v1:";
const CACHE_MAGIC: &str = "CTDOZLITE-CACHE-1\n";
const MAX_CACHE_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) struct DataCipher(Zeroizing<Vec<u8>>);
impl DataCipher {
    pub(crate) fn for_path(path: &Path, create: bool) -> Result<Self> {
        Ok(Self(crate::secure_session::data_key(path, create)?))
    }
    pub(crate) fn random() -> Self {
        Self(Zeroizing::new(Aes256Gcm::generate_key(&mut OsRng).to_vec()))
    }
    #[cfg(test)]
    pub(crate) fn test_key(key: u8) -> Self {
        Self(Zeroizing::new(vec![key; 32]))
    }

    pub(crate) fn encrypt(&self, column: &str, id: &str, value: &str) -> Result<String> {
        let cipher = Aes256Gcm::new_from_slice(&self.0)
            .map_err(|_| anyhow::anyhow!("Khóa lịch sử không hợp lệ"))?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let context = serde_json::to_vec(&("ctdozlite-history-v1", column, id))?;
        let encrypted = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: value.as_bytes(),
                    aad: &context,
                },
            )
            .map_err(|_| anyhow::anyhow!("Không mã hóa được lịch sử"))?;
        let mut bytes = nonce.to_vec();
        bytes.extend(encrypted);
        Ok(format!("{PREFIX}{}", B64.encode(bytes)))
    }
    pub(crate) fn decrypt(&self, column: &str, id: &str, value: &str) -> Result<String> {
        let bytes = B64.decode(
            value
                .strip_prefix(PREFIX)
                .context("Lịch sử không đúng định dạng mã hóa")?,
        )?;
        if bytes.len() < 28 {
            bail!("Dữ liệu mã hóa lịch sử quá ngắn");
        }
        let cipher = Aes256Gcm::new_from_slice(&self.0)
            .map_err(|_| anyhow::anyhow!("Khóa lịch sử không hợp lệ"))?;
        let context = serde_json::to_vec(&("ctdozlite-history-v1", column, id))?;
        let plaintext = Zeroizing::new(
            cipher
                .decrypt(
                    Nonce::from_slice(&bytes[..12]),
                    Payload {
                        msg: &bytes[12..],
                        aad: &context,
                    },
                )
                .map_err(|_| anyhow::anyhow!("Lịch sử đã bị sửa đổi hoặc không đúng khóa"))?,
        );
        Ok(std::str::from_utf8(&plaintext)?.to_owned())
    }
}

fn read_cache(path: &Path) -> Result<Option<Zeroizing<String>>> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
        Ok(meta) => {
            if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_CACHE_BYTES {
                bail!("Tệp cache lịch sử không hợp lệ hoặc quá lớn");
            }
        }
    }
    Ok(Some(Zeroizing::new(std::fs::read_to_string(path)?)))
}

fn decode_cache(text: &str, cipher: &DataCipher) -> Result<Vec<crate::chat::Thread>> {
    let plaintext = match text.strip_prefix(CACHE_MAGIC) {
        Some(encrypted) => Zeroizing::new(cipher.decrypt("cache", "threads", encrypted)?),
        None => Zeroizing::new(text.to_owned()),
    };
    Ok(serde_json::from_str(&plaintext)?)
}
fn write_cache(path: &Path, threads: &[crate::chat::Thread], cipher: &DataCipher) -> Result<()> {
    let json = Zeroizing::new(serde_json::to_string(threads)?);
    let encrypted = format!(
        "{CACHE_MAGIC}{}",
        cipher.encrypt("cache", "threads", &json)?
    );
    if encrypted.len() as u64 > MAX_CACHE_BYTES {
        bail!("Cache lịch sử quá lớn; giữ nguyên bản cũ");
    }
    crate::storage::atomic_write(path, encrypted.as_bytes())
}

pub(crate) fn load_cache(path: &Path) -> Result<Vec<crate::chat::Thread>> {
    let _lock = crate::secure_session::lock_session_file(&path.with_extension("cache-io"))?;
    let Some(text) = read_cache(path)? else {
        return Ok(Vec::new());
    };
    let legacy = !text.starts_with(CACHE_MAGIC);
    // Validate legacy JSON before ever creating a new key.
    if legacy {
        let _: Vec<crate::chat::Thread> = serde_json::from_str(&text)?;
    }
    let cipher = DataCipher::for_path(path, legacy)?;
    let threads = decode_cache(&text, &cipher)?;
    if legacy {
        write_cache(path, &threads, &cipher)?;
    }
    Ok(threads)
}
pub(crate) fn save_cache(path: &Path, threads: &[crate::chat::Thread]) -> Result<()> {
    let _lock = crate::secure_session::lock_session_file(&path.with_extension("cache-io"))?;
    let previous = read_cache(path)?;
    let encrypted = previous
        .as_ref()
        .is_some_and(|s| s.starts_with(CACHE_MAGIC));
    if let Some(text) = previous.as_ref().filter(|_| !encrypted) {
        let _: Vec<crate::chat::Thread> = serde_json::from_str(text)?;
    }
    let cipher = DataCipher::for_path(path, !encrypted)?;
    // Refuse to overwrite unreadable or tampered history with an empty/new snapshot.
    if let Some(text) = &previous {
        // Authenticate and validate the old file without allocating a second complete history.
        let plaintext=match text.strip_prefix(CACHE_MAGIC) {
            Some(encrypted)=>Zeroizing::new(cipher.decrypt("cache", "threads", encrypted)?),
            None=>Zeroizing::new(text.to_string()),
        };
        let _: serde::de::IgnoredAny=serde_json::from_str(&plaintext)?;
    }
    drop(previous);
    write_cache(path, threads, &cipher)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires unlocked native OS vault; creates and removes temporary keys"]
    fn native_history_and_cache_roundtrip_and_missing_key_protection() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("history.db");
        let cache = dir.path().join("threads_cache.json");
        struct Cleanup(Vec<std::path::PathBuf>);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                for p in &self.0 {
                    crate::secure_session::delete_test_data_key(p);
                }
            }
        }
        let _cleanup = Cleanup(vec![db.clone(), cache.clone()]);
        let thread: crate::chat::Thread = serde_json::from_str(r#"{"id":"t1","name":"PRIVATE_NATIVE_NAME","preview":"PRIVATE_NATIVE_PREVIEW","time":"","avatar":null,"is_group":false,"messages":[]}"#).unwrap();
        // The public cache loader must migrate a legacy file before returning data.
        std::fs::write(&cache, serde_json::to_vec(&vec![thread.clone()]).unwrap()).unwrap();
        assert_eq!(load_cache(&cache).unwrap()[0].name, thread.name);
        let bytes = std::fs::read_to_string(&cache).unwrap();
        assert!(bytes.starts_with(CACHE_MAGIC));
        assert!(!bytes.contains("PRIVATE_NATIVE"));
        save_cache(&cache, &[thread.clone()]).unwrap();
        assert_eq!(load_cache(&cache).unwrap()[0].preview, thread.preview);
        let mut store = crate::store::MessageStore::open(db.to_str().unwrap()).unwrap();
        crate::store::persist_threads(&mut store, &[thread]).unwrap();
        drop(store);
        let store = crate::store::MessageStore::open(db.to_str().unwrap()).unwrap();
        assert_eq!(store.all_threads().unwrap()[0].name, "PRIVATE_NATIVE_NAME");
        drop(store);
        let intact = std::fs::read(&cache).unwrap();
        let mut tampered = intact.clone();
        let n = tampered.len();
        tampered[n - 5] ^= 1;
        std::fs::write(&cache, &tampered).unwrap();
        assert!(save_cache(&cache, &[]).is_err());
        assert_eq!(std::fs::read(&cache).unwrap(), tampered);
        std::fs::write(&cache, intact).unwrap();
        // Missing keys must never be silently replaced, or overwrite unreadable history.
        crate::secure_session::delete_test_data_key(&db);
        crate::secure_session::delete_test_data_key(&cache);
        let db_before = std::fs::read(&db).unwrap();
        let cache_before = std::fs::read(&cache).unwrap();
        assert!(crate::store::MessageStore::open(db.to_str().unwrap()).is_err());
        assert!(save_cache(&cache, &[]).is_err());
        assert!(load_cache(&cache).is_err());
        assert_eq!(std::fs::read(&db).unwrap(), db_before);
        assert_eq!(std::fs::read(&cache).unwrap(), cache_before);
        assert!(DataCipher::for_path(&cache, false).is_err());
    }
    #[test]
    fn binds_ciphertext_to_field_and_record_and_uses_fresh_nonces() {
        let cipher = DataCipher::test_key(7);
        let a = cipher.encrypt("text", "m1", "Nội dung riêng tư").unwrap();
        assert_ne!(
            a,
            cipher.encrypt("text", "m1", "Nội dung riêng tư").unwrap()
        );
        assert_eq!(
            cipher.decrypt("text", "m1", &a).unwrap(),
            "Nội dung riêng tư"
        );
        assert!(cipher.decrypt("text", "m2", &a).is_err());
        assert!(cipher.decrypt("sender", "m1", &a).is_err());
        assert!(DataCipher::test_key(8).decrypt("text", "m1", &a).is_err());
        assert!(cipher.decrypt("text", "m1", "ctdoenc:v1:AAAA").is_err());
        let mut damaged = a.into_bytes();
        let n = damaged.len();
        damaged[n - 5] ^= 1;
        assert!(cipher
            .decrypt("text", "m1", std::str::from_utf8(&damaged).unwrap())
            .is_err());
    }
    #[test]
    fn complete_cache_roundtrips_without_plaintext_names_or_previews() {
        let path = tempfile::tempdir().unwrap();
        let file = path.path().join("cache.json");
        let cipher = DataCipher::test_key(9);
        let mut thread: crate::chat::Thread = serde_json::from_str(r#"{"id":"1","name":"PRIVATE_NAME","preview":"PRIVATE_PREVIEW","time":"","avatar":null,"is_group":false,"messages":[]}"#).unwrap();
        thread.preview = "PRIVATE_PREVIEW".into();
        write_cache(&file, &[thread], &cipher).unwrap();
        let bytes = std::fs::read_to_string(file).unwrap();
        assert!(!bytes.contains("PRIVATE_NAME"));
        assert!(!bytes.contains("PRIVATE_PREVIEW"));
        let restored = decode_cache(&bytes, &cipher).unwrap();
        assert_eq!(restored[0].name, "PRIVATE_NAME");
        assert_eq!(restored[0].preview, "PRIVATE_PREVIEW");
        assert!(decode_cache(&bytes, &DataCipher::test_key(10)).is_err());
    }
}
