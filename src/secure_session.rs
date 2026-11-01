//! AES-256-GCM session envelope; the random key is stored only in the OS credential vault.
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
compile_error!("Secure session persistence currently supports Linux, Windows and macOS only");
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
    Aes256Gcm, Nonce,
};
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Mutex};
use zeroize::Zeroizing;

const AAD: &[u8] = b"ctdozlite-session-v1";
const MAX_SESSION_BYTES: u64 = 8 * 1024 * 1024;
// Serialize key creation and file replacement within one process.
static SAVE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: String,
    version: u32,
    nonce: String,
    ciphertext: String,
}

trait KeyStore {
    fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>>;
    fn write(&self, key: &[u8]) -> Result<()>;
}
struct Vault(keyring::Entry);
impl Vault {
    fn for_path(path: &Path) -> Result<Self> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let canonical = parent
            .canonicalize()?
            .join(path.file_name().context("Thiếu tên tệp phiên")?);
        let account = format!(
            "session-{:x}",
            Sha256::digest(canonical.to_string_lossy().as_bytes())
        );
        Ok(Self(keyring::Entry::new(
            "net.Trikim7.ctdozlite",
            &account,
        )?))
    }
}
impl KeyStore for Vault {
    fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>> {
        match self.0.get_secret() {
            Ok(key) => Ok(Some(Zeroizing::new(key))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e)
                .context("Không đọc được kho khóa hệ điều hành; không dùng tệp phiên không mã hóa"),
        }
    }
    fn write(&self, key: &[u8]) -> Result<()> {
        self.0
            .set_secret(key)
            .context("Không lưu được khóa vào kho hệ điều hành; phiên chỉ còn trong bộ nhớ")
    }
}

fn seal(plaintext: &[u8], vault: &impl KeyStore) -> Result<Vec<u8>> {
    let key = match vault.read()? {
        Some(key) => key,
        None => {
            let key = Zeroizing::new(Aes256Gcm::generate_key(&mut OsRng).to_vec());
            vault.write(&key)?;
            key
        }
    };
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow::anyhow!("Khóa lưu phiên không hợp lệ"))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: AAD,
            },
        )
        .map_err(|_| anyhow::anyhow!("Không mã hóa được phiên"))?;
    Ok(serde_json::to_vec(&Envelope {
        format: "ctdozlite-session".into(),
        version: 1,
        nonce: B64.encode(nonce),
        ciphertext: B64.encode(ciphertext),
    })?)
}

fn unseal(bytes: &[u8], vault: &impl KeyStore) -> Result<Zeroizing<Vec<u8>>> {
    let envelope: Envelope =
        serde_json::from_slice(bytes).context("Tệp phiên không đúng định dạng mã hóa")?;
    if envelope.format != "ctdozlite-session" || envelope.version != 1 {
        bail!("Phiên bản tệp phiên không được hỗ trợ");
    }
    let nonce = B64.decode(envelope.nonce)?;
    if nonce.len() != 12 {
        bail!("Nonce lưu phiên không hợp lệ");
    }
    let ciphertext = B64.decode(envelope.ciphertext)?;
    let key = vault
        .read()?
        .context("Không có khóa cho tệp phiên này; hãy đăng nhập lại")?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow::anyhow!("Khóa lưu phiên không hợp lệ"))?;
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: AAD,
            },
        )
        .map_err(|_| anyhow::anyhow!("Tệp phiên đã bị sửa đổi hoặc không đúng khóa"))?;
    Ok(Zeroizing::new(plaintext))
}

// A second process must not overwrite a just-created vault key before the first commits its file.
fn lock_session_file(path: &Path) -> Result<std::fs::File> {
    use fs2::FileExt;
    let mut name = path
        .file_name()
        .context("Thiếu tên tệp phiên")?
        .to_os_string();
    name.push(".lock");
    let lock_path = path.with_file_name(name);
    if let Ok(meta) = std::fs::symlink_metadata(&lock_path) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            bail!("Tệp khóa phiên không hợp lệ");
        }
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(lock_path)?;
    file.lock_exclusive()?;
    Ok(file)
}

pub fn save(path: &Path, session: &crate::session::SavedSession) -> Result<()> {
    let _lock = SAVE_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("Khóa lưu phiên bị lỗi"))?;
    let _file_lock = lock_session_file(path)?;
    let plaintext = Zeroizing::new(serde_json::to_vec(session)?);
    let bytes = seal(&plaintext, &Vault::for_path(path)?)?;
    crate::storage::atomic_write(path, &bytes)
}

pub fn load(path: &Path) -> Result<crate::session::SavedSession> {
    let _lock = SAVE_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("Khóa lưu phiên bị lỗi"))?;
    let _file_lock = lock_session_file(path)?;
    load_with_vault(path, &Vault::for_path(path)?)
}

