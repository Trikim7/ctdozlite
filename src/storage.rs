//! Private application data and atomic file writes. No credentials belong in the working directory.
use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Initialize before starting threads, so existing relative cache paths stay inside user app data.
pub fn init_data_directory() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("net", "Trikim7", "ctdozlite")
        .context("Không xác định được thư mục dữ liệu riêng của người dùng")?;
    let path = dirs.data_local_dir();
    create_private_directory(path)?;
    let path = path.canonicalize()?;
    std::env::set_current_dir(&path)?;
    Ok(path)
}

pub fn create_private_directory(path: &Path) -> Result<()> {
    // Refuse a redirected leaf instead of changing permissions on an unrelated target.
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if !meta.is_dir() || meta.file_type().is_symlink() {
            bail!("Thư mục dữ liệu phải là thư mục thật, không phải liên kết");
        }
    }
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Replace rather than truncate: a failed write leaves the previous file intact.
/// tempfile creates mode 0600 on Unix; on Windows the private app-data directory supplies its ACL.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            bail!("Từ chối ghi dữ liệu qua liên kết hoặc tệp không thông thường");
        }
    }
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    #[cfg(unix)]
    std::fs::File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replaces_whole_file_and_restricts_unix_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        atomic_write(&path, b"long previous session").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_without_touching_their_target() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("session.json");
        std::fs::write(&target, b"unchanged").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(atomic_write(&link, b"secret").is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"unchanged");
        assert!(create_private_directory(&link).is_err());
    }
}
