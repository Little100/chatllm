use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// 流任务条目
struct StreamEntry {
    handle: JoinHandle<()>,
    cancelled: Arc<AtomicBool>,
}

/// 工具确认等待器
struct ConfirmWaiter {
    tx: tokio::sync::oneshot::Sender<bool>,
}

/// 活跃流的句柄管理器
pub struct StreamManager {
    inner: Arc<Mutex<HashMap<String, StreamEntry>>>,
    confirms: Arc<Mutex<HashMap<String, ConfirmWaiter>>>,
}

impl StreamManager {
    /// 构造新管理器
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            confirms: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 创建取消标记(在 spawn 前调用，传入异步任务中检查)
    pub fn create_cancel_token() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    /// 注册流任务
    pub async fn register(&self, id: String, handle: JoinHandle<()>, cancelled: Arc<AtomicBool>) {
        let old_opt = {
            let mut map = self.inner.lock().await;
            map.remove(&id)
        };
        if let Some(old) = old_opt {
            old.cancelled.store(true, Ordering::Relaxed);
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            old.handle.abort();
        }
        let mut map = self.inner.lock().await;
        map.insert(id, StreamEntry { handle, cancelled });
    }

    /// 优雅取消
    pub async fn cancel(&self, id: &str) -> bool {
        let entry_opt = {
            let mut map = self.inner.lock().await;
            map.remove(id)
        };
        if let Some(entry) = entry_opt {
            entry.cancelled.store(true, Ordering::Relaxed);
            let handle = entry.handle;
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                if !handle.is_finished() {
                    handle.abort();
                }
            });
            true
        } else {
            false
        }
    }

    /// 注册一个工具确认等待, 返回 rx 供后端 await
    pub async fn register_confirm(&self, tool_call_id: String) -> tokio::sync::oneshot::Receiver<bool> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut map = self.confirms.lock().await;
        map.insert(tool_call_id, ConfirmWaiter { tx });
        rx
    }

    /// 前端回复确认结果
    pub async fn resolve_confirm(&self, tool_call_id: &str, approved: bool) -> bool {
        let mut map = self.confirms.lock().await;
        if let Some(waiter) = map.remove(tool_call_id) {
            let _ = waiter.tx.send(approved);
            true
        } else {
            false
        }
    }

    /// 移除已完成流
    #[allow(dead_code)]
    pub async fn unregister(&self, id: &str) {
        let mut map = self.inner.lock().await;
        map.remove(id);
    }

    /// 是否存在活跃流
    #[allow(dead_code)]
    pub async fn is_active(&self, id: &str) -> bool {
        let map = self.inner.lock().await;
        map.contains_key(id)
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}
