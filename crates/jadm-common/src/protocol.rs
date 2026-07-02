use serde::{Deserialize, Serialize};
use crate::types::DownloadView;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatInfo {
    pub id: String,
    pub resolution: String,
    pub ext: String,
    pub note: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "PascalCase")]
pub enum Request {
    AddDownload {
        url: String,
        folder: Option<String>,
        priority: Option<u32>,
        cookies: Option<String>,
        category: Option<String>,
        #[serde(default)]
        mime_type: Option<String>,
        #[serde(default)]
        write_subs: Option<bool>,
        #[serde(default)]
        embed_thumbnail: Option<bool>,
        #[serde(default)]
        embed_chapters: Option<bool>,
        #[serde(default)]
        format: Option<String>,
        #[serde(default)]
        netscape_cookies: Option<String>,
        #[serde(default, alias = "userAgent")]
        user_agent: Option<String>,
        #[serde(default)]
        ghost_mode: Option<bool>,
        #[serde(default)]
        engine: Option<String>,
        #[serde(default)]
        live_support: Option<bool>,
        #[serde(default)]
        live_from_start: Option<bool>,
        #[serde(default)]
        compress_video: Option<bool>,
        #[serde(default)]
        download_playlist: Option<bool>,
        #[serde(default)]
        referer: Option<String>,
        #[serde(default)]
        write_description: Option<bool>,
    },
    GetFormats {
        url: String,
        cookies: Option<String>,
        #[serde(default)]
        netscape_cookies: Option<String>,
        #[serde(default, alias = "userAgent")]
        user_agent: Option<String>,
        #[serde(default)]
        mode: Option<String>,
        #[serde(default)]
        referer: Option<String>,
    },
    PauseDownload { id: Uuid },
    ResumeDownload { id: Uuid },
    StopDownload { id: Uuid },
    SetCookiePassword { password: String },
    DeleteDownload { 
        id: Uuid, 
        #[serde(default)]
        delete_file: bool 
    },
    GetQueue,
    GetDownload { id: Uuid },
    SetSpeedLimit { bytes_per_sec: u64 },
    MoveFile {
        source: String,
        destination: String,
        daemon_id: Option<String>,
    },
    SiphonChunk {
        daemon_id: String,
        chunk_index: usize,
        is_last: bool,
        filename: String,
        total_size: u64,
        data: Vec<u8>,
    },
    StopSiphon {
        daemon_id: String,
    },
    CdmStart {
        url: String,
        license_url: String,
        headers: Option<std::collections::HashMap<String, String>>,
    },
    CdmGetKeys,
    Float,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    Queue { downloads: Vec<DownloadView> },
    Single { download: Box<DownloadView> },
    Formats {
        status: String,
        formats: Vec<FormatInfo>,
    },
    Ok { 
        status: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<uuid::Uuid>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<String>,
    },
    Error { error: String },
}
