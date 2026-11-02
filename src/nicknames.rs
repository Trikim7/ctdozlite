//! Read-only Zalo aliases; kept separately from canonical contact identity and encrypted at rest.
use crate::{history_crypto::DataCipher, session::Session};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, path::Path};
const CACHE: &str = "nicknames.cache";
const MAX_BYTES: u64 = 4 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
struct Snapshot {
    owner: String,
    aliases: HashMap<String, String>,
}
pub fn display_name<'a>(
    aliases: &'a HashMap<String, String>,
    thread: &'a crate::chat::Thread,
) -> &'a str {
    if thread.is_group {
        &thread.name
    } else {
        aliases
            .get(&thread.id)
            .map(String::as_str)
            .unwrap_or(&thread.name)
    }
}
pub async fn fetch(session: &Session) -> Result<HashMap<String, String>> {
    ensure!(session.login.is_some(), "Chưa đăng nhập");
    let mut last_error = None;
    for endpoint in ["/api/alias/list/v2", "/api/alias/list"] {
        match fetch_pages(session, endpoint).await {
            Ok(aliases) => return Ok(aliases),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap())
}
async fn fetch_pages(s: &Session, endpoint: &str) -> Result<HashMap<String, String>> {
    let mut all = HashMap::new();
    let mut previous = None;
    for page in 1..=1000 {
        let params = s.encoded_params(&json!({"page":page,"count":20,"src":-1,"imei":s.imei}))?;
        let response = s
            .get(&format!(
                "{}{endpoint}?{}&params={params}",
                s.domains.alias,
                s.common_params()
            ))
            .await?;
        let decoded = s.maybe_decrypt(response, s.session_key_bytes()).await;
        if let Some(code) = decoded.get("error_code").and_then(Value::as_i64) {
            ensure!(code == 0, "Không tải được biệt danh (mã {code})");
        }
        let data = crate::chat::response_payload(&decoded);
        let (items, more) = parse_page(&data)?;
        let fingerprint = serde_json::to_string(&data["items"])?;
        ensure!(
            previous.as_deref() != Some(fingerprint.as_str()) || !more,
            "Máy chủ lặp lại trang biệt danh"
        );
        previous = Some(fingerprint);
        all.extend(items);
        ensure!(all.len() <= 20000, "Danh sách biệt danh quá lớn");
        if !more {
            return Ok(all);
        }
    }
    anyhow::bail!("Chưa tải hết các trang biệt danh")
}
fn parse_page(value: &Value) -> Result<(HashMap<String, String>, bool)> {
    let rows = value
        .get("items")
        .and_then(Value::as_array)
        .context("Phản hồi biệt danh không hợp lệ")?;
    let mut aliases = HashMap::new();
    for row in rows {
        let uid = row.get("userId").and_then(|v| {
            v.as_str()
                .map(str::to_owned)
                .or_else(|| v.as_u64().map(|n| n.to_string()))
        });
        if let (Some(uid), Some(alias)) = (uid, row.get("alias").and_then(Value::as_str)) {
            let alias = alias.trim();
            if !uid.is_empty()
                && !alias.is_empty()
                && uid.len() <= 64
                && alias.chars().count() <= 256
            {
                aliases.insert(uid, alias.to_string());
            }
        }
    }
    let more = value
        .get("hasMore")
        .and_then(|v| v.as_bool().or_else(|| v.as_u64().map(|n| n != 0)))
        .unwrap_or(rows.len() == 20);
    Ok((aliases, more))
}
fn read_cache(path: &Path) -> Result<Option<String>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file()
                    && !metadata.file_type().is_symlink()
                    && metadata.len() <= MAX_BYTES,
                "Cache biệt danh không hợp lệ"
            );
            Ok(Some(std::fs::read_to_string(path)?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
pub fn load(owner: &str) -> HashMap<String, String> {
    let result = (|| -> Result<HashMap<String, String>> {
        let path = Path::new(CACHE);
        let Some(encrypted) = read_cache(path)? else {
            return Ok(HashMap::new());
        };
        let cipher = DataCipher::for_path(path, false)?;
        let plain = zeroize::Zeroizing::new(cipher.decrypt("cache", "nicknames", &encrypted)?);
        let snapshot: Snapshot = serde_json::from_str(&plain)?;
        Ok(if snapshot.owner == owner {
            snapshot.aliases
        } else {
            HashMap::new()
        })
    })();
    result.unwrap_or_default()
}
pub fn save(owner: &str, aliases: &HashMap<String, String>) -> Result<()> {
    let path = Path::new(CACHE);
    let _lock = crate::secure_session::lock_session_file(&path.with_extension("io"))?;
    let previous = read_cache(path)?;
    let cipher = DataCipher::for_path(path, previous.is_none())?;
    if let Some(previous) = previous {
        let plain = zeroize::Zeroizing::new(cipher.decrypt("cache", "nicknames", &previous)?);
        let _: Snapshot = serde_json::from_str(&plain)?;
    }
    let plain = zeroize::Zeroizing::new(serde_json::to_string(&Snapshot {
        owner: owner.into(),
        aliases: aliases.clone(),
    })?);
    let encrypted = cipher.encrypt("cache", "nicknames", &plain)?;
    ensure!(
        encrypted.len() as u64 <= MAX_BYTES,
        "Cache biệt danh quá lớn"
    );
    crate::storage::atomic_write(path, encrypted.as_bytes())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alias_pages_keep_large_ids_and_do_not_treat_errors_as_empty_lists() {
        let (aliases,more)=parse_page(&json!({"items":[{"userId":"900719925474099199","alias":"Anh Hai"},{"userId":123,"alias":""}],"hasMore":true})).unwrap();
        assert_eq!(aliases["900719925474099199"], "Anh Hai");
        assert!(more);
        assert_eq!(aliases.len(), 1);
        assert!(parse_page(&json!({"error":-69})).is_err());
        assert_eq!(
            parse_page(&json!({"items":[],"hasMore":false})).unwrap(),
            (HashMap::new(), false)
        );
    }
    #[test]
    fn shared_aliases_do_not_change_canonical_names_or_contact_ids() {
        let aliases = HashMap::from([
            ("a".into(), "Anh Hai".into()),
            ("b".into(), "Anh Hai".into()),
        ]);
        let first = crate::chat::Thread {
            id: "a".into(),
            name: "Tên gốc A".into(),
            preview: String::new(),
            time: String::new(),
            avatar: None,
            is_group: false,
            messages: Vec::new(),
        };
        let mut second = first.clone();
        second.id = "b".into();
        second.name = "Tên gốc B".into();
        assert_eq!(display_name(&aliases, &first), "Anh Hai");
        assert_eq!(display_name(&aliases, &second), "Anh Hai");
        assert_ne!(first.id, second.id);
        assert_eq!(first.name, "Tên gốc A");
        second.is_group = true;
        assert_eq!(display_name(&aliases, &second), "Tên gốc B");
    }
}
