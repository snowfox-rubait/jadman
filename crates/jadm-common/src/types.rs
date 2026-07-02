use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Done,
    Failed,
    Cancelled,
}

impl fmt::Display for DownloadStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Queued => "Queued",
            Self::Downloading => "Downloading",
            Self::Paused => "Paused",
            Self::Done => "Done",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadEngine {
    Aria2c,
    Ytdlp,
    #[serde(rename = "chrome_native")]
    ChromeNative,
    Camoufox,
    #[serde(rename = "browser_fetch")]
    BrowserFetch,
    #[serde(rename = "debugger_capture")]
    DebuggerCapture,
    #[serde(rename = "webgl_capture")]
    WebGLCapture,
}

impl fmt::Display for DownloadEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Aria2c => "aria2c",
            Self::Ytdlp => "yt-dlp",
            Self::ChromeNative => "chrome_native",
            Self::Camoufox => "camoufox",
            Self::BrowserFetch => "browser_fetch",
            Self::DebuggerCapture => "debugger_capture",
            Self::WebGLCapture => "webgl_capture",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: Uuid,
    pub url: String,
    pub filename: Option<String>,
    pub size: Option<u64>,
    pub downloaded: u64,
    pub status: DownloadStatus,
    pub category: Option<String>,
    pub folder: String,
    pub resumable: bool,
    pub connections: u32,
    pub engine: DownloadEngine,
    pub format: Option<String>,
    pub mime_type: Option<String>,
    pub cookies: Option<String>,
    pub netscape_cookies: Option<String>,
    pub user_agent: Option<String>,
    pub error: Option<String>,
    pub added_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_tried_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub write_subs: bool,
    #[serde(default)]
    pub embed_thumbnail: bool,
    #[serde(default)]
    pub embed_chapters: bool,
    #[serde(default)]
    pub ghost_mode: bool,
    #[serde(default)]
    pub live_support: bool,
    #[serde(default)]
    pub live_from_start: bool,
    #[serde(default)]
    pub compress_video: bool,
    #[serde(default)]
    pub download_playlist: bool,
    #[serde(default)]
    pub referer: Option<String>,
    #[serde(default)]
    pub write_description: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadView {
    #[serde(flatten)]
    pub download: Download,
    
    pub rate_bytes: u64,
    pub eta_secs: Option<u64>,
    pub percent: u8,
}

impl DownloadView {
    pub fn new(download: Download, rate_bytes: u64, eta_secs: Option<u64>) -> Self {
        let mut percent = 0;
        if download.status == DownloadStatus::Done {
            percent = 100;
        } else if let Some(size) = download.size.filter(|&s| s > 0) {
            percent = ((download.downloaded as f64 / size as f64) * 100.0).min(100.0) as u8;
        }
        Self {
            download,
            rate_bytes,
            eta_secs,
            percent,
        }
    }
}
