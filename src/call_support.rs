//! On-demand local capability checks. Never places calls or captures microphone/camera data.
use cpal::traits::{DeviceTrait, HostTrait};
#[derive(Clone, Debug, Default)]
pub struct CallSupport {
    pub microphone: bool,
    pub speaker: bool,
    pub opus: bool,
    pub h264_encoder: bool,
    pub h264_decoder: bool,
    pub camera: bool,
}
impl CallSupport {
    pub fn inspect() -> Self {
        let host = cpal::default_host();
        let microphone = host
            .default_input_device()
            .is_some_and(|d| d.default_input_config().is_ok());
        let speaker = host
            .default_output_device()
            .is_some_and(|d| d.default_output_config().is_ok());
        let opus = opus_roundtrip();
        let encoders = ffmpeg_listing("-encoders");
        let decoders = ffmpeg_listing("-decoders");
        let (_, cameras) = crate::media::list_media_devices(true);
        Self {
            microphone,
            speaker,
            opus,
            h264_encoder: has_codec(&encoders, "libx264"),
            h264_decoder: has_codec(&decoders, "h264"),
            camera: !cameras.is_empty(),
        }
    }
    pub fn ready_voice(&self) -> bool {
        self.microphone && self.speaker && self.opus
    }
    pub fn ready_video(&self) -> bool {
        self.ready_voice() && self.h264_encoder && self.h264_decoder && self.camera
    }
    pub fn checks(&self) -> [(&'static str, bool); 6] {
        [
            ("Micro", self.microphone),
            ("Loa", self.speaker),
            ("Âm thanh cuộc gọi", self.opus),
            ("Gửi video", self.h264_encoder),
            ("Nhận video", self.h264_decoder),
            ("Camera", self.camera),
        ]
    }
}
fn opus_roundtrip() -> bool {
    let Ok(mut encoder) = opus::Encoder::new(16000, opus::Channels::Mono, opus::Application::Voip)
    else {
        return false;
    };
    let Ok(mut decoder) = opus::Decoder::new(16000, opus::Channels::Mono) else {
        return false;
    };
    let mut packet = [0u8; 1500];
    let mut output = [0f32; 320];
    let Ok(bytes) = encoder.encode_float(&[0.0; 320], &mut packet) else {
        return false;
    };
    decoder
        .decode_float(&packet[..bytes], &mut output, false)
        .is_ok_and(|samples| samples == 320)
}
fn ffmpeg_listing(flag: &str) -> String {
    let mut command = std::process::Command::new("ffmpeg");
    command
        .args(["-hide_banner", flag])
        .stdin(std::process::Stdio::null());
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}
fn has_codec(list: &str, codec: &str) -> bool {
    list.lines()
        .any(|line| line.split_whitespace().nth(1) == Some(codec))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn voice_codec_roundtrip_is_available_without_microphone_capture() {
        assert!(opus_roundtrip());
    }

    #[test]
    fn hardware_and_codec_requirements_are_reported_separately() {
        let mut support = CallSupport {
            microphone: true,
            speaker: true,
            opus: true,
            ..Default::default()
        };
        assert!(support.ready_voice());
        assert!(!support.ready_video());
        support.h264_encoder = true;
        support.h264_decoder = true;
        support.camera = true;
        assert!(support.ready_video());
        support.microphone = false;
        assert!(!support.ready_video());
        assert!(has_codec(" V....D libx264 H.264 encoder", "libx264"));
        assert!(!has_codec(" V....D other mentions libx264", "libx264"));
    }
}
