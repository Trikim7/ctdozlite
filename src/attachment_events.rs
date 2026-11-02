//! Bounded upload completions from the authenticated control socket.
use serde_json::Value;
use tokio::sync::broadcast;
#[derive(Clone, Debug)]
pub struct Completed {
    pub owner: String,
    pub file_id: String,
    pub url: String,
}
static EVENTS: std::sync::LazyLock<broadcast::Sender<Completed>> =
    std::sync::LazyLock::new(|| broadcast::channel(32).0);
pub fn subscribe() -> broadcast::Receiver<Completed> {
    EVENTS.subscribe()
}
fn string(v: &Value) -> Option<String> {
    v.as_str()
        .map(str::to_owned)
        .or_else(|| v.as_u64().map(|n| n.to_string()))
}
fn completion(content: &Value) -> Option<(String, String)> {
    if !matches!(
        content.get("act_type")?.as_str()?,
        "file_done" | "video_done"
    ) {
        return None;
    }
    let id = string(content.get("fileId")?)?;
    let data = content.get("data")?;
    let parsed;
    let data = if let Some(text) = data.as_str() {
        if text.len() > 16_384 {
            return None;
        }
        parsed = serde_json::from_str::<Value>(text).ok()?;
        &parsed
    } else {
        data
    };
    let url = data.get("url")?.as_str()?;
    if id.is_empty()
        || id == "-1"
        || url.len() > 8192
        || !crate::inline_video::trusted_media_url(url)
    {
        return None;
    }
    Some((id, url.into()))
}
pub fn observe(owner: &str, payload: &Value) {
    let data = payload.get("data").unwrap_or(payload);
    let Some(controls) = data.get("controls").and_then(Value::as_array) else {
        return;
    };
    for control in controls {
        let Some(content) = control.get("content") else {
            continue;
        };
        let parsed;
        let content = if let Some(text) = content.as_str() {
            if text.len() > 32_768 {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            parsed = v;
            &parsed
        } else {
            content
        };
        if let Some((file_id, url)) = completion(content) {
            let _ = EVENTS.send(Completed {
                owner: owner.into(),
                file_id,
                url,
            });
        }
    }
}
pub async fn wait(
    receiver: &mut broadcast::Receiver<Completed>,
    owner: &str,
    file_id: &str,
    duration: std::time::Duration,
) -> anyhow::Result<String> {
    tokio::time::timeout(duration, async {
        loop {
            match receiver.recv().await {
                Ok(event) if event.owner == owner && event.file_id == file_id => {
                    return Ok(event.url)
                }
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => anyhow::bail!("Kết nối nhận kết quả tải tệp đã đóng"),
            }
        }
    })
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "Zalo chưa trả đường dẫn tệp sau thời gian chờ; tệp chưa được gửi. Hãy thử lại."
        )
    })?
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[tokio::test]
    async fn completion_is_correlated_and_private() {
        let mut receiver = subscribe();
        observe(
            "other",
            &json!({"data":{"controls":[{"content":{"act_type":"file_done","fileId":42,"data":{"url":"https://files.zalo.me/a"}}}]}}),
        );
        observe(
            "owner",
            &json!({"data":{"controls":[{"content":{"act_type":"file_done","fileId":7,"data":{"url":"https://files.zalo.me/wrong"}}},{"content":{"act_type":"file_done","fileId":42,"data":{"url":"https://files.zalo.me/ok"}}}]}}),
        );
        assert_eq!(
            wait(
                &mut receiver,
                "owner",
                "42",
                std::time::Duration::from_millis(50)
            )
            .await
            .unwrap(),
            "https://files.zalo.me/ok"
        );
    }
    #[test]
    fn untrusted_or_unrelated_controls_do_not_complete() {
        assert!(completion(
            &json!({"act_type":"typing","fileId":1,"data":{"url":"https://files.zalo.me/a"}})
        )
        .is_none());
        assert!(completion(
            &json!({"act_type":"file_done","fileId":1,"data":{"url":"https://evil.test/a"}})
        )
        .is_none());
    }
    #[tokio::test]
    async fn missing_completion_times_out() {
        let mut r = subscribe();
        assert!(wait(
            &mut r,
            "missing-owner",
            "missing-file",
            std::time::Duration::from_millis(5)
        )
        .await
        .is_err());
    }
}
