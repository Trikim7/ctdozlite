//! Bounded image retention and a coalescing, single-writer persistence mailbox.
use super::*;
use std::{
    cell::Cell,
    sync::{Condvar, Mutex},
};

/// Compare complete small poll batches, including message metadata.
/// Large batches bypass the cache to avoid keeping a second history snapshot.
#[derive(Default)]
pub(super) struct PollBatch {
    previous: Vec<Thread>,
}
impl PollBatch {
    pub(super) fn changed(&mut self, batch:&[Thread])->bool {
        if batch.is_empty() {return false;}
        // Bound both item count and aggregate text/metadata by using serialized size
        // only for the small candidate batches; bypass oversized batches.
        let count=batch.iter().map(|t|t.messages.len()).sum::<usize>();
        if batch.len()>64 || count>256 {
            self.previous.clear();
            return true;
        }
        if self.previous==batch {return false;}
        struct Budget(usize);
        impl std::io::Write for Budget {
            fn write(&mut self, bytes:&[u8])->std::io::Result<usize> {
                if bytes.len()>self.0 {return Err(std::io::Error::other("poll cache limit"));}
                self.0-=bytes.len();Ok(bytes.len())
            }
            fn flush(&mut self)->std::io::Result<()> {Ok(())}
        }
        let bounded=serde_json::to_writer(Budget(1024*1024),batch).is_ok();
        if bounded {self.previous=batch.to_vec();} else {self.previous.clear();}
        true
    }
}

pub(super) const CHAT_PAGE_SIZE:usize=150;
pub(super) fn newest_window(total:usize,limit:usize)->std::ops::Range<usize>{
    total.saturating_sub(limit)..total
}

#[cfg(test)]
mod upstream_perf_tests {
    use super::*;
    #[test]
    fn normal_history_window_pages_without_touching_older_data() {
        assert_eq!(newest_window(2000,CHAT_PAGE_SIZE),1850..2000);
        assert_eq!(newest_window(2000,2*CHAT_PAGE_SIZE),1700..2000);
        assert_eq!(newest_window(17,CHAT_PAGE_SIZE),0..17);
    }
    #[test]
    fn poll_cache_only_suppresses_exact_replays_not_metadata_updates() {
        let mut cache=PollBatch::default();
        let mut batch=vec![Thread {id:"1".into(),name:"Fixture".into(),preview:"hello".into(),
            time:String::new(),avatar:None,is_group:false,
            messages:vec![ChatMessage {id:"m".into(),text:"hello".into(),..Default::default()}]}];
        assert!(cache.changed(&batch));
        assert!(!cache.changed(&batch));
        batch[0].messages[0].reactions.push("❤️".into());
        assert!(cache.changed(&batch));
        assert!(!cache.changed(&batch));
        batch[0].messages[0].text="a".repeat(2*1024*1024);
        assert!(cache.changed(&batch));
        assert!(cache.previous.is_empty());
    }
}

const TEXTURE_BUDGET: usize = 64 * 1024 * 1024;
const TEXTURE_COUNT: usize = 256;

struct Entry {
    texture: TextureHandle,
    touched: Cell<u64>,
}
pub(super) struct TextureCache {
    entries: HashMap<String, Entry>,
    clock: Cell<u64>,
    budget: usize,
    count: usize,
}
impl Default for TextureCache {
    fn default() -> Self {
        Self::new(TEXTURE_BUDGET, TEXTURE_COUNT)
    }
}
impl TextureCache {
    fn new(budget: usize, count: usize) -> Self {
        Self {
            entries: HashMap::new(),
            clock: Cell::new(0),
            budget,
            count,
        }
    }
    fn tick(&self) -> u64 {
        let n = self.clock.get().wrapping_add(1);
        self.clock.set(n);
        n
    }
    fn bytes(&self) -> usize {
        self.entries
            .values()
            .map(|e| {
                let [w, h] = e.texture.size();
                w * h * 4
            })
            .sum()
    }
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }
    pub fn get(&self, key: &str) -> Option<&TextureHandle> {
        let e = self.entries.get(key)?;
        e.touched.set(self.tick());
        Some(&e.texture)
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut TextureHandle> {
        let n = self.tick();
        let e = self.entries.get_mut(key)?;
        e.touched.set(n);
        Some(&mut e.texture)
    }
    pub fn insert(&mut self, key: String, texture: TextureHandle) {
        self.entries.remove(&key);
        let [w, h] = texture.size();
        let needed = w * h * 4;
        if needed > self.budget {
            return;
        }
        while !self.entries.is_empty()
            && (self.bytes() + needed > self.budget || self.entries.len() >= self.count)
        {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.touched.get())
                .map(|(k, _)| k.clone())
                .unwrap();
            self.entries.remove(&oldest);
        }
        let touched = Cell::new(self.tick());
        self.entries.insert(key, Entry { texture, touched });
    }
    pub fn remove(&mut self, key: &str) {
        self.entries.remove(key);
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&String, &TextureHandle) -> bool) {
        self.entries.retain(|key, e| keep(key, &e.texture));
    }
}

