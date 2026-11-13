//! Account boundaries keep existing encrypted files at their original key-bound paths.
use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
static RESTART: AtomicU8 = AtomicU8::new(0);
static FORGET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const OWNER: &str = "account-owner.sha256";
const ACTIVE: &str = "active-account.txt";
pub fn root() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("net", "Trikim7", "ctdozlite")
        .context("Không xác định được thư mục dữ liệu")?;
    crate::storage::create_private_directory(dirs.data_local_dir())?;
    Ok(dirs.data_local_dir().canonicalize()?)
}
fn hash(owner: &str) -> Result<String> {
    ensure!(!owner.is_empty(), "Không có định danh tài khoản");
    Ok(format!("{:x}", Sha256::digest(owner.as_bytes())))
}
fn read_marker(path: &Path) -> Result<Option<String>> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
        Ok(m) => {
            ensure!(
                m.is_file() && !m.file_type().is_symlink() && m.len() <= 128,
                "Thông tin thư mục tài khoản không hợp lệ"
            );
            Ok(Some(std::fs::read_to_string(path)?.trim().into()))
        }
    }
}
fn account_dir(root: &Path, id: &str) -> Result<PathBuf> {
    ensure!(
        id.len() == 64
            && id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "Đường dẫn tài khoản không hợp lệ"
    );
    let parent = root.join("accounts");
    crate::storage::create_private_directory(&parent)?;
    let path = parent.join(id);
    crate::storage::create_private_directory(&path)?;
    Ok(path)
}
pub fn selected_directory(root: &Path) -> Result<PathBuf> {
    match read_marker(&root.join(ACTIVE))? {
        None => Ok(root.to_path_buf()),
        Some(id) if id == "legacy" => Ok(root.to_path_buf()),
        Some(id) => account_dir(root, &id),
    }
}
/// Only called for an already encrypted, restored session, before loading any history.
pub fn bind_restored(owner: &str) -> Result<()> {
    let here = std::env::current_dir()?;
    bind(&here, &hash(owner)?)
}
fn bind(path: &Path, id: &str) -> Result<()> {
    if let Some(existing) = read_marker(&path.join(OWNER))? {
        ensure!(
            existing == id,
            "Lịch sử thuộc tài khoản khác; dừng để bảo vệ dữ liệu"
        );
    } else {
        crate::storage::atomic_write(&path.join(OWNER), id.as_bytes())?;
    }
    Ok(())
}
fn destination(root: &Path, current: &Path, id: &str) -> Result<PathBuf> {
    if read_marker(&current.join(OWNER))?.as_deref() == Some(id) {
        return Ok(current.to_path_buf());
    }
    if read_marker(&root.join(OWNER))?.as_deref() == Some(id) {
        return Ok(root.to_path_buf());
    }
    // Unknown legacy history remains untouched; a new login gets a separate directory.
    account_dir(root, id)
}
pub fn stage_login(session: &crate::session::Session) -> Result<()> {
    let owner = &session
        .login
        .as_ref()
        .context("Đăng nhập chưa hoàn tất")?
        .uin;
    ensure!(
        !session.session_key_bytes().is_empty()
            && session.zpw_sek.as_ref().is_some_and(|s| !s.is_empty()),
        "Máy chủ chưa cấp đủ thông tin phiên"
    );
    let base = root()?;
    let id = hash(owner)?;
    let target = destination(&base, &std::env::current_dir()?, &id)?;
    if let Some(existing) = read_marker(&target.join(OWNER))? {
        ensure!(existing == id, "Thư mục thuộc tài khoản khác");
    }
    // Persist only at the target path: never move an envelope away from its vault key.
    session.save_to_file(
        target
            .join("session.json")
            .to_str()
            .context("Đường dẫn không hợp lệ")?,
    )?;
    bind(&target, &id)?;
    let selected = if target == base { "legacy" } else { &id };
    crate::storage::atomic_write(&base.join(ACTIVE), selected.as_bytes())?;
    RESTART.store(1, Ordering::SeqCst);
    Ok(())
}
pub fn request_logout() {
    RESTART.store(2, Ordering::SeqCst);
}
pub fn restarting() -> bool {
    RESTART.load(Ordering::SeqCst) != 0
}
pub fn logging_out() -> bool {
    RESTART.load(Ordering::SeqCst) == 2
}
pub fn request_forget_on_exit() {
    FORGET.store(true, Ordering::SeqCst);
}
pub fn forgetting() -> bool {
    FORGET.load(Ordering::SeqCst)
}
pub fn persistence_blocked() -> bool {
    restarting() || forgetting()
}
/// Called after the window has saved its current history and closed.
pub fn finish_logout() -> Result<()> {
    remove_session(Path::new("session.json"))
}
fn remove_session(path: &Path) -> Result<()> {
    crate::secure_session::remove_saved_session(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accounts_stay_separate_and_legacy_is_not_moved() {
        let root = tempfile::tempdir().unwrap();
        let a = hash("account-a").unwrap();
        let b = hash("account-b").unwrap();
        bind(root.path(), &a).unwrap();
        std::fs::write(root.path().join("threads_cache.json"), b"encrypted-history").unwrap();
        assert_eq!(
            destination(root.path(), root.path(), &a).unwrap(),
            root.path()
        );
        let second = destination(root.path(), root.path(), &b).unwrap();
        assert_ne!(second, root.path());
        bind(&second, &b).unwrap();
        assert_eq!(destination(root.path(), &second, &a).unwrap(), root.path());
        assert_eq!(
            std::fs::read(root.path().join("threads_cache.json")).unwrap(),
            b"encrypted-history"
        );
        assert!(bind(root.path(), &b).is_err());
    }
    #[test]
    fn selection_rejects_traversal_and_bad_markers() {
        let root = tempfile::tempdir().unwrap();
        for value in ["../../other", "", "UPPER", "/tmp"] {
            crate::storage::atomic_write(&root.path().join(ACTIVE), value.as_bytes()).unwrap();
            assert!(selected_directory(root.path()).is_err());
        }
    }
    #[test]
    fn unknown_old_history_is_never_assigned_to_a_new_login() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("zalo_history.db"), b"untouched").unwrap();
        let target = destination(root.path(), root.path(), &hash("new").unwrap()).unwrap();
        assert_ne!(target, root.path());
        assert!(target.starts_with(root.path().join("accounts")));
    }
    #[cfg(unix)]
    #[test]
    fn account_symlinks_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(other.path(), root.path().join("accounts")).unwrap();
        assert!(account_dir(root.path(), &hash("a").unwrap()).is_err());
    }
}