fn load_with_vault(path: &Path, vault: &impl KeyStore) -> Result<crate::session::SavedSession> {
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_SESSION_BYTES {
        bail!("Tệp phiên không thông thường hoặc quá lớn");
    }
    let bytes = Zeroizing::new(std::fs::read(path)?);
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if value.get("format").is_some() {
        return Ok(serde_json::from_slice(&unseal(&bytes, vault)?)?);
    }
    // Migrate an existing plaintext session in place before accepting credentials.
    // Never overwrite it or resume it if the vault is locked/unavailable.
    let session: crate::session::SavedSession = serde_json::from_value(value)?;
    let encrypted = seal(&bytes, vault)?;
    crate::storage::atomic_write(path, &encrypted)?;
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[derive(Default)]
    struct MemoryVault(RefCell<Option<Vec<u8>>>);
    impl KeyStore for MemoryVault {
        fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>> {
            Ok(self.0.borrow().clone().map(Zeroizing::new))
        }
        fn write(&self, key: &[u8]) -> Result<()> {
            *self.0.borrow_mut() = Some(key.to_vec());
            Ok(())
        }
    }
    #[test]
    #[ignore = "requires an unlocked native OS credential vault; uses a temporary test entry"]
    fn native_vault_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let vault = Vault::for_path(&path).unwrap();
        struct Cleanup(Vault);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0 .0.delete_credential();
            }
        }
        let cleanup = Cleanup(vault);
        let bytes = seal(b"temporary-test-secret", &cleanup.0).unwrap();
        assert_eq!(
            &*unseal(&bytes, &cleanup.0).unwrap(),
            b"temporary-test-secret"
        );
    }
    #[test]
    fn migrates_legacy_session_before_returning_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let saved = crate::session::SavedSession {
            imei: "test-device".into(),
            computer_name: "test".into(),
            secret_key_b64: None,
            session_key: None,
            zpw_sek: Some("private-cookie".into()),
            login: None,
            domains: crate::session::Domains::default(),
        };
        let original = serde_json::to_vec(&saved).unwrap();
        std::fs::write(&path, &original).unwrap();
        let vault = MemoryVault::default();
        let loaded = load_with_vault(&path, &vault).unwrap();
        assert_eq!(loaded.zpw_sek, saved.zpw_sek);
        let migrated = std::fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&migrated).contains("private-cookie"));
        assert!(serde_json::from_slice::<Envelope>(&migrated).is_ok());
        assert_eq!(
            load_with_vault(&path, &vault).unwrap().zpw_sek,
            saved.zpw_sek
        );
        struct ReadOnlyVault;
        impl KeyStore for ReadOnlyVault {
            fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>> {
                Ok(None)
            }
            fn write(&self, _: &[u8]) -> Result<()> {
                bail!("vault unavailable")
            }
        }
        std::fs::write(&path, &original).unwrap();
        assert!(load_with_vault(&path, &ReadOnlyVault).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
    #[test]
    fn encrypts_with_unique_nonces_and_roundtrips() {
        let vault = MemoryVault::default();
        let secret = b"zpw_sek=secret-session-cookie";
        let a = seal(secret, &vault).unwrap();
        let b = seal(secret, &vault).unwrap();
        assert_ne!(a, b);
        assert!(!String::from_utf8_lossy(&a).contains("secret-session-cookie"));
        assert_eq!(&*unseal(&a, &vault).unwrap(), secret);
    }
    #[test]
    fn rejects_tampering_wrong_keys_bad_nonces_and_future_versions() {
        let vault = MemoryVault::default();
        let bytes = seal(b"secret", &vault).unwrap();
        assert!(unseal(&bytes, &MemoryVault::default()).is_err());
        let wrong = MemoryVault(RefCell::new(Some(vec![99; 32])));
        assert!(unseal(&bytes, &wrong).is_err());
        let mut e: Envelope = serde_json::from_slice(&bytes).unwrap();
        let mut cipher = B64.decode(&e.ciphertext).unwrap();
        cipher[0] ^= 1;
        e.ciphertext = B64.encode(cipher);
        assert!(unseal(&serde_json::to_vec(&e).unwrap(), &vault).is_err());
        e.nonce = B64.encode([0; 11]);
        assert!(unseal(&serde_json::to_vec(&e).unwrap(), &vault).is_err());
        e.version = 2;
        assert!(unseal(&serde_json::to_vec(&e).unwrap(), &vault).is_err());
    }
    #[test]
    fn unavailable_vault_never_falls_back_to_plaintext() {
        struct LockedVault;
        impl KeyStore for LockedVault {
            fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>> {
                bail!("locked")
            }
            fn write(&self, _: &[u8]) -> Result<()> {
                panic!("must not write")
            }
        }
        assert!(seal(b"secret", &LockedVault).is_err());
    }
}
