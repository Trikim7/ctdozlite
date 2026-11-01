//! Background backfill of server-available history. This is not phone-to-PC transfer.
use crate::{chat::{self,Thread}, session::Session};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Default)]
pub struct PageCursor { previous: Option<u128>, seen: HashSet<u128> }
impl PageCursor {
    /// The oldest global ID is an exclusive cursor. Stop on empty/repeated/non-decreasing pages.
    pub fn advance(&mut self, response: &Value) -> Option<String> {
        let data=chat::response_payload(response);
        let messages=data.get("groupMsgs").or_else(||data.get("msgs")).or_else(||data.get("messages")).and_then(Value::as_array)?;
        if messages.is_empty() {return None;}
        let oldest=messages.iter().filter_map(|v| v.get("msgId").or_else(||v.get("globalMsgId")))
            .filter_map(|v|v.as_str().and_then(|s|s.parse::<u128>().ok()).or_else(||v.as_u64().map(u128::from)))
            .filter(|id|*id>0).min()?;
        if self.previous.is_some_and(|prev|oldest>=prev) || !self.seen.insert(oldest) {return None;}
        self.previous=Some(oldest);Some(oldest.to_string())
    }
}

/// Stop after a bounded batch; a later run can resume from the oldest locally saved ID.
pub async fn backfill_thread(s: &Session, thread: &mut Thread) -> anyhow::Result<usize> {
    let before=thread.messages.len();
    let recent=chat::get_cloud_messages(s,&thread.id,"0",false,50).await?;
    chat::merge_group_history(thread,&recent);
    let mut cursor=PageCursor::default();
    let Some(mut next)=cursor.advance(&recent) else {return Ok(thread.messages.len().saturating_sub(before));};
    // Loading the server's adjacent pages avoids gaps if local history was incomplete.
    for _ in 0..200 {
        let page=chat::get_cloud_messages(s,&thread.id,&next,true,50).await?;
        chat::merge_group_history(thread,&page);
        let data=chat::response_payload(&page);
        let Some(older)=cursor.advance(&page) else {break;};
        // Persist each page before moving the cursor; interrupted sync keeps its messages.
        let mut store=crate::store::MessageStore::open("zalo_history.db")?;
        crate::store::persist_threads(&mut store,std::slice::from_ref(thread))?;
        drop(store);
        next=older;
        if data.get("hasMore").is_some_and(|v|v.as_bool()==Some(false)||v.as_i64()==Some(0)) {break;}
        tokio::time::sleep(std::time::Duration::from_millis(180)).await;
    }
    chat::normalize_thread_times(std::slice::from_mut(thread));
    Ok(thread.messages.len().saturating_sub(before))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn follows_numeric_global_ids_and_stops_repeated_or_empty_pages() {
        let mut cursor=PageCursor::default();
        assert_eq!(cursor.advance(&json!({"data":{"groupMsgs":[{"msgId":"100"},{"msgId":"99"}]}})),Some("99".into()));
        assert_eq!(cursor.advance(&json!({"data":"{\"groupMsgs\":[{\"msgId\":9}]}"})),Some("9".into()));
        assert_eq!(cursor.advance(&json!({"data":{"groupMsgs":[{"msgId":"9"}]}})),None);
        assert_eq!(cursor.advance(&json!({"data":{"groupMsgs":[]}})),None);
    }
    #[test]
    fn does_not_move_forward_or_use_timestamp_as_history_cursor() {
        let mut cursor=PageCursor::default();
        assert_eq!(cursor.advance(&json!({"data":{"groupMsgs":[{"msgId":"50"}]}})),Some("50".into()));
        assert!(cursor.advance(&json!({"data":{"groupMsgs":[{"msgId":"70"}]}})).is_none());
        assert!(cursor.advance(&json!({"data":{"groupMsgs":[{"ts":1700000000000u64}]}})).is_none());
    }
}
