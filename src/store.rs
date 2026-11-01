//! Local message and thread store backed by SQLite (rusqlite bundled).
//!
//! Persists every message seen via polling or group history so chat history accumulates locally.

use crate::history_crypto::DataCipher;
use anyhow::{bail, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::chat::{ChatMessage, Thread};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredMessage {
    pub msg_id: String,
    pub thread_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub text: String,
    pub msg_type: String,
    pub ts: i64,
    pub is_outgoing: bool,
    pub raw_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredThread {
    pub thread_id: String,
    pub name: String,
    pub is_group: bool,
    pub last_ts: i64,
}

pub struct MessageStore {
    conn: Connection,
    cipher: DataCipher,
}

impl MessageStore {
    /// Open SQLite database at `path` and initialize table schemas and indices if absent.
    pub fn open(path: &str) -> Result<Self> {
        let result = Self::open_secure(path);
        if result.is_err() {
            crate::logger::report_history_error();
        }
        result
    }

    fn open_secure(path: &str) -> Result<Self> {
        if path == ":memory:" {
            return Self::initialize(Connection::open_in_memory()?, DataCipher::random());
        }
        let path = Path::new(path);
        let _migration_lock =
            crate::secure_session::lock_session_file(&path.with_extension("migration"))?;
        if let Ok(meta) = std::fs::symlink_metadata(path) {
            if !meta.is_file() || meta.file_type().is_symlink() {
                bail!("Tệp lịch sử không hợp lệ");
            }
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(15))?;
        let marked: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='history_encryption')",
            [],
            |row| row.get(0),
        )?;
        let cipher = DataCipher::for_path(path, !marked)?;
        Self::initialize(conn, cipher)
    }

    fn initialize(conn: Connection, cipher: DataCipher) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_secs(15))?;
        conn.execute_batch(
            "PRAGMA secure_delete=ON; PRAGMA temp_store=MEMORY; PRAGMA journal_mode=DELETE;",
        )?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                msg_id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                sender_id TEXT,
                sender_name TEXT,
                text TEXT,
                msg_type TEXT,
                ts INTEGER,
                is_outgoing INTEGER,
                raw_json TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_thread_ts ON messages(thread_id, ts);
            CREATE TABLE IF NOT EXISTS threads (
                thread_id TEXT PRIMARY KEY,
                name TEXT,
                is_group INTEGER,
                last_ts INTEGER
            );",
        )?;

        let store = Self { conn, cipher };
        store.migrate_history()?;
        Ok(store)
    }

    fn crypt_message(&self, mut msg: StoredMessage, encrypt: bool) -> Result<StoredMessage> {
        let id = msg.msg_id.clone();
        for (column, value) in [
            ("sender_id", &mut msg.sender_id),
            ("sender_name", &mut msg.sender_name),
            ("text", &mut msg.text),
            ("msg_type", &mut msg.msg_type),
            ("raw_json", &mut msg.raw_json),
        ] {
            *value = if encrypt {
                self.cipher.encrypt(column, &id, value)?
            } else {
                self.cipher.decrypt(column, &id, value)?
            };
        }
        Ok(msg)
    }

    fn migrate_history(&self) -> Result<()> {
        let marked: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='history_encryption')",
            [],
            |row| row.get(0),
        )?;
        if marked {
            let (version, check, compacted): (i64, String, bool) = self.conn.query_row(
                "SELECT version, key_check, compacted FROM history_encryption WHERE id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
            if version != 1
                || self.cipher.decrypt("metadata", "key-check", &check)? != "ctdozlite-history-v1"
            {
                bail!("Không xác thực được định dạng/khóa lịch sử");
            }
            if !compacted {
                self.finish_migration()?;
            }
            return Ok(());
        }
        let tx = self.conn.unchecked_transaction()?;
        let mut last_row = i64::MIN;
        loop {
            let batch: Vec<(i64, StoredMessage)> = {
                let mut stmt = tx.prepare("SELECT rowid, msg_id, thread_id, sender_id, sender_name, text, msg_type, ts, is_outgoing, raw_json FROM messages WHERE rowid > ?1 ORDER BY rowid LIMIT 128")?;
                let rows = stmt.query_map([last_row], |r| {
                    Ok((
                        r.get(0)?,
                        StoredMessage {
                            msg_id: r.get(1)?,
                            thread_id: r.get(2)?,
                            sender_id: r.get(3).unwrap_or_default(),
                            sender_name: r.get(4).unwrap_or_default(),
                            text: r.get(5).unwrap_or_default(),
                            msg_type: r.get(6).unwrap_or_default(),
                            ts: r.get(7)?,
                            is_outgoing: r.get::<_, i64>(8)? != 0,
                            raw_json: r.get(9).unwrap_or_default(),
                        },
                    ))
                })?;
                rows.collect::<rusqlite::Result<_>>()?
            };
            if batch.is_empty() {
                break;
            }
            for (rowid, msg) in batch {
                last_row = rowid;
                let m = self.crypt_message(msg, true)?;
                tx.execute("UPDATE messages SET sender_id=?1, sender_name=?2, text=?3, msg_type=?4, raw_json=?5 WHERE msg_id=?6",
                    params![m.sender_id,m.sender_name,m.text,m.msg_type,m.raw_json,m.msg_id])?;
            }
        }
        let threads: Vec<(String, String)> = {
            let mut stmt = tx.prepare("SELECT thread_id, name FROM threads")?;
            let rows =
                stmt.query_map([], |row| Ok((row.get(0)?, row.get(1).unwrap_or_default())))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for (id, name) in threads {
            tx.execute(
                "UPDATE threads SET name=?1 WHERE thread_id=?2",
                params![self.cipher.encrypt("thread_name", &id, &name)?, id],
            )?;
        }
        tx.execute_batch("CREATE TABLE history_encryption(id INTEGER PRIMARY KEY CHECK(id=1), version INTEGER NOT NULL, key_check TEXT NOT NULL, compacted INTEGER NOT NULL);")?;
        tx.execute(
            "INSERT INTO history_encryption VALUES(1,1,?1,0)",
            [self
                .cipher
                .encrypt("metadata", "key-check", "ctdozlite-history-v1")?],
        )?;
        tx.commit()?;
        self.finish_migration()
    }

    fn finish_migration(&self) -> Result<()> {
        // Remove legacy plaintext from free SQLite pages. This cannot erase old SSD blocks/backups.
        // Persist the completion bit only afterwards, so an interrupted vacuum is retried on open.
        self.conn
            .execute_batch("VACUUM; UPDATE history_encryption SET compacted=1 WHERE id=1;")?;
        Ok(())
    }

    /// Insert a message. Returns `true` if a new row was inserted, `false` if already existed.
    pub fn upsert_message(&self, msg: &StoredMessage) -> Result<bool> {
        let encoded = self.crypt_message(msg.clone(), true)?;
        let rows = self.conn.execute(
            "INSERT OR IGNORE INTO messages (msg_id, thread_id, sender_id, sender_name, text, msg_type, ts, is_outgoing, raw_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                msg.msg_id,
                msg.thread_id,
                encoded.sender_id,
                encoded.sender_name,
                encoded.text,
                encoded.msg_type,
                msg.ts,
                if msg.is_outgoing { 1 } else { 0 },
                encoded.raw_json,
            ],
        )?;
        if rows == 0 && !msg.raw_json.is_empty() {
            self.conn.execute(
                "UPDATE messages SET raw_json = ?1, msg_type = ?2, text = ?4 WHERE msg_id = ?3",
                params![encoded.raw_json, encoded.msg_type, msg.msg_id, encoded.text],
            )?;
        }
        Ok(rows > 0)
    }

    /// Delete a message from local SQLite store when user selects "Xóa chỉ ở phía tôi".
    pub fn delete_message(&mut self, thread_id: &str, msg_id: &str, ts: i64) -> Result<()> {
        if !msg_id.is_empty() {
            let _ = self
                .conn
                .execute("DELETE FROM messages WHERE msg_id = ?1", params![msg_id]);
        }
        if ts > 0 && !thread_id.is_empty() {
            let _ = self.conn.execute(
                "DELETE FROM messages WHERE thread_id = ?1 AND ts = ?2",
                params![thread_id, ts],
            );
        }
        Ok(())
    }

    /// Insert or update thread metadata.
    pub fn upsert_thread(&self, t: &StoredThread) -> Result<()> {
        self.conn.execute(
            "INSERT INTO threads (thread_id, name, is_group, last_ts)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(thread_id) DO UPDATE SET
                 name = excluded.name,
                 is_group = excluded.is_group,
                 last_ts = MAX(threads.last_ts, excluded.last_ts)",
            params![
                t.thread_id,
                self.cipher.encrypt("thread_name", &t.thread_id, &t.name)?,
                if t.is_group { 1 } else { 0 },
                t.last_ts,
            ],
        )?;
        Ok(())
    }

    /// Retrieve the latest `limit` messages, returned in chronological order.
    pub fn messages_for(&self, thread_id: &str, limit: usize) -> Result<Vec<StoredMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT msg_id, thread_id, sender_id, sender_name, text, msg_type, ts, is_outgoing, raw_json
             FROM messages
             WHERE thread_id = ?1
             ORDER BY ts DESC, msg_id DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![thread_id, limit as i64], |row| {
            let is_outgoing_num: i64 = row.get(7)?;
            Ok(StoredMessage {
                msg_id: row.get(0)?,
                thread_id: row.get(1)?,
                sender_id: row.get(2).unwrap_or_default(),
                sender_name: row.get(3).unwrap_or_default(),
                text: row.get(4).unwrap_or_default(),
                msg_type: row.get(5).unwrap_or_default(),
                ts: row.get(6)?,
                is_outgoing: is_outgoing_num != 0,
                raw_json: row.get(8).unwrap_or_default(),
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(self.crypt_message(r?, false)?);
        }
        list.reverse();
        Ok(list)
    }

    /// Retrieve all persisted thread metadata ordered by `last_ts` descending.
    pub fn all_threads(&self) -> Result<Vec<StoredThread>> {
        let mut stmt = self.conn.prepare(
            "SELECT thread_id, name, is_group, last_ts
             FROM threads
             ORDER BY last_ts DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let is_group_num: i64 = row.get(2).unwrap_or(0);
            Ok(StoredThread {
                thread_id: row.get(0)?,
                name: row.get(1).unwrap_or_default(),
                is_group: is_group_num != 0,
                last_ts: row.get(3).unwrap_or(0),
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            let mut thread = r?;
            thread.name = self
                .cipher
                .decrypt("thread_name", &thread.thread_id, &thread.name)?;
            out.push(thread);
        }
        Ok(out)
    }

    /// Hydrate `threads` with persisted threads and message history from SQLite.
    /// Returns the total number of messages loaded into `threads`.
    pub fn hydrate_threads(
        &self,
        threads: &mut Vec<Thread>,
        limit_per_thread: usize,
    ) -> Result<usize> {
        crate::chat::deduplicate_threads(threads, None);

        let stored_threads = self.all_threads()?;
        for st in stored_threads {
            if st.thread_id.is_empty() || st.thread_id == "0" {
                continue;
            }
            if let Some(existing) = threads.iter_mut().find(|t| t.id == st.thread_id) {
                if crate::chat::is_generic_thread_name(&existing.name) && !st.name.is_empty() {
                    existing.name = st.name;
                }
                existing.is_group = existing.is_group || st.is_group;
            } else if !crate::chat::is_generic_thread_name(&st.name)
                && threads
                    .iter()
                    .any(|t| t.is_group == st.is_group && t.name.trim() == st.name.trim())
            {
                // Migrate stale session-scoped `thread_id` in SQLite to the active contact's `id`
                if let Some(live_id) = threads
                    .iter()
                    .find(|t| t.is_group == st.is_group && t.name.trim() == st.name.trim())
                    .map(|t| t.id.clone())
                {
                    let _ = self.conn.execute(
                        "UPDATE messages SET thread_id = ?1 WHERE thread_id = ?2",
                        params![live_id, st.thread_id],
                    );
                    let _ = self.conn.execute(
                        "DELETE FROM threads WHERE thread_id = ?1",
                        params![st.thread_id],
                    );
                }
            } else {
                threads.push(Thread {
                    id: st.thread_id.clone(),
                    name: if st.name.is_empty() {
                        format!("Người dùng {}", st.thread_id)
                    } else {
                        st.name
                    },
                    preview: String::new(),
                    time: String::new(),
                    avatar: None,
                    is_group: st.is_group,
                    messages: Vec::new(),
                });
            }
        }

        let mut msg_owner: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        if let Ok(mut stmt) = self.conn.prepare("SELECT msg_id, thread_id FROM messages") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                for r in rows.flatten() {
                    msg_owner.insert(r.0, r.1);
                }
            }
        }

        let mut loaded_count = 0;
        for t in threads.iter_mut() {
            let before = t.messages.len();
            t.messages.retain(|m| {
                if let Some(owner_tid) = msg_owner.get(&m.id) {
                    owner_tid == &t.id
                } else {
                    true
                }
            });
            if t.messages.len() != before && t.messages.is_empty() {
                t.preview.clear();
                t.time.clear();
                let _ = self.conn.execute(
                    "UPDATE threads SET last_ts = 0 WHERE thread_id = ?1",
                    params![t.id],
                );
            }
            let saved_msgs = self.messages_for(&t.id, limit_per_thread)?;
            for sm in saved_msgs {
                if sm.text == "sendBubbleMessage" {
                    self.conn
                        .execute("DELETE FROM messages WHERE msg_id=?1", [&sm.msg_id])?;
                    continue;
                }
                let time = if sm.ts > 0 {
                    crate::ui::time_hm(sm.ts)
                } else {
                    crate::ui::clock()
                };
                let mut sticker_id = None;
                let mut sticker_cate_id = None;
                let mut sticker_type = None;
                let mut reply_to_id = None;
                let mut reply_to_sender = None;
                let mut reply_to_text = None;
                let mut reactions = Vec::new();
                let mut is_pinned = false;
                let mut is_starred = false;
                let mut link_url = None;
                let mut link_title = None;
                let mut link_desc = None;
                let mut link_thumb = None;
                let mut link_src = None;
                if !sm.raw_json.is_empty() {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&sm.raw_json) {
                        sticker_id = val
                            .get("stickerId")
                            .or_else(|| val.get("id"))
                            .and_then(|v| v.as_u64());
                        sticker_cate_id = val
                            .get("cateId")
                            .or_else(|| val.get("catId"))
                            .and_then(|v| v.as_u64());
                        sticker_type = val.get("type").and_then(|v| v.as_u64()).map(|n| n as u32);
                        reply_to_id = val
                            .get("reply_to_id")
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                        reply_to_sender = val
                            .get("reply_to_sender")
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                        reply_to_text = val
                            .get("reply_to_text")
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                        if let Some(arr) = val.get("reactions").and_then(|v| v.as_array()) {
                            reactions = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect();
                        }
                        is_pinned = val
                            .get("is_pinned")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        is_starred = val
                            .get("is_starred")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        link_url = val
                            .get("link_url")
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                        link_title = val
                            .get("link_title")
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                        link_desc = val
                            .get("link_desc")
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                        link_thumb = val
                            .get("link_thumb")
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                        link_src = val
                            .get("link_src")
                            .and_then(|v| v.as_str())
                            .map(str::to_string);
                    }
                }
                let cm = ChatMessage {
                    id: sm.msg_id,
                    sender: if !sm.sender_name.is_empty() {
                        sm.sender_name
                    } else {
                        sm.sender_id
                    },
                    text: sm.text,
                    time,
                    is_outgoing: sm.is_outgoing,
                    ts: sm.ts,
                    sticker_id,
                    sticker_cate_id,
                    sticker_type,
                    reply_to_id,
                    reply_to_sender,
                    reply_to_text,
                    reactions,
                    is_pinned,
                    is_starred,
                    link_url,
                    link_title,
                    link_desc,
                    link_thumb,
                    link_src,
                };
                if let Some(existing) = t.messages.iter_mut().find(|existing| {
                    (!existing.id.is_empty() && existing.id == cm.id)
                        || (existing.text == cm.text
                            && existing.is_outgoing == cm.is_outgoing
                            && (existing.ts == cm.ts
                                || (existing.ts != 0
                                    && cm.ts != 0
                                    && (existing.ts - cm.ts).abs() < 5000)))
                }) {
                    if existing.sticker_id.is_none() && cm.sticker_id.is_some() {
                        existing.sticker_id = cm.sticker_id;
                        existing.sticker_cate_id = cm.sticker_cate_id;
                        existing.sticker_type = cm.sticker_type;
                    }
                    if existing.reply_to_text.is_none() && cm.reply_to_text.is_some() {
                        existing.reply_to_id = cm.reply_to_id;
                        existing.reply_to_sender = cm.reply_to_sender;
                        existing.reply_to_text = cm.reply_to_text;
                    }
                    if existing.link_url.is_none() && cm.link_url.is_some() {
                        existing.link_url = cm.link_url;
                        existing.link_title = cm.link_title;
                        existing.link_desc = cm.link_desc;
                        existing.link_thumb = cm.link_thumb;
                        existing.link_src = cm.link_src;
                        if !cm.text.is_empty() {
                            existing.text = cm.text;
                        }
                    }
                    if existing.reactions.is_empty() && !cm.reactions.is_empty() {
                        existing.reactions = cm.reactions;
                    }
                    existing.is_pinned = existing.is_pinned || cm.is_pinned;
                    existing.is_starred = existing.is_starred || cm.is_starred;
                } else {
                    t.messages.push(cm);
                    loaded_count += 1;
                }
            }
            if !t.messages.is_empty() {
                t.messages.sort_by_key(|m| m.ts);
                if let Some(newest) = t.messages.last() {
                    if !newest.text.is_empty() {
                        t.preview = newest.text.replace('\n', " ");
                        t.time = newest.time.clone();
                    }
                }
            }
        }
        crate::chat::deduplicate_threads(threads, None);
        Ok(loaded_count)
    }
}