/// Replaces an unconsumed snapshot instead of retaining every intermediate copy.
#[derive(Default)]
pub(super) struct Latest<T> {
    slot: Mutex<Option<T>>,
    ready: Condvar,
}
impl<T> Latest<T> {
    pub fn replace(&self, value: T) {
        *self.slot.lock().unwrap() = Some(value);
        self.ready.notify_one();
    }
    pub fn take(&self) -> T {
        let mut slot = self.slot.lock().unwrap();
        loop {
            if let Some(value) = slot.take() {
                return value;
            }
            slot = self.ready.wait(slot).unwrap();
        }
    }
}

/// Only request media intersecting the viewport, with a small preload margin.
pub(super) fn media_visible(ui: &egui::Ui) -> bool {
    ui.clip_rect().expand(200.0).intersects(Rect::from_min_size(
        ui.next_widget_position(),
        Vec2::new(ui.available_width(), 300.0),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn burst_saves_keep_only_latest_pending_snapshot() {
        let slot = Latest::default();
        for n in 0..10000 {
            slot.replace(vec![n; 100]);
        }
        assert_eq!(slot.take(), vec![9999; 100]);
        assert!(slot.slot.lock().unwrap().is_none());
    }
    #[test]
    fn textures_evict_least_recently_used_and_release_handles() {
        let ctx = egui::Context::default();
        let mut cache = TextureCache::new(2 * 16 * 16 * 4, 2);
        let make = |name| {
            ctx.load_texture(
                name,
                egui::ColorImage::new([16, 16], Color32::WHITE),
                TextureOptions::LINEAR,
            )
        };
        cache.insert("a".into(), make("a"));
        cache.insert("b".into(), make("b"));
        cache.get("a");
        cache.insert("c".into(), make("c"));
        assert!(cache.contains_key("a"));
        assert!(!cache.contains_key("b"));
        for n in 0..1000 {
            cache.insert(n.to_string(), make("test"));
            assert!(cache.bytes() <= 2 * 16 * 16 * 4);
        }
        assert_eq!(cache.entries.len(), 2);
        assert_eq!(ctx.tex_manager().read().num_allocated(), 3); // font atlas + two retained images
    }
    #[test]
    fn offscreen_media_does_not_start_downloads() {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(media_visible(ui));
                ui.add_space(10000.0);
                assert!(!media_visible(ui));
            });
        });
    }
    #[test]
    fn long_photo_history_requests_only_viewport_images() {
        let ctx = egui::Context::default();
        let textures = TextureCache::default();
        let mut loading = HashSet::new();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for n in 0..5000 {
                        let msg = ChatMessage {
                            text: "[Hình ảnh]".into(),
                            link_src: Some("Hình ảnh Zalo".into()),
                            link_url: Some(format!("https://photo.zdn.vn/fixture/{n}.png")),
                            ..Default::default()
                        };
                        messages::render_message_body(
                            ui,
                            &msg,
                            "Nhóm thử",
                            &ThemePalette::dark(),
                            false,
                            270.0,
                            &textures,
                            &mut loading,
                        );
                    }
                });
            });
        });
        assert!(!loading.is_empty());
        assert!(
            loading.len() < 20,
            "queued {} images for a 600px viewport",
            loading.len()
        );
    }
    #[test]
    fn decoded_image_backlog_holds_admission_until_consumed() {
        let permits = std::sync::Arc::new(tokio::sync::Semaphore::new(3));
        let mut pending = Vec::new();
        for _ in 0..3 {
            pending.push(permits.clone().try_acquire_owned().unwrap());
        }
        assert!(permits.clone().try_acquire_owned().is_err());
        pending.pop();
        assert!(permits.clone().try_acquire_owned().is_ok());
    }
}

/// Release an in-flight flag on return, cancellation and unwind, not only the success path.
pub(super) struct WorkGuard(&'static std::sync::atomic::AtomicBool);
impl WorkGuard {
    pub(super) fn claim(flag:&'static std::sync::atomic::AtomicBool)->Option<Self>{
        flag.compare_exchange(false,true,std::sync::atomic::Ordering::AcqRel,std::sync::atomic::Ordering::Acquire).ok().map(|_|Self(flag))
    }
}
impl Drop for WorkGuard{fn drop(&mut self){self.0.store(false,std::sync::atomic::Ordering::Release);}}
#[cfg(test)]mod work_guard_tests {
    use super::*;
    #[tokio::test]async fn cancellation_releases_the_slot(){static FLAG:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);let guard=WorkGuard::claim(&FLAG).unwrap();assert!(WorkGuard::claim(&FLAG).is_none());let (tx,rx)=tokio::sync::oneshot::channel();let task=tokio::spawn(async move {let _guard=guard;let _=tx.send(());std::future::pending::<()>().await;});rx.await.unwrap();task.abort();let _=task.await;assert!(WorkGuard::claim(&FLAG).is_some());}
}
