use std::io::Write;
use portable_pty::{Child, MasterPty};

/// MasterPty 包装(MasterPty trait 已要求 Send)
pub struct SendMaster(pub Box<dyn MasterPty>);

/// 单个终端会话
pub struct PtySession {
    #[allow(dead_code)]
    pub id: String,
    pub writer: Box<dyn Write + Send>,
    pub child: Box<dyn Child + Send + Sync>,
    pub master: SendMaster,
    #[allow(dead_code)]
    pub reader_handle: Option<std::thread::JoinHandle<()>>,
}