impl From<&ChatMessage> for StoredMessage {
    fn from(m: &ChatMessage) -> Self {
        let mut meta = serde_json::Map::new();
        let msg_type = if let Some(sid) = m.sticker_id {
            let cid = m.sticker_cate_id.unwrap_or(0);
            let st = m.sticker_type.unwrap_or(7);
            meta.insert("stickerId".into(), serde_json::json!(sid));
            meta.insert("cateId".into(), serde_json::json!(cid));
            meta.insert("type".into(), serde_json::json!(st));
            "chat.sticker".to_string()
        } else {
            "chat.text".to_string()
        };
        if let Some(ref rid) = m.reply_to_id {
            meta.insert("reply_to_id".into(), serde_json::json!(rid));
        }
        if let Some(ref rsender) = m.reply_to_sender {
            meta.insert("reply_to_sender".into(), serde_json::json!(rsender));
        }
        if let Some(ref rtext) = m.reply_to_text {
            meta.insert("reply_to_text".into(), serde_json::json!(rtext));
        }
        if !m.reactions.is_empty() {
            meta.insert("reactions".into(), serde_json::json!(m.reactions));
        }
        if m.is_pinned {
            meta.insert("is_pinned".into(), serde_json::json!(true));
        }
        if m.is_starred {
            meta.insert("is_starred".into(), serde_json::json!(true));
        }
        if let Some(lurl) = &m.link_url {
            meta.insert("link_url".into(), serde_json::json!(lurl));
        }
        if let Some(ltitle) = &m.link_title {
            meta.insert("link_title".into(), serde_json::json!(ltitle));
        }
        if let Some(ldesc) = &m.link_desc {
            meta.insert("link_desc".into(), serde_json::json!(ldesc));
        }
        if let Some(lthumb) = &m.link_thumb {
            meta.insert("link_thumb".into(), serde_json::json!(lthumb));
        }
        if let Some(lsrc) = &m.link_src {
            meta.insert("link_src".into(), serde_json::json!(lsrc));
        }
        let raw_json = if meta.is_empty() {
            String::new()
        } else {
            serde_json::Value::Object(meta).to_string()
        };
        Self {
            msg_id: if !m.id.is_empty() {
                m.id.clone()
            } else {
                format!("msg_{}", m.ts)
            },
            thread_id: String::new(),
            sender_id: if m.is_outgoing {
                "0".into()
            } else {
                m.sender.clone()
            },
            sender_name: m.sender.clone(),
            text: m.text.clone(),
            msg_type,
            ts: m.ts,
            is_outgoing: m.is_outgoing,
            raw_json,
        }
    }
}

