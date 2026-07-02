use arboard::Clipboard;
use std::sync::Arc;
use crate::queue::manager::{QueueManager, AddDownloadParams};
use tokio::time::{sleep, Duration};
use anyhow::Result;

pub struct ClipboardMonitor {
    queue_manager: Arc<QueueManager>,
}

impl ClipboardMonitor {
    pub fn new(queue_manager: Arc<QueueManager>) -> Self {
        Self { queue_manager }
    }

    pub async fn run(&self) -> Result<()> {
        let mut clipboard = Clipboard::new()?;
        let mut last_clipboard = String::new();

        loop {
            if let Ok(current) = clipboard.get_text() {
                let current = current.trim().to_string();
                if !current.is_empty() && current != last_clipboard {
                    if current.starts_with("http://") || current.starts_with("https://") {
                        println!("Clipboard URL detected: {}", current);
                        let default_folder = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()) + "/Downloads";
                        let params = AddDownloadParams {
                            url: current.clone(),
                            folder: default_folder,
                            category: None,
                            format: None,
                            mime_type: None,
                            cookies: None,
                            netscape_cookies: None,
                            user_agent: None,
                            ghost_mode: false,
                            write_subs: false,
                            embed_thumbnail: false,
                            embed_chapters: false,
                            engine: None,
                            live_support: false,
                            live_from_start: false,
                            compress_video: false,
                            download_playlist: false,
                            referer: None,
                            write_description: false,
                        };
                        let _ = self.queue_manager.add_download(params).await;
                    }
                    last_clipboard = current;
                }
            }
            sleep(Duration::from_millis(1000)).await;
        }
    }
}
