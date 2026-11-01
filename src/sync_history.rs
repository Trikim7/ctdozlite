//! Background backfill of server-available history. This is not phone-to-PC transfer.
use crate::{
    chat::{self, Thread},
    session::Session,
};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Default)]
pub struct PageCursor {
    previous: Option<u128>,
    seen: HashSet<u128>,
}
impl PageCursor {
    /// The oldest global ID is an exclusive cursor. Stop on empty/repeated/non-decreasing pages.
    pub fn advance(&mut self, response: &Value) -> Option<String> {
        let data = chat::response_payload(response);
        let messages = data
            .get("groupMsgs")
            .or_else(|| data.get("msgs"))
            .or_else(|| data.get("messages"))
            .and_then(Value::as_array)?;
        if messages.is_empty() {
            return None;
        }
        let oldest = messages
            .iter()
            .filter_map(|v| v.get("msgId").or_else(|| v.get("globalMsgId")))
            .filter_map(|v| {
                v.as_str()
                    .and_then(|s| s.parse::<u128>().ok())
                    .or_else(|| v.as_u64().map(u128::from))
            })
            .filter(|id| *id > 0)
            .min()?;
        if self.previous.is_some_and(|prev| oldest >= prev) || !self.seen.insert(oldest) {
            return None;
        }
        self.previous = Some(oldest);
        Some(oldest.to_string())
    }
}

/// Walk adjacent pages until the service stops or repeats its cursor. Each page is saved
/// independently; after interruption the next run safely replays/deduplicates server pages.
pub async fn backfill_thread(
    s: &Session,
    thread: &mut Thread,
    mut notify: impl FnMut(usize) + Send,
) -> anyhow::Result<usize> {
    let before = thread.messages.len();
    let recent = chat::get_cloud_messages(s, &thread.id, "0", false, 50).await?;
    validate_page(&recent)?;
    save_page(thread, &recent)?;
    chat::merge_group_history(thread, &recent);
    notify(thread.messages.len().saturating_sub(before));
    if exhausted(&recent) {
        return Ok(thread.messages.len().saturating_sub(before));
    }
    let mut cursor = PageCursor::default();
    let Some(mut next) = cursor.advance(&recent) else {
        return Ok(thread.messages.len().saturating_sub(before));
    };
    // Loading the server's adjacent pages avoids gaps if local history was incomplete.
    loop {
        let page = chat::get_cloud_messages(s, &thread.id, &next, true, 50).await?;
        validate_page(&page)?;
        save_page(thread, &page)?;
        chat::merge_group_history(thread, &page);
        notify(thread.messages.len().saturating_sub(before));
        let Some(older) = cursor.advance(&page) else {
            break;
        };
        next = older;
        if exhausted(&page) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(180)).await;
    }
    chat::normalize_thread_times(std::slice::from_mut(thread));
    Ok(thread.messages.len().saturating_sub(before))
}

fn exhausted(response: &Value) -> bool {
    chat::response_payload(response)
        .get("hasMore")
        .is_some_and(|v| v.as_bool() == Some(false) || v.as_i64() == Some(0))
}

fn validate_page(response: &Value) -> anyhow::Result<()> {
    if response
        .get("error_code")
        .and_then(Value::as_i64)
        .is_some_and(|code| code != 0)
    {
        anyhow::bail!("Máy chủ từ chối tải lịch sử");
    }
    let data = chat::response_payload(response);
    if !["groupMsgs", "msgs", "messages"]
        .iter()
        .any(|field| data.get(field).is_some_and(Value::is_array))
    {
        anyhow::bail!("Phản hồi lịch sử không có danh sách tin nhắn");
    }
    Ok(())
}

fn save_page(thread: &Thread, response: &Value) -> anyhow::Result<()> {
    let mut delta = Thread {
        id: thread.id.clone(),
        name: thread.name.clone(),
        preview: String::new(),
        time: String::new(),
        avatar: thread.avatar.clone(),
        is_group: thread.is_group,
        messages: Vec::new(),
    };
    chat::merge_group_history(&mut delta, response);
    if !delta.messages.is_empty() {
        let mut store = crate::store::MessageStore::open("zalo_history.db")?;
        crate::store::persist_threads(&mut store, &[delta])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn distinguishes_empty_history_from_failed_request() {
        assert!(
            validate_page(&json!({"error_code":0,"data":{"groupMsgs":[],"hasMore":false}})).is_ok()
        );
        assert!(exhausted(&json!({"data":{"hasMore":0}})));
        assert!(validate_page(&json!({"error_code":-1,"data":{}})).is_err());
        assert!(validate_page(&json!({"data":null})).is_err());
    }
    #[test]
    fn follows_numeric_global_ids_and_stops_repeated_or_empty_pages() {
        let mut cursor = PageCursor::default();
        assert_eq!(
            cursor.advance(&json!({"data":{"groupMsgs":[{"msgId":"100"},{"msgId":"99"}]}})),
            Some("99".into())
        );
        assert_eq!(
            cursor.advance(&json!({"data":"{\"groupMsgs\":[{\"msgId\":9}]}"})),
            Some("9".into())
        );
        assert_eq!(
            cursor.advance(&json!({"data":{"groupMsgs":[{"msgId":"9"}]}})),
            None
        );
        assert_eq!(cursor.advance(&json!({"data":{"groupMsgs":[]}})), None);
    }
    #[test]
    fn does_not_move_forward_or_use_timestamp_as_history_cursor() {
        let mut cursor = PageCursor::default();
        assert_eq!(
            cursor.advance(&json!({"data":{"groupMsgs":[{"msgId":"50"}]}})),
            Some("50".into())
        );
        assert!(cursor
            .advance(&json!({"data":{"groupMsgs":[{"msgId":"70"}]}}))
            .is_none());
        assert!(cursor
            .advance(&json!({"data":{"groupMsgs":[{"ts":1700000000000u64}]}}))
            .is_none());
    }
}