/// Persist threads and their messages to SQLite store, returning the count of newly inserted messages.
pub fn persist_threads(store: &mut MessageStore, threads: &[Thread]) -> Result<usize> {
    let result = (|| {
        let transaction=rusqlite::Transaction::new_unchecked(&store.conn,rusqlite::TransactionBehavior::Immediate)?;
        let inserted=persist_threads_inner(store,threads)?;
        transaction.commit()?;
        Ok(inserted)
    })();
    if result.is_err() {
        crate::logger::report_history_error();
    }
    result
}

fn persist_threads_inner(store: &MessageStore, threads: &[Thread]) -> Result<usize> {
    let mut new_count = 0;
    for t in threads {
        let last_ts = t.messages.iter().map(|m| m.ts).max().unwrap_or(0);
        let st = StoredThread {
            thread_id: t.id.clone(),
            name: t.name.clone(),
            is_group: t.is_group,
            last_ts,
        };
        store.upsert_thread(&st)?;

        for m in &t.messages {
            let mut sm = StoredMessage::from(m);
            sm.thread_id = t.id.clone();
            if sm.msg_id.is_empty() {
                sm.msg_id = format!("{}_{}", t.id, sm.ts);
            }
            if store.upsert_message(&sm)? {
                new_count += 1;
            }
        }
    }
    Ok(new_count)
}

