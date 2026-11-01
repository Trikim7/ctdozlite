//! One inline video at a time. Decoders only receive a private, bounded local download.
use std::{io::Read, process::{Child,Command,Stdio}, sync::{Arc,Mutex,atomic::{AtomicBool,AtomicU64,Ordering}}, time::{Duration,Instant}};
use eframe::egui::{self,ColorImage};

const WIDTH:usize=640;
const HEIGHT:usize=360;
const FPS:u64=24;

#[derive(Clone,Default)]
pub struct VideoUiState {
    pub url:Option<String>, pub loading:Option<String>, pub playing:bool,
    pub finished:bool, pub error:Option<String>, pub seconds:u64,
}
pub fn state_id()->egui::Id {egui::Id::new("inline_video_state")}
pub fn texture_key(url:&str)->String {format!("video_{}",crate::crypto::md5_hex(url))}

struct Playback {
    stop:Arc<AtomicBool>, video:Arc<Mutex<Child>>, audio:Option<Child>,
}
impl Drop for Playback {
    fn drop(&mut self) {
        self.stop.store(true,Ordering::Release);
        if let Ok(mut child)=self.video.lock(){let _=child.kill();}
        if let Some(child)=self.audio.as_mut(){let _=child.kill();let _=child.wait();}
    }
}

