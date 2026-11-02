//! Clipboard attachments are prepared off the UI thread, bounded, and kept private until sent.
use anyhow::{ensure, Context, Result};
use std::{path::PathBuf, sync::Arc};
pub struct ClipboardImage {
    pub path: Arc<tempfile::TempPath>,
    pub width: u32,
    pub height: u32,
    pub preview: eframe::egui::ColorImage,
}
pub enum Contents {
    Image(ClipboardImage),
    Files(Vec<PathBuf>),
    Text(String),
    Empty,
}
pub struct Attachment {
    pub path: PathBuf,
    pub temporary: Option<Arc<tempfile::TempPath>>,
}
const MAX_BYTES: usize = 24 * 1024 * 1024;
pub fn read(fallback: Option<String>) -> Result<Contents> {
    #[cfg(target_os = "linux")]
    {
        let wl = if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            command("wl-paste", &["--list-types"])
        } else {
            None
        };
        let wayland = wl.is_some();
        let targets = wl.or_else(|| {
            command(
                "xclip",
                &["-selection", "clipboard", "-target", "TARGETS", "-out"],
            )
        });
        if let Some(targets) = targets {
            let targets = String::from_utf8_lossy(&targets);
            let read = |mime: &str| {
                if wayland {
                    command("wl-paste", &["--no-newline", "--type", mime])
                } else {
                    command(
                        "xclip",
                        &["-selection", "clipboard", "-target", mime, "-out"],
                    )
                }
            };
            for mime in ["x-special/gnome-copied-files", "text/uri-list"] {
                if targets.lines().any(|t| t.trim() == mime) {
                    if let Some(bytes) = read(mime) {
                        let files = parse_file_list(&String::from_utf8_lossy(&bytes))?;
                        if !files.is_empty() {
                            return Ok(Contents::Files(files));
                        }
                    }
                }
            }
            for mime in ["image/png", "image/jpeg", "image/webp", "image/bmp"] {
                if targets.lines().any(|t| t.trim() == mime) {
                    if let Some(bytes) = read(mime) {
                        return prepare_image(&bytes).map(Contents::Image);
                    }
                }
            }
            if let Some(text) = fallback {
                return Ok(Contents::Text(text));
            }
            for mime in ["UTF8_STRING", "text/plain;charset=utf-8", "text/plain"] {
                if targets.lines().any(|t| t.trim() == mime) {
                    if let Some(bytes) = read(mime) {
                        return Ok(Contents::Text(String::from_utf8(bytes)?));
                    }
                }
            }
        }
    }
    Ok(fallback.map(Contents::Text).unwrap_or(Contents::Empty))
}
fn command(program: &str, args: &[&str]) -> Option<Vec<u8>> {
    use std::{
        io::Read,
        process::{Command, Stdio},
        time::Duration,
    };
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout.take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes);
        let _ = tx.send(result.ok().map(|_| bytes));
    });
    let bytes = rx.recv_timeout(Duration::from_secs(2)).ok().flatten();
    if bytes.as_ref().is_none_or(|b| b.len() > MAX_BYTES) {
        let _ = child.kill();
    }
    let success = child.wait().ok().is_some_and(|s| s.success());
    let _ = reader.join();
    bytes.filter(|b| success && b.len() <= MAX_BYTES)
}
pub fn parse_file_list(text: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('#') && !matches!(*s, "copy" | "cut"))
    {
        let url = reqwest::Url::parse(line).context("Clipboard không chứa danh sách tệp hợp lệ")?;
        ensure!(
            url.scheme() == "file" && url.host_str().is_none_or(|s| s == "localhost"),
            "Chỉ dán được tệp có trên máy này"
        );
        let path = url
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("Đường dẫn tệp không hợp lệ"))?;
        if path.is_file() && !files.contains(&path) {
            ensure!(
                path.metadata()?.len() <= 128 * 1024 * 1024,
                "Mỗi tệp hiện tối đa 128 MB để giữ mức dùng RAM thấp"
            );
            files.push(path);
        }
        ensure!(files.len() <= 20, "Mỗi lần dán tối đa 20 tệp");
    }
    Ok(files)
}
pub fn prepare_image(bytes: &[u8]) -> Result<ClipboardImage> {
    ensure!(bytes.len() <= MAX_BYTES, "Ảnh clipboard quá lớn");
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?;
    let (width, height) = (image.width(), image.height());
    let file = tempfile::Builder::new()
        .prefix("ctdozlite-clipboard-")
        .suffix(".png")
        .tempfile()?;
    image.save_with_format(file.path(), image::ImageFormat::Png)?;
    let thumb = image.thumbnail(240, 240).to_rgba8();
    let preview = eframe::egui::ColorImage::from_rgba_unmultiplied(
        [thumb.width() as usize, thumb.height() as usize],
        thumb.as_raw(),
    );
    Ok(ClipboardImage {
        path: Arc::new(file.into_temp_path()),
        width,
        height,
        preview,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_manager_clipboards_preserve_unicode_and_reject_remote_files() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("tệp thử.txt");
        std::fs::write(&p, "fixture").unwrap();
        let uri = reqwest::Url::from_file_path(&p).unwrap();
        assert_eq!(
            parse_file_list(&format!("copy\n{uri}\n{uri}\n")).unwrap(),
            vec![p]
        );
        assert!(parse_file_list("file://remote-host/etc/passwd").is_err());
        assert!(parse_file_list("https://example.test/file").is_err());
    }
    #[test]
    fn clipboard_images_keep_original_size_and_remove_private_temporary_file() {
        let image = image::DynamicImage::new_rgb8(800, 600);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        let prepared = prepare_image(bytes.get_ref()).unwrap();
        assert_eq!((prepared.width, prepared.height), (800, 600));
        assert_eq!(prepared.preview.size, [240, 180]);
        let path = prepared.path.to_path_buf();
        assert!(path.is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        drop(prepared);
        assert!(!path.exists());
    }
}