/// Convenience helper: open `zalo_history.db` (or `path`) and hydrate `threads` with saved history.
pub fn load_history_into_threads(path: &str, threads: &mut Vec<Thread>, limit_per_thread: usize) {
    let result = (|| -> Result<()> {
        let store = MessageStore::open(path)?;
        let mut candidate = threads.clone();
        store.hydrate_threads(&mut candidate, limit_per_thread)?;
        *threads = candidate;
        Ok(())
    })();
    if result.is_err() {
        crate::logger::report_history_error();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_database(path: &std::path::Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(r#"CREATE TABLE messages(msg_id TEXT PRIMARY KEY, thread_id TEXT NOT NULL, sender_id TEXT, sender_name TEXT, text TEXT, msg_type TEXT, ts INTEGER, is_outgoing INTEGER, raw_json TEXT);
            CREATE TABLE threads(thread_id TEXT PRIMARY KEY, name TEXT, is_group INTEGER, last_ts INTEGER);
            INSERT INTO messages VALUES('m1','t1','PRIVATE_ID','PRIVATE_SENDER','Nội dung riêng tư ONE','chat.text',1,0,'{"reply":"PRIVATE_REPLY"}');
            INSERT INTO messages VALUES('m2','t1','PRIVATE_ID','PRIVATE_SENDER','Nội dung riêng tư TWO','chat.text',2,0,'');
            INSERT INTO threads VALUES('t1','PRIVATE_THREAD',0,2);"#).unwrap();
        conn
    }

    #[test]
    fn migrates_existing_sqlite_and_removes_plaintext_from_active_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        let conn = legacy_database(&path);
        let mut store = MessageStore::initialize(conn, DataCipher::test_key(42)).unwrap();
        let before = store.messages_for("t1", 10).unwrap();
        assert_eq!(before.len(), 2);
        assert_eq!(before[0].text, "Nội dung riêng tư ONE");
        assert_eq!(before[0].raw_json, r#"{"reply":"PRIVATE_REPLY"}"#);
        assert_eq!(store.all_threads().unwrap()[0].name, "PRIVATE_THREAD");
        let mut changed = before[0].clone();
        changed.text = "UPDATED_PRIVATE_TEXT".into();
        changed.raw_json = r#"{"quote":"UPDATED_PRIVATE_QUOTE"}"#.into();
        assert!(!store.upsert_message(&changed).unwrap());
        assert_eq!(store.messages_for("t1", 10).unwrap()[0], changed);
        let ciphertext: String = store
            .conn
            .query_row("SELECT text FROM messages WHERE msg_id='m1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(ciphertext.starts_with("ctdoenc:v1:"));
        drop(store);
        for file in std::fs::read_dir(dir.path()).unwrap() {
            let bytes = std::fs::read(file.unwrap().path()).unwrap();
            for secret in [
                "PRIVATE_ID",
                "PRIVATE_SENDER",
                "PRIVATE_THREAD",
                "PRIVATE_REPLY",
                "Nội dung riêng tư",
                "UPDATED_PRIVATE_TEXT",
                "UPDATED_PRIVATE_QUOTE",
            ] {
                assert!(
                    !bytes.windows(secret.len()).any(|w| w == secret.as_bytes()),
                    "{secret}"
                );
            }
        }
        let store =
            MessageStore::initialize(Connection::open(&path).unwrap(), DataCipher::test_key(42))
                .unwrap();
        assert_eq!(store.messages_for("t1", 10).unwrap()[0], changed);
        assert!(MessageStore::initialize(
            Connection::open(&path).unwrap(),
            DataCipher::test_key(43)
        )
        .is_err());
        assert_eq!(store.messages_for("t1", 10).unwrap().len(), 2);
    }

    #[test]
    fn failed_migration_rolls_back_and_can_be_retried() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        let conn = legacy_database(&path);
        conn.execute_batch("CREATE TRIGGER fail_migration BEFORE UPDATE OF text ON messages WHEN OLD.msg_id='m2' BEGIN SELECT RAISE(ABORT, 'injected write failure'); END;").unwrap();
        assert!(MessageStore::initialize(conn, DataCipher::test_key(42)).is_err());
        let conn = Connection::open(path).unwrap();
        let text: String = conn
            .query_row("SELECT text FROM messages WHERE msg_id='m1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(text, "Nội dung riêng tư ONE");
        let marked: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='history_encryption')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!marked);
        conn.execute_batch("DROP TRIGGER fail_migration;").unwrap();
        let store = MessageStore::initialize(conn, DataCipher::test_key(42)).unwrap();
        assert_eq!(store.messages_for("t1", 10).unwrap().len(), 2);
    }

    #[test]
    fn refuses_tampered_fields_and_retries_interrupted_compaction() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        let store =
            MessageStore::initialize(legacy_database(&path), DataCipher::test_key(42)).unwrap();
        store
            .conn
            .execute("UPDATE history_encryption SET compacted=0", [])
            .unwrap();
        drop(store);
        let store =
            MessageStore::initialize(Connection::open(path).unwrap(), DataCipher::test_key(42))
                .unwrap();
        let compacted: bool = store
            .conn
            .query_row("SELECT compacted FROM history_encryption", [], |r| r.get(0))
            .unwrap();
        assert!(compacted);
        store.conn.execute("UPDATE messages SET text=(SELECT text FROM messages WHERE msg_id='m1') WHERE msg_id='m2'",[]).unwrap();
        assert!(store.messages_for("t1", 10).is_err());
        store
            .conn
            .execute("UPDATE threads SET name='plaintext injection'", [])
            .unwrap();
        assert!(store.all_threads().is_err());
    }

    #[test]
    fn test_insert_twice_returns_false() {
        let mut store = MessageStore::open(":memory:").unwrap();
        let msg = StoredMessage {
            msg_id: "m_1001".into(),
            thread_id: "thread_1".into(),
            sender_id: "u_1".into(),
            sender_name: "Alice".into(),
            text: "Xin chao".into(),
            msg_type: "chat.text".into(),
            ts: 1729500000000,
            is_outgoing: false,
            raw_json: "".into(),
        };

        assert!(store.upsert_message(&msg).unwrap());
        assert!(!store.upsert_message(&msg).unwrap());
    }

    #[test]
    fn test_messages_for_returns_ordered_rows() {
        let mut store = MessageStore::open(":memory:").unwrap();
        let thread_id = "thread_order_test";

        let m1 = StoredMessage {
            msg_id: "m1".into(),
            thread_id: thread_id.into(),
            sender_id: "u1".into(),
            sender_name: "User".into(),
            text: "Msg 200".into(),
            msg_type: "chat.text".into(),
            ts: 200,
            is_outgoing: false,
            raw_json: "".into(),
        };
        let m2 = StoredMessage {
            msg_id: "m2".into(),
            thread_id: thread_id.into(),
            sender_id: "u1".into(),
            sender_name: "User".into(),
            text: "Msg 100".into(),
            msg_type: "chat.text".into(),
            ts: 100,
            is_outgoing: false,
            raw_json: "".into(),
        };
        let m3 = StoredMessage {
            msg_id: "m3".into(),
            thread_id: thread_id.into(),
            sender_id: "u1".into(),
            sender_name: "User".into(),
            text: "Msg 300".into(),
            msg_type: "chat.text".into(),
            ts: 300,
            is_outgoing: false,
            raw_json: "".into(),
        };

        store.upsert_message(&m1).unwrap();
        store.upsert_message(&m2).unwrap();
        store.upsert_message(&m3).unwrap();

        let msgs = store.messages_for(thread_id, 10).unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].ts, 100);
        assert_eq!(msgs[0].msg_id, "m2");
        assert_eq!(msgs[1].ts, 200);
        assert_eq!(msgs[1].msg_id, "m1");
        assert_eq!(msgs[2].ts, 300);
        assert_eq!(msgs[2].msg_id, "m3");
        let latest=store.messages_for(thread_id,2).unwrap();
        assert_eq!(latest.iter().map(|m|m.ts).collect::<Vec<_>>(),vec![200,300]);
    }

    #[test]
    fn failed_history_batch_rolls_back_all_its_messages() {
        let mut store=MessageStore::open(":memory:").unwrap();
        store.conn.execute_batch("CREATE TRIGGER reject_test_message BEFORE INSERT ON messages WHEN NEW.msg_id='reject' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        let thread=Thread{id:"rollback-thread".into(),name:"Name".into(),preview:String::new(),time:String::new(),avatar:None,is_group:true,messages:vec![ChatMessage{id:"accept".into(),text:"first".into(),ts:1000,..Default::default()},ChatMessage{id:"reject".into(),text:"second".into(),ts:2000,..Default::default()}]};
        assert!(persist_threads(&mut store,&[thread]).is_err());
        assert!(store.messages_for("rollback-thread",10).unwrap().is_empty());
        assert!(store.all_threads().unwrap().is_empty());
    }

    #[test]
    fn test_persist_threads_and_upsert_thread() {
        let mut store = MessageStore::open(":memory:").unwrap();
        let thread = Thread {
            id: "t_test".into(),
            name: "Test Thread".into(),
            preview: "Preview".into(),
            time: "10:00".into(),
            avatar: None,
            is_group: false,
            messages: vec![
                ChatMessage {
                    id: "pt_m1".into(),
                    sender: "Alice".into(),
                    text: "Hello".into(),
                    time: "10:00".into(),
                    is_outgoing: false,
                    ts: 1000,
                    ..Default::default()
                },
                ChatMessage {
                    id: "pt_m2".into(),
                    sender: "Tôi".into(),
                    text: "Hi Alice".into(),
                    time: "10:01".into(),
                    is_outgoing: true,
                    ts: 2000,
                    ..Default::default()
                },
            ],
        };

        let count1 = persist_threads(&mut store, &[thread.clone()]).unwrap();
        assert_eq!(count1, 2);

        // Second persist returns 0 newly inserted
        let count2 = persist_threads(&mut store, &[thread]).unwrap();
        assert_eq!(count2, 0);

        // An old history page must not move the conversation's latest timestamp backwards.
        store.upsert_thread(&StoredThread {thread_id:"t_test".into(),name:"Test Thread".into(),is_group:false,last_ts:500}).unwrap();
        let last_ts:i64=store.conn.query_row("SELECT last_ts FROM threads WHERE thread_id='t_test'",[],|row|row.get(0)).unwrap();
        assert_eq!(last_ts,2000);

        let msgs = store.messages_for("t_test", 10).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].text, "Hello");
        assert_eq!(msgs[1].text, "Hi Alice");

        // Hydrate into an empty thread list
        let mut hydrated: Vec<Thread> = Vec::new();
        let loaded = store.hydrate_threads(&mut hydrated, 50).unwrap();
        assert_eq!(loaded, 2);
        assert_eq!(hydrated.len(), 1);
        assert_eq!(hydrated[0].id, "t_test");
        assert_eq!(hydrated[0].name, "Test Thread");
        assert_eq!(hydrated[0].messages.len(), 2);
        assert_eq!(hydrated[0].preview, "Hi Alice");
    }
}