pub struct InlineVideo {
    pub url:String,
    path:Arc<tempfile::TempPath>,
    frame:Arc<Mutex<Option<ColorImage>>>,
    position:Arc<AtomicU64>, finished:Arc<AtomicBool>, error:Arc<Mutex<Option<String>>>,
    playback:Option<Playback>, ctx:egui::Context,
}
impl InlineVideo {
    pub fn new(url:String,path:tempfile::TempPath,ctx:egui::Context)->anyhow::Result<Self> {
        let mut player=Self {url,path:Arc::new(path),frame:Default::default(),position:Default::default(),finished:Default::default(),error:Default::default(),playback:None,ctx};
        player.play()?;Ok(player)
    }
    fn play(&mut self)->anyhow::Result<()> {
        if self.finished.load(Ordering::Acquire) {self.position.store(0,Ordering::Release);}
        self.finished.store(false,Ordering::Release);
        let offset=self.position.load(Ordering::Acquire);
        let seek=format!("{:.3}",offset as f64/FPS as f64);
        let mut command=Command::new("ffmpeg");
        command.args(["-nostdin","-v","error","-protocol_whitelist","file,pipe","-ss",&seek,"-i"]).arg(self.path.as_os_str())
            .args(["-an","-vf","fps=24,scale=640:360:force_original_aspect_ratio=decrease,pad=640:360:(ow-iw)/2:(oh-ih)/2","-pix_fmt","rgba","-f","rawvideo","pipe:1"])
            .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
        hide_console(&mut command);
        let mut child=command.spawn().map_err(|_|anyhow::anyhow!("Cần cài FFmpeg để phát video trong hội thoại"))?;
        let mut stdout=child.stdout.take().unwrap();
        let video=Arc::new(Mutex::new(child));
        let mut audio_command=Command::new("ffplay");
        audio_command.args(["-v","error","-nodisp","-autoexit","-protocol_whitelist","file,pipe","-ss",&seek,"-i"]).arg(self.path.as_os_str())
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        hide_console(&mut audio_command);
        let audio=audio_command.spawn().ok();
        let stop=Arc::new(AtomicBool::new(false));
        self.playback=Some(Playback{stop:stop.clone(),video:video.clone(),audio});
        let frame=self.frame.clone();let pos=self.position.clone();let finished=self.finished.clone();let error=self.error.clone();let ctx=self.ctx.clone();let path=self.path.clone();
        std::thread::spawn(move || {
            let _keep_file=path;
            let mut bytes=vec![0u8;WIDTH*HEIGHT*4];let start=Instant::now();let mut count=0u64;
            while !stop.load(Ordering::Acquire) {
                if stdout.read_exact(&mut bytes).is_err() {break;}
                if stop.load(Ordering::Acquire) {break;}
                let deadline=start+Duration::from_secs_f64(count as f64/FPS as f64);
                while Instant::now()<deadline && !stop.load(Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(5));
                }
                if stop.load(Ordering::Acquire) {break;}
                if let Ok(mut slot)=frame.lock(){*slot=Some(ColorImage::from_rgba_unmultiplied([WIDTH,HEIGHT],&bytes));}
                count+=1;pos.store(offset+count,Ordering::Release);ctx.request_repaint();
            }
            if let Ok(mut child)=video.lock() {
                if stop.load(Ordering::Acquire) {let _=child.kill();}
                let status=child.wait();
                if !stop.load(Ordering::Acquire) {
                    if count==0 || status.is_ok_and(|s|!s.success()) {
                        *error.lock().unwrap()=Some("Không giải mã được video; tệp có thể đã hết hạn hoặc không được hỗ trợ".into());
                    }
                    finished.store(true,Ordering::Release);ctx.request_repaint();
                }
            }
        });
        Ok(())
    }
    pub fn toggle(&mut self)->anyhow::Result<()> {
        if self.playback.is_some() && !self.finished.load(Ordering::Acquire) {self.playback=None;Ok(())}
        else {self.playback=None;self.play()}
    }
    pub fn take_frame(&self)->Option<ColorImage> {self.frame.lock().ok()?.take()}
    pub fn state(&self)->VideoUiState {
        VideoUiState {url:Some(self.url.clone()),playing:self.playback.is_some()&&!self.finished.load(Ordering::Acquire),
            finished:self.finished.load(Ordering::Acquire),error:self.error.lock().ok().and_then(|e|e.clone()),seconds:self.position.load(Ordering::Acquire)/FPS,..Default::default()}
    }
}
impl Drop for InlineVideo {fn drop(&mut self){self.playback=None;}}
fn hide_console(command:&mut Command) {
    #[cfg(target_os="windows")]
    {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    #[cfg(not(target_os="windows"))]let _=command;
}

/// Media is fetched without following redirects or sending credentials to unrelated hosts.
pub fn trusted_media_url(raw:&str)->bool {
    let Ok(url)=reqwest::Url::parse(raw) else{return false;};
    url.scheme()=="https" && url.username().is_empty()&&url.password().is_none()
        && url.port_or_known_default()==Some(443)
        && url.host_str().is_some_and(|host|["zdn.vn","zadn.vn","dlmd.me","zalo.me","zaloapp.com"].iter().any(|suffix|host==*suffix||host.strip_suffix(suffix).is_some_and(|p|p.ends_with('.'))))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn media_hosts_require_https_and_a_real_domain_boundary() {
        assert!(trusted_media_url("https://video.zadn.zdn.vn/v.mp4?token=123"));
        for url in ["http://photo.zdn.vn/p.jpg","https://zdn.vn.evil.test/v.mp4","file:///etc/passwd","https://user:pass@video.zdn.vn/v.mp4"] {assert!(!trusted_media_url(url));}
    }
}

pub fn upgrade_media_url(url:&str)->String {
    let upgraded=url.strip_prefix("http://").map(|s|format!("https://{s}"));
    upgraded.filter(|s|trusted_media_url(s)).unwrap_or_else(||url.to_string())
}
pub async fn media_response(client:&reqwest::Client,raw:&str)->anyhow::Result<reqwest::Response> {
    let mut url=upgrade_media_url(raw);
    for _ in 0..5 {
        anyhow::ensure!(trusted_media_url(&url),"Đường dẫn media không hợp lệ");
        let response=client.get(&url).send().await?;
        if response.status().is_redirection() {
            let location=response.headers().get(reqwest::header::LOCATION).and_then(|h|h.to_str().ok()).ok_or_else(||anyhow::anyhow!("Media thiếu đường dẫn chuyển tiếp"))?;
            url=response.url().join(location)?.to_string();
        } else {return Ok(response.error_for_status()?);}
    }
    anyhow::bail!("Media chuyển tiếp quá nhiều lần")
}
pub async fn download_video(client:&reqwest::Client,url:&str)->anyhow::Result<tempfile::TempPath> {
    use std::io::Write;
    const MAX:u64=256*1024*1024;
    let mut response=media_response(client,url).await?;
    anyhow::ensure!(response.content_length().unwrap_or(0)<=MAX,"Video lớn hơn 256 MB");
    let mut file=tempfile::Builder::new().prefix("ctdozlite-video-").suffix(".media").tempfile()?;
    let mut size=0u64;
    while let Some(chunk)=response.chunk().await? {
        size+=chunk.len() as u64;anyhow::ensure!(size<=MAX,"Video lớn hơn 256 MB");
        file.write_all(&chunk)?;
    }
    anyhow::ensure!(size>0,"Video trống hoặc đã hết hạn");
    file.flush()?;Ok(file.into_temp_path())
}
pub fn decode_image(bytes:&[u8])->anyhow::Result<image::DynamicImage> {
    let mut reader=image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
    let mut limits=image::Limits::default();limits.max_image_width=Some(8192);limits.max_image_height=Some(8192);limits.max_alloc=Some(128*1024*1024);reader.limits(limits);
    Ok(reader.decode()?.thumbnail(1600,1600))
}

pub async fn read_image_bytes(mut response:reqwest::Response)->anyhow::Result<Vec<u8>> {
    const MAX:usize=24*1024*1024;
    anyhow::ensure!(response.content_length().unwrap_or(0)<=MAX as u64,"Ảnh quá lớn");
    let mut bytes=Vec::new();
    while let Some(chunk)=response.chunk().await? {anyhow::ensure!(bytes.len()+chunk.len()<=MAX,"Ảnh quá lớn");bytes.extend_from_slice(&chunk);}
    Ok(bytes)
}

#[cfg(test)]
mod playback_tests {
    use super::*;
    #[test]
    #[ignore="Requires locally installed FFmpeg and an audio backend"]
    fn local_video_decodes_and_pauses_without_opening_an_external_window() {
        let directory=tempfile::tempdir().unwrap();let fixture=directory.path().join("fixture.mp4");
        let result=Command::new("ffmpeg").args(["-nostdin","-v","error","-f","lavfi","-i","testsrc2=size=160x90:rate=24","-f","lavfi","-i","anullsrc=r=48000:cl=stereo","-t","3","-c:v","mpeg4","-c:a","aac","-y"]).arg(&fixture).status().unwrap();assert!(result.success());
        let mut file=tempfile::NamedTempFile::new().unwrap();std::io::copy(&mut std::fs::File::open(fixture).unwrap(),&mut file).unwrap();
        let mut player=InlineVideo::new("fixture".into(),file.into_temp_path(),egui::Context::default()).unwrap();
        let deadline=Instant::now()+Duration::from_secs(3);
        let image=loop {if let Some(image)=player.take_frame(){break image;}assert!(Instant::now()<deadline,"no decoded video frame");std::thread::sleep(Duration::from_millis(20));};
        assert_eq!(image.size,[WIDTH,HEIGHT]);assert!(!player.state().finished);
        player.toggle().unwrap();let paused=player.position.load(Ordering::Acquire);std::thread::sleep(Duration::from_millis(120));assert_eq!(player.position.load(Ordering::Acquire),paused);assert!(!player.state().playing);
        player.toggle().unwrap();let deadline=Instant::now()+Duration::from_secs(3);while player.position.load(Ordering::Acquire)<=paused {assert!(Instant::now()<deadline,"resume failed");std::thread::sleep(Duration::from_millis(20));}
        let path=player.path.to_path_buf();drop(player);
        let deadline=Instant::now()+Duration::from_secs(2);while path.exists(){assert!(Instant::now()<deadline,"private video file not removed");std::thread::sleep(Duration::from_millis(20));}
    }
}
