//! On-demand previews, private temporary files, bounded download and helper output.
use anyhow::{ensure, Context, Result};
use std::io::Write;
use tokio::io::AsyncReadExt;
use std::time::Duration;

pub const MAX_BYTES: usize = 10 * 1024 * 1024;
const HELPER:&str=include_str!("../scripts/document_preview.py");
#[derive(Debug,Clone,serde::Deserialize)]
pub struct Preview {
    pub text:String,
    #[serde(default)] pub truncated:bool,
}

pub fn extension(title:&str)->Option<String>{
    let title=title.split(" (").next().unwrap_or(title).trim();
    let ext=title.rsplit_once('.')?.1.to_ascii_lowercase();
    matches!(ext.as_str(),"pdf"|"doc"|"docx"|"xls"|"xlsx"|"xlsm"|"markdown"|"md"|"txt"|"csv"|"tsv").then_some(ext)
}

pub async fn read(client:&reqwest::Client,url:&str,ext:&str)->Result<Preview>{
    ensure!(extension(&format!("file.{ext}")).as_deref()==Some(ext),"Không hỗ trợ định dạng");
    let mut response=crate::inline_video::media_response(client,url).await?;
    ensure!(response.content_length().unwrap_or(0)<=MAX_BYTES as u64,"Chỉ xem trước tệp tối đa 10 MB");
    let mut temp=tempfile::NamedTempFile::new()?;
    let mut size=0usize;
    while let Some(bytes)=response.chunk().await? {
        size+=bytes.len();ensure!(size<=MAX_BYTES,"Chỉ xem trước tệp tối đa 10 MB");
        temp.write_all(&bytes)?;
    }
    temp.flush()?;
    extract(temp.path(),ext).await
}
pub async fn extract(path:&std::path::Path, ext:&str)->Result<Preview>{
    let mut cmd=tokio::process::Command::new("python3");
    cmd.args(["-I","-c",HELPER]).arg(path).arg(ext)
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).kill_on_drop(true);
    let mut child=cmd.spawn().context("Cần Python 3 để xem trước tài liệu")?;
    let stdout=child.stdout.take().context("Không mở được bộ đọc")?;
    let mut output=Vec::new();
    tokio::time::timeout(Duration::from_secs(20),async {
        stdout.take(256*1024+1).read_to_end(&mut output).await?;
        ensure!(output.len()<=256*1024,"Kết quả xem trước quá lớn");
        ensure!(child.wait().await?.success(),"Bộ đọc tài liệu đã dừng");
        Ok::<(),anyhow::Error>(())
    }).await.context("Hết thời gian đọc tài liệu")??;
    let raw:serde_json::Value=serde_json::from_slice(&output)?;
    if let Some(error)=raw.get("error").and_then(serde_json::Value::as_str){anyhow::bail!("{error}");}
    let preview:Preview=serde_json::from_value(raw)?;
    ensure!(preview.text.chars().count()<=24000,"Kết quả vượt giới hạn");
    Ok(preview)
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn finds_extension_with_attachment_size_suffix(){
        assert_eq!(extension("Văn bản.docx (2 MB)").as_deref(),Some("docx"));
        assert_eq!(extension("notes.MD").as_deref(),Some("md"));
        assert_eq!(extension("program.exe"),None);
    }
    #[tokio::test]async fn reads_utf8_text_and_markdown_and_bounds_output(){
        let path=tempfile::NamedTempFile::new().unwrap();
        std::fs::write(path.path(),"# Tiêu đề\nNội dung tiếng Việt").unwrap();
        for ext in ["md","txt"] {
            let p=extract(path.path(),ext).await.unwrap();
            assert!(p.text.contains("Nội dung tiếng Việt"));assert!(!p.truncated);
        }
        std::fs::write(path.path(),"a".repeat(25000)).unwrap();
        let p=extract(path.path(),"txt").await.unwrap();
        assert_eq!(p.text.len(),24000);assert!(p.truncated);
    }
}