#[cfg(test)]
mod logout_tests {
    use super::*;
    #[test]
    fn logout_removes_only_authentication_and_keeps_encrypted_history() {
        let dir = tempfile::tempdir().unwrap();
        let session = dir.path().join("session.json");
        std::fs::write(&session, b"encrypted-session").unwrap();
        let history = dir.path().join("zalo_history.db");
        std::fs::write(&history, b"encrypted-history").unwrap();
        remove_session(&session).unwrap();
        remove_session(&session).unwrap();
        assert!(!session.exists());
        assert_eq!(std::fs::read(history).unwrap(), b"encrypted-history");
    }
}

#[cfg(all(test,target_os="linux"))]
mod vault_cycle_tests {
    use super::*;
    use crate::session::{LoginInfo, Session};
    fn fixture(owner: &str) -> Session {
        let mut s = Session::new(false, false, false).unwrap();
        s.login = Some(LoginInfo {
            uin: owner.into(),
            ..Default::default()
        });
        s.secret_key = Some(vec![17; 32]);
        s.set_zpw_sek("fixture-cookie-not-a-real-account".into());
        s
    }
    fn save_history(name: &str) {
        let threads = vec![crate::chat::Thread {
            id: name.into(),
            name: name.into(),
            preview: String::new(),
            time: String::new(),
            avatar: None,
            is_group: false,
            messages: vec![],
        }];
        crate::chat::save_threads_cache(&threads);
        assert_eq!(crate::chat::load_threads_cache()[0].id, name);
    }
    #[test]
    #[ignore = "uses the real OS credential vault with disposable, isolated fixture paths"]
    fn encrypted_account_cycle() {
        if let Ok(phase) = std::env::var("CTDO_ACCOUNT_FIXTURE_PHASE") {
            let xdg=PathBuf::from(std::env::var_os("XDG_DATA_HOME").expect("isolated fixture directory required"));
            assert!(xdg.is_absolute());
            assert_eq!(std::fs::read_to_string(xdg.join(".ctdozlite-test-fixture")).unwrap(),"synthetic-only");
            crate::storage::init_data_directory().unwrap();
            match phase.as_str() {
                "initialize-a" => {
                    let s = fixture("fixture-a");
                    s.save_to_file("session.json").unwrap();
                    bind_restored("fixture-a").unwrap();
                    save_history("history-a");
                    let before = std::fs::read("threads_cache.json").unwrap();
                    request_logout();
                    assert!(s.save_to_file("session.json").is_err());
                    finish_logout().unwrap();
                    assert_eq!(before, std::fs::read("threads_cache.json").unwrap());
                    assert!(!Path::new("session.json").exists());
                }
                "login-a" => stage_login(&fixture("fixture-a")).unwrap(),
                "login-b" => stage_login(&fixture("fixture-b")).unwrap(),
                "forget-a" => {
                    let s = fixture("fixture-a");
                    request_forget_on_exit();
                    assert!(!restarting());
                    assert!(forgetting());
                    assert!(s.save_to_file("session.json").is_err());
                    finish_logout().unwrap();
                    assert_eq!(crate::chat::load_threads_cache()[0].id, "history-a");
                }
                "verify-logout" => {
                    assert!(!Path::new("session.json").exists());
                    assert_eq!(crate::chat::load_threads_cache()[0].id, "history-a");
                }
                "verify-a" => {
                    let mut s = Session::new(false, false, false).unwrap();
                    s.load_from_file("session.json").unwrap();
                    assert_eq!(s.login.unwrap().uin, "fixture-a");
                    assert_eq!(crate::chat::load_threads_cache()[0].id, "history-a");
                }
                "verify-b" => {
                    let mut s = Session::new(false, false, false).unwrap();
                    s.load_from_file("session.json").unwrap();
                    assert_eq!(s.login.unwrap().uin, "fixture-b");
                    assert!(crate::chat::load_threads_cache().is_empty());
                    save_history("history-b");
                }
                "cleanup" => {
                    let base = root().unwrap();
                    for directory in [
                        &base,
                        &base.join("accounts").join(hash("fixture-b").unwrap()),
                    ] {
                        for name in ["session.json", "threads_cache.json"] {
                            crate::secure_session::delete_test_data_key(&directory.join(name));
                        }
                    }
                }
                _ => panic!("unknown fixture phase"),
            }
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(".ctdozlite-test-fixture"),"synthetic-only").unwrap();
        let result = (|| -> Result<()> {
            for phase in [
                "initialize-a",
                "login-a",
                "verify-a",
                "forget-a",
                "login-a",
                "login-b",
                "verify-b",
                "login-a",
                "verify-a",
            ] {
                let output = std::process::Command::new(std::env::current_exe()?)
                    .args([
                        "--ignored",
                        "--exact",
                        "account_storage::vault_cycle_tests::encrypted_account_cycle",
                        "--test-threads=1",
                    ])
                    .env("XDG_DATA_HOME", temp.path())
                    .env("CTDO_ACCOUNT_FIXTURE_PHASE", phase)
                    .output()?;
                ensure!(
                    output.status.success(),
                    "Fixture {phase} failed: {} {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Ok(())
        })();
        let base = temp.path().join("ctdozlite");
        for directory in [
            &base,
            &base.join("accounts").join(hash("fixture-b").unwrap()),
        ] {
            for name in ["session.json", "threads_cache.json"] {
                crate::secure_session::delete_test_data_key(&directory.join(name));
            }
        }
        result.unwrap();
    }
}
