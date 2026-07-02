use std::sync::Arc;
use dashmap::DashMap;
use jadm_common::types::{Download, DownloadStatus, DownloadEngine, DownloadView};
use crate::db::queries;
use crate::aria2::client::Aria2ClientTrait;
use crate::ytdlp::detect::detect_engine;
use crate::ytdlp::runner::run_ytdlp;
use uuid::Uuid;
use anyhow::{Result, anyhow};
use sqlx::SqlitePool;
use chrono::Utc;

use crate::notify::dispatcher::NotificationDispatcher;

pub struct AddDownloadParams {
    pub url: String,
    pub folder: String,
    pub category: Option<String>,
    pub format: Option<String>,
    pub mime_type: Option<String>,
    pub cookies: Option<String>,
    pub netscape_cookies: Option<String>,
    pub user_agent: Option<String>,
    pub ghost_mode: bool,
    pub write_subs: bool,
    pub embed_thumbnail: bool,
    pub embed_chapters: bool,
    pub engine: Option<String>,
    pub live_support: bool,
    pub live_from_start: bool,
    pub compress_video: bool,
    pub download_playlist: bool,
    pub referer: Option<String>,
    pub write_description: bool,
}


use tokio::sync::OwnedSemaphorePermit;

pub struct QueueManager {
    downloads: DashMap<Uuid, Download>,
    db_pool: SqlitePool,
    aria2: Arc<dyn Aria2ClientTrait>,
    // Mapping from internal ID to aria2 GID
    aria2_gids: DashMap<Uuid, String>,
    // Mapping from internal ID to ytdlp tokio JoinHandle
    ytdlp_handles: DashMap<Uuid, tokio::task::JoinHandle<Result<()>>>,
    // Mapping from internal ID to ytdlp spawned child PID
    ytdlp_pids: DashMap<Uuid, u32>,
    #[allow(dead_code)]
    notifier: NotificationDispatcher,
    download_semaphore: Arc<tokio::sync::Semaphore>,
    active_permits: DashMap<Uuid, OwnedSemaphorePermit>,
    download_metrics: DashMap<Uuid, (u64, Option<u64>)>, // (rate_bytes, eta_secs)
    pub cookie_jar_password: Arc<tokio::sync::Mutex<Option<String>>>,
}

impl QueueManager {
    pub fn new(db_pool: SqlitePool, aria2: Arc<dyn Aria2ClientTrait>, max_concurrent_downloads: usize) -> Self {
        Self {
            downloads: DashMap::new(),
            db_pool,
            aria2,
            aria2_gids: DashMap::new(),
            ytdlp_handles: DashMap::new(),
            ytdlp_pids: DashMap::new(),
            notifier: NotificationDispatcher::new(true),
            download_semaphore: Arc::new(tokio::sync::Semaphore::new(max_concurrent_downloads)),
            active_permits: DashMap::new(),
            download_metrics: DashMap::new(),
            cookie_jar_password: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub async fn load_from_db(&self) -> Result<()> {
        let mut dls = queries::get_all_downloads(&self.db_pool).await?;
        self.downloads.clear();
        self.download_metrics.clear();
        for dl in &mut dls {
            if dl.status == DownloadStatus::Downloading {
                dl.status = DownloadStatus::Queued;
                if let Err(e) = queries::update_download(&self.db_pool, dl).await {
                    eprintln!("Failed to reset downloading task {} to Queued on startup: {}", dl.id, e);
                }
            }
            self.downloads.insert(dl.id, dl.clone());
        }
        Ok(())
    }

    pub async fn add_download(self: &Arc<Self>, params: AddDownloadParams) -> Result<(Uuid, String)> {
        let mut engine_str = if let Some(ref eng) = params.engine {
            eng.clone()
        } else {
            detect_engine(&params.url, params.mime_type.as_deref()).await?
        };
        
        // FORCE yt-dlp if a specific format is selected
        if params.format.is_some() {
            engine_str = "ytdlp".to_string();
        }

        let engine = match engine_str.as_str() {
            "ytdlp" => DownloadEngine::Ytdlp,
            "chrome_native" => DownloadEngine::ChromeNative,
            "camoufox" => DownloadEngine::Camoufox,
            "browser_fetch" | "siphon_record" => DownloadEngine::BrowserFetch,
            "debugger_capture" => DownloadEngine::DebuggerCapture,
            "webgl_capture" => DownloadEngine::WebGLCapture,
            _ => DownloadEngine::Aria2c,
        };

        // Determine structured download folder
        let mode_str = if params.ghost_mode {
            "ghost"
        } else if params.cookies.is_some() || params.netscape_cookies.is_some() {
            "siphon"
        } else {
            "general"
        };
        let engine_str_val = engine.to_string();
        let structured_folder = get_target_folder(
            &params.folder,
            mode_str,
            &engine_str_val,
            &params.url,
            params.mime_type.as_deref()
        );
        let structured_folder_str = structured_folder.to_string_lossy().into_owned();
        
        let id = Uuid::new_v4();
        let download = Download {
            id,
            url: params.url.clone(),
            filename: None,
            size: None,
            downloaded: 0,
            status: DownloadStatus::Queued,
            category: params.category,
            folder: structured_folder_str.clone(),
            resumable: true,
            connections: 8,
            engine,
            format: params.format.clone(),
            mime_type: params.mime_type,
            cookies: params.cookies,
            netscape_cookies: params.netscape_cookies,
            user_agent: params.user_agent,
            ghost_mode: params.ghost_mode,
            error: None,
            added_at: Utc::now(),
            completed_at: None,
            last_tried_at: None,
            write_subs: params.write_subs,
            embed_thumbnail: params.embed_thumbnail,
            embed_chapters: params.embed_chapters,
            live_support: params.live_support,
            live_from_start: params.live_from_start,
            compress_video: params.compress_video,
            download_playlist: params.download_playlist,
            referer: params.referer,
            write_description: params.write_description,
        };

        {
            queries::insert_download(&self.db_pool, &download).await?;
        }
        
        self.downloads.insert(id, download);
        
        Ok((id, structured_folder_str))
    }

    pub async fn start_download(self: &Arc<Self>, id: Uuid, format: Option<String>) -> Result<()> {
        if self.active_permits.contains_key(&id) {
            return Ok(()); // Already active
        }

        let permit = match self.download_semaphore.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                println!("Concurrency limit reached. Download {} remains in queue.", id);
                return Ok(());
            }
        };

        let (url, folder, engine, format, cookies, netscape_cookies, user_agent, ghost_mode, write_subs, embed_thumbnail, embed_chapters, live_from_start, compress_video, download_playlist, referer, write_description) = {
            let dl = self.downloads.get(&id).ok_or_else(|| anyhow!("Download not found"))?;
            (
                dl.url.clone(),
                dl.folder.clone(),
                dl.engine,
                format.or_else(|| dl.format.clone()),
                dl.cookies.clone(),
                dl.netscape_cookies.clone(),
                dl.user_agent.clone(),
                dl.ghost_mode,
                dl.write_subs,
                dl.embed_thumbnail,
                dl.embed_chapters,
                dl.live_from_start,
                dl.compress_video,
                dl.download_playlist,
                dl.referer.clone(),
                dl.write_description,
            )
        };

        let mut cookies = cookies;
        let mut netscape_cookies = netscape_cookies;
        if cookies.is_none() && netscape_cookies.is_none() {
            if let Ok(url_parsed) = reqwest::Url::parse(&url) {
                if let Some(domain) = url_parsed.domain() {
                    if let Some(profile_id) = find_cookie_master_profile_id(domain).await {
                        let pass_lock = self.cookie_jar_password.lock().await;
                        let pass_opt = pass_lock.clone().or_else(|| std::env::var("COOKIE_JAR_PASSWORD").ok());
                        if let Some(pass) = pass_opt {
                            match decrypt_cookie_master_profile(profile_id, &pass).await {
                                Ok(nc) => {
                                    println!("[JADMan Daemon] Loaded cookies from Cookie Master profile ID {} for {}", profile_id, domain);
                                    cookies = Some(netscape_to_cookie_header(&nc));
                                    netscape_cookies = Some(nc);
                                }
                                Err(e) => {
                                    eprintln!("[JADMan Daemon] Failed to decrypt Cookie Master profile {}: {}", profile_id, e);
                                }
                            }
                        } else {
                            eprintln!("[JADMan Daemon] Cookie Master profile found for {}, but no password is set. Prompt in TUI or set COOKIE_JAR_PASSWORD.", domain);
                        }
                    }
                }
            }
        }
 
        match engine {
            DownloadEngine::Aria2c => {
                let existing_gid = self.aria2_gids.get(&id).map(|g| g.value().clone());
                
                let gid = if let Some(gid) = existing_gid {
                    if let Ok(res) = self.aria2.unpause(&gid).await {
                        res
                    } else {
                        let mut options = serde_json::json!({ "dir": folder });
                        if let Some(c) = cookies {
                            options["header"] = serde_json::json!(vec![format!("Cookie: {}", c)]);
                        }
                        if let Some(ref ref_url) = referer {
                            options["referer"] = serde_json::json!(ref_url);
                        }
                        if ghost_mode {
                            let token = crate::config::get_socks_token()?;
                            options["all-proxy"] = serde_json::json!(format!("socks5://jadm:{}@127.0.0.1:6247", token));
                        }
                        self.aria2.add_uri(&url, options).await?
                    }
                } else {
                    let mut options = serde_json::json!({ "dir": folder });
                    if let Some(c) = cookies {
                        options["header"] = serde_json::json!(vec![format!("Cookie: {}", c)]);
                    }
                    if let Some(ref ref_url) = referer {
                        options["referer"] = serde_json::json!(ref_url);
                    }
                    if ghost_mode {
                        let token = crate::config::get_socks_token()?;
                        options["all-proxy"] = serde_json::json!(format!("socks5://jadm:{}@127.0.0.1:6247", token));
                    }
                    self.aria2.add_uri(&url, options).await?
                };
                
                self.aria2_gids.insert(id, gid);
            }
            DownloadEngine::Ytdlp => {
                let manager = self.clone();
                let handle = tokio::spawn(async move {
                    run_ytdlp(&url, &folder, format, cookies, netscape_cookies, user_agent, ghost_mode, write_subs, embed_thumbnail, embed_chapters, live_from_start, compress_video, download_playlist, referer, write_description,
                        |pid| {
                            manager.ytdlp_pids.insert(id, pid);
                        },
                        |progress| {
                            if let Some(mut dl) = manager.downloads.get_mut(&id)
                                && (dl.status == DownloadStatus::Downloading || dl.status == DownloadStatus::Queued) {
                                    dl.size = progress.total_size;
                                    dl.status = DownloadStatus::Downloading;
                                    
                                    if let Some(fname) = progress.filename {
                                        dl.filename = Some(fname);
                                    }
                                    
                                    if let Some(dl_bytes) = progress.downloaded_bytes {
                                        dl.downloaded = dl_bytes;
                                    } else if let Some(s) = progress.total_size {
                                        dl.downloaded = (progress.percent as f64 / 100.0 * s as f64) as u64;
                                    }
                            }
                            manager.download_metrics.insert(id, (progress.rate_bytes, progress.eta_secs));
                        }
                    ).await
                });
                self.ytdlp_handles.insert(id, handle);
            }
            DownloadEngine::ChromeNative | DownloadEngine::BrowserFetch | DownloadEngine::DebuggerCapture | DownloadEngine::WebGLCapture => {
                // These are handled by extension or siphoning. 
                // We keep the permit while they are "Downloading".
            }
            DownloadEngine::Camoufox => {
                let manager = self.clone();
                let handle = tokio::spawn(async move {
                    let mut _cf_cookie_guard = None;
                    let cookie_file = if let Some(nc) = netscape_cookies {
                        if let Ok(mut temp) = tempfile::NamedTempFile::new() {
                            use std::io::Write;
                            if temp.write_all(nc.as_bytes()).is_ok() {
                                let path = temp.path().to_path_buf();
                                _cf_cookie_guard = Some(temp);
                                Some(path)
                            } else { None }
                        } else { None }
                    } else { None };

                    let script_path = std::env::current_dir()
                       .map(|d| d.join("scripts/download_camoufox.py"))
                       .unwrap_or_else(|_| std::path::PathBuf::from("scripts/download_camoufox.py"));

                    let mut cmd = tokio::process::Command::new("python3");
                    cmd.arg(script_path)
                       .arg("--url").arg(&url)
                       .arg("--folder").arg(&folder);

                    if let Some(ref ua) = user_agent {
                        cmd.arg("--user-agent").arg(ua);
                    }
                    if let Some(ref cf) = cookie_file {
                        cmd.arg("--cookies-file").arg(cf);
                    }

                    cmd.stdout(std::process::Stdio::piped());
                    cmd.stderr(std::process::Stdio::piped());

                    let mut child = cmd.spawn()?;
                    let pid = child.id().unwrap_or(0);
                    manager.ytdlp_pids.insert(id, pid);

                    let status = child.wait().await?;
                    
                    if status.success() {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!("Camoufox download subprocess returned error status"))
                    }
                });
                self.ytdlp_handles.insert(id, handle);
            }
        }

        self.active_permits.insert(id, permit);
        if let Some(mut dl) = self.downloads.get_mut(&id) {
            dl.status = DownloadStatus::Downloading;
        }
        
        Ok(())
    }

    pub async fn tick(self: &Arc<Self>) -> Result<()> {
        let mut to_start = Vec::new();
        let mut active_dls = Vec::new();

        // 1. Gather downloads of interest under a short-lived read lock
        for entry in self.downloads.iter() {
            let id = *entry.key();
            let dl = entry.value();
            if dl.status == DownloadStatus::Queued {
                to_start.push(id);
            } else if dl.status == DownloadStatus::Downloading {
                active_dls.push((id, dl.engine));
            }
        }

        // 2. Query engine status without holding any DashMap locks
        for (id, engine) in &active_dls {
            match engine {
                DownloadEngine::Aria2c => {
                    let gid_val = self.aria2_gids.get(id).map(|g| g.value().clone());
                    if let Some(gid_val) = gid_val
                        && let Ok(status) = self.aria2.tell_status(&gid_val).await
                        && let Some(mut dl_entry) = self.downloads.get_mut(id) {
                                dl_entry.size = status.total_length.parse().ok();
                                dl_entry.downloaded = status.completed_length.parse().unwrap_or(0);
                                
                                let rate = status.download_speed.parse().unwrap_or(0);
                                self.download_metrics.insert(*id, (rate, None));
                                
                                dl_entry.status = match status.status.as_str() {
                                    "active" => DownloadStatus::Downloading,
                                    "waiting" => DownloadStatus::Queued,
                                    "paused" => DownloadStatus::Paused,
                                    "error" => {
                                        println!("Aria2 failed for {}. Attempting yt-dlp fallback...", dl_entry.url);
                                        dl_entry.engine = DownloadEngine::Ytdlp;
                                        DownloadStatus::Queued
                                    },
                                    "complete" => {
                                        let mut rescue_needed = false;
                                        if let Some(filename) = status.files.first().map(|f| f.path.clone()) {
                                            let path_str = filename.clone();
                                            let path = std::path::Path::new(&path_str);
                                            
                                            // ASYNC check for HTML Trap
                                            let rescue_check = async {
                                                if tokio::fs::metadata(path).await.is_ok() {
                                                    let meta = tokio::fs::metadata(path).await?;
                                                    if meta.len() < 150 * 1024 {
                                                        let content = tokio::fs::read_to_string(path).await?;
                                                        let c = content.to_lowercase();
                                                        if c.contains("<!doctype html") || c.contains("<html") {
                                                            return Ok::<bool, anyhow::Error>(true);
                                                        }
                                                    }
                                                }
                                                Ok::<bool, anyhow::Error>(false)
                                            };
                                            
                                            if let Ok(true) = rescue_check.await {
                                                println!("HTML Trap detected for {}. Rescuing...", dl_entry.url);
                                                let _ = tokio::fs::remove_file(path).await;
                                                rescue_needed = true;
                                            }
                                        }

                                        if rescue_needed {
                                            dl_entry.engine = DownloadEngine::Ytdlp;
                                            DownloadStatus::Queued
                                        } else {
                                            dl_entry.completed_at = Some(Utc::now());
                                            dl_entry.cookies = None;
                                            DownloadStatus::Done
                                        }
                                    },
                                    "removed" => {
                                        dl_entry.cookies = None;
                                        DownloadStatus::Cancelled
                                    },
                                    _ => dl_entry.status.clone(),
                                };
                                
                                if dl_entry.status == DownloadStatus::Done || dl_entry.status == DownloadStatus::Cancelled || dl_entry.status == DownloadStatus::Failed {
                                    self.active_permits.remove(id);
                                    self.download_metrics.remove(id);
                                }
                        }
                }
                DownloadEngine::Ytdlp => {
                    let is_finished = self.ytdlp_handles.get(id)
                        .map(|h| h.value().is_finished())
                        .unwrap_or(false);
                    if is_finished
                        && let Some((_, handle)) = self.ytdlp_handles.remove(id) {
                            let result = handle.await;
                            self.ytdlp_pids.remove(id);
                            if let Some(mut dl_entry) = self.downloads.get_mut(id) {
                                match result {
                                    Ok(Ok(_)) => {
                                        dl_entry.status = DownloadStatus::Done;
                                        dl_entry.completed_at = Some(Utc::now());
                                        dl_entry.cookies = None;
                                        
                                        // Update size from actual file on disk if missing
                                        if let Some(ref fname) = dl_entry.filename {
                                            let path = std::path::Path::new(&dl_entry.folder).join(fname);
                                            if let Ok(metadata) = std::fs::metadata(&path) {
                                                let size = metadata.len();
                                                dl_entry.size = Some(size);
                                                dl_entry.downloaded = size;
                                            }
                                        }
                                    }
                                    res => {
                                        let err_desc = match res {
                                            Ok(Err(e)) => e.to_string(),
                                            Err(e) => format!("Tokio join error: {}", e),
                                            _ => "Unknown error".to_string(),
                                        };
                                        eprintln!("yt-dlp download failed for ID {}: {}", id, err_desc);
                                        if dl_entry.ghost_mode {
                                            println!("yt-dlp failed in ghost mode for {}. Falling back to Camoufox...", dl_entry.url);
                                            dl_entry.engine = DownloadEngine::Camoufox;
                                            dl_entry.status = DownloadStatus::Queued;
                                        } else {
                                            dl_entry.status = DownloadStatus::Failed;
                                            dl_entry.error = Some(err_desc);
                                        }
                                    }
                                }
                                
                                if dl_entry.status != DownloadStatus::Queued {
                                    self.active_permits.remove(id);
                                    self.download_metrics.remove(id);
                                }
                            }
                        }
                }
                DownloadEngine::ChromeNative => {
                    // Handled natively by browser extension.
                }
                DownloadEngine::BrowserFetch => {
                    // Handled natively by browser extension.
                }
                DownloadEngine::DebuggerCapture => {
                    // Handled natively by browser extension.
                }
                DownloadEngine::WebGLCapture => {
                    // Handled natively by browser extension.
                }
                DownloadEngine::Camoufox => {
                    let is_finished = self.ytdlp_handles.get(id)
                        .map(|h| h.value().is_finished())
                        .unwrap_or(false);
                    if is_finished
                        && let Some((_, handle)) = self.ytdlp_handles.remove(id) {
                            let result = handle.await;
                            self.ytdlp_pids.remove(id);
                            if let Some(mut dl_entry) = self.downloads.get_mut(id) {
                                match result {
                                    Ok(Ok(_)) => {
                                        dl_entry.status = DownloadStatus::Done;
                                        dl_entry.completed_at = Some(Utc::now());
                                        dl_entry.cookies = None;
                                        // Let's grab the filename if we can
                                        if dl_entry.filename.is_none()
                                            && let Some(fname) = dl_entry.url.split('/').next_back().map(|s| s.split('?').next().unwrap_or(s)) {
                                                dl_entry.filename = Some(fname.to_string());
                                        }
                                        
                                        // Update size from actual file on disk if missing
                                        if let Some(ref fname) = dl_entry.filename {
                                            let path = std::path::Path::new(&dl_entry.folder).join(fname);
                                            if let Ok(metadata) = std::fs::metadata(&path) {
                                                let size = metadata.len();
                                                dl_entry.size = Some(size);
                                                dl_entry.downloaded = size;
                                            }
                                        }
                                    }
                                    _ => {
                                        dl_entry.status = DownloadStatus::Failed;
                                        dl_entry.error = Some("Camoufox download failed".to_string());
                                    }
                                }
                                self.active_permits.remove(id);
                                self.download_metrics.remove(id);
                            }
                        }
                }
            }
        }
        
        // Handle engines that don't have active polling tasks but might be done via RPC (like ChromeNative)
        let to_remove = self.active_permits.iter()
            .filter(|entry| {
                let id = entry.key();
                self.downloads.get(id).map(|dl| dl.status == DownloadStatus::Done || dl.status == DownloadStatus::Cancelled || dl.status == DownloadStatus::Failed).unwrap_or(true)
            })
            .map(|entry| *entry.key())
            .collect::<Vec<_>>();
        
        for id in to_remove {
            self.active_permits.remove(&id);
        }
        
        // 3. Persist updates to DB unconditionally (removed db_snapshot)
        let mut dls_to_save = Vec::new();
        for entry in self.downloads.iter() {
            dls_to_save.push(entry.value().clone());
        }

        if !dls_to_save.is_empty() {
            for dl in dls_to_save {
                if let Err(e) = queries::update_download(&self.db_pool, &dl).await {
                    eprintln!("Failed to update download {} in DB: {}", dl.id, e);
                }
            }
        }

        // 4. Start queued downloads (respecting concurrency limit)
        let slots_available = self.download_semaphore.available_permits();
        for id in to_start.into_iter().take(slots_available) {
            if let Err(e) = self.start_download(id, None).await {
                eprintln!("Failed to start queued download {}: {}", id, e);
                if let Some(mut dl) = self.downloads.get_mut(&id) {
                    dl.status = DownloadStatus::Failed;
                    dl.error = Some(format!("Failed to start: {}", e));
                    let _ = queries::update_download(&self.db_pool, &*dl).await;
                }
            }
        }
        
        Ok(())
    }

    pub async fn cleanup_old_history(&self, days: i64) -> Result<()> {
        let count = queries::purge_old_history(&self.db_pool, days).await?;
        if count > 0 {
            println!("JADM Privacy: Purged {} old history entries.", count);
            
            let cutoff = Utc::now() - chrono::Duration::days(days);
            self.downloads.retain(|_, dl| {
                !( (dl.status == DownloadStatus::Done || dl.status == DownloadStatus::Cancelled || dl.status == DownloadStatus::Failed)
                   && dl.added_at < cutoff )
            });
        }
        Ok(())
    }

    pub async fn pause_download(&self, id: Uuid) -> Result<()> {
        let has_subprocess = {
            let dl = self.downloads.get(&id);
            dl.map(|d| d.engine == DownloadEngine::Ytdlp || d.engine == DownloadEngine::Camoufox).unwrap_or(false)
        };

        if has_subprocess {
            if let Some((_, pid)) = self.ytdlp_pids.remove(&id) {
                println!("Stopping child PID {}", pid);
                #[cfg(unix)]
                {
                    let res = unsafe { libc::kill(pid as libc::pid_t, libc::SIGINT) };
                    if res != 0 {
                        let err = std::io::Error::last_os_error();
                        eprintln!("Failed to send SIGINT to PID {}: {}", pid, err);
                    } else {
                        println!("Successfully sent SIGINT to child PID {}", pid);
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = std::process::Command::new("taskkill")
                        .args(&["/F", "/PID", &pid.to_string()])
                        .status();
                }
            }
            if let Some((_, handle)) = self.ytdlp_handles.remove(&id) {
                tokio::spawn(async move {
                    let _ = handle.await;
                });
            }
        } else {
            if let Some(gid) = self.aria2_gids.get(&id) {
                self.aria2.pause(&gid).await?;
            }
        }

        if let Some(mut dl) = self.downloads.get_mut(&id) {
            dl.status = DownloadStatus::Paused;
        }
        self.active_permits.remove(&id);
        Ok(())
    }

    pub async fn resume_download(&self, id: Uuid) -> Result<()> {
        let (engine, gid) = {
            let dl = self.downloads.get(&id).ok_or_else(|| anyhow!("Download not found"))?;
            let gid = self.aria2_gids.get(&id).map(|g| g.value().clone());
            (dl.engine, gid)
        };

        match engine {
            DownloadEngine::Aria2c => {
                // aria2c tracks paused state itself — call unpause, don't re-add
                if let Some(gid) = gid {
                    self.aria2.unpause(&gid).await?;
                }
                if let Some(mut dl) = self.downloads.get_mut(&id) {
                    dl.status = DownloadStatus::Downloading;
                }
            }
            DownloadEngine::Ytdlp | DownloadEngine::Camoufox => {
                // yt-dlp process was killed on pause; re-queue it.
                // tick() will re-spawn with --continue automatically (handled in runner.rs).
                if let Some(mut dl) = self.downloads.get_mut(&id) {
                    dl.status = DownloadStatus::Queued;
                }
            }
            _ => {
                // Browser-native engines: just re-queue
                if let Some(mut dl) = self.downloads.get_mut(&id) {
                    dl.status = DownloadStatus::Queued;
                }
            }
        }

        Ok(())
    }

    pub async fn stop_download(&self, id: Uuid) -> Result<()> {
        let current_status = {
            let dl = self.downloads.get(&id);
            dl.map(|d| d.status.clone()).ok_or_else(|| anyhow!("Download not found"))?
        };

        if current_status == DownloadStatus::Done || current_status == DownloadStatus::Failed || current_status == DownloadStatus::Cancelled {
            return Ok(());
        }

        let has_subprocess = {
            let dl = self.downloads.get(&id);
            dl.map(|d| d.engine == DownloadEngine::Ytdlp || d.engine == DownloadEngine::Camoufox).unwrap_or(false)
        };

        if has_subprocess {
            if let Some((_, pid)) = self.ytdlp_pids.remove(&id) {
                println!("Stopping child PID {}", pid);
                #[cfg(unix)]
                {
                    let res = unsafe { libc::kill(pid as libc::pid_t, libc::SIGINT) };
                    if res != 0 {
                        let err = std::io::Error::last_os_error();
                        eprintln!("Failed to send SIGINT to PID {}: {}", pid, err);
                    } else {
                        println!("Successfully sent SIGINT to child PID {}", pid);
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = std::process::Command::new("taskkill")
                        .args(&["/F", "/PID", &pid.to_string()])
                        .status();
                }
            }
            if let Some((_, handle)) = self.ytdlp_handles.remove(&id) {
                tokio::spawn(async move {
                    let _ = handle.await;
                });
            }
        } else {
            if let Some(gid) = self.aria2_gids.get(&id) {
                self.aria2.remove(&gid).await?;
            }
        }

        if let Some(mut dl) = self.downloads.get_mut(&id) {
            let is_playlist = dl.download_playlist;
            let has_finished_some = is_playlist && has_completed_media_files(&dl.folder);

            if (dl.live_support && (dl.size.is_none() || dl.size == Some(0)) && dl.status == DownloadStatus::Downloading)
                || has_finished_some
            {
                dl.status = DownloadStatus::Done;
                dl.completed_at = Some(Utc::now());
                if dl.size.is_none() || dl.size == Some(0) {
                    dl.size = Some(dl.downloaded);
                }
            } else {
                dl.status = DownloadStatus::Cancelled;
            }
        }
        self.active_permits.remove(&id);
        Ok(())
    }

    pub async fn delete_download(&self, id: Uuid, delete_file: bool) -> Result<()> {
        let path = {
            let dl = self.downloads.get(&id).ok_or_else(|| anyhow!("Download not found"))?;
            if let Some(filename) = &dl.filename {
                Some(std::path::Path::new(&dl.folder).join(filename))
            } else {
                None
            }
        };

        self.stop_download(id).await?;
        self.downloads.remove(&id);

        
        queries::delete_download(&self.db_pool, id).await?;

        if delete_file && let Some(p) = path && p.exists() {
            let _ = std::fs::remove_file(p);
        }
        
        Ok(())
    }

    pub fn get_queue(&self) -> Vec<DownloadView> {
        let mut views = Vec::new();
        for entry in self.downloads.iter() {
            let id = *entry.key();
            let dl = entry.value().clone();
            let (rate, eta) = self.download_metrics.get(&id).map(|m| *m.value()).unwrap_or((0, None));
            views.push(DownloadView::new(dl, rate, eta));
        }
        views.sort_by_key(|v| v.download.added_at);
        views
    }

    pub fn get_download(&self, id: &Uuid) -> Option<DownloadView> {
        self.downloads.get(id).map(|d| {
            let (rate, eta) = self.download_metrics.get(id).map(|m| *m.value()).unwrap_or((0, None));
            DownloadView::new(d.value().clone(), rate, eta)
        })
    }

    pub async fn update_download_metadata(&self, id: &Uuid, filename: Option<String>, size: Option<u64>, downloaded: Option<u64>, rate: Option<u64>, eta: Option<u64>) {
        if let Some(mut dl) = self.downloads.get_mut(id) {
            if let Some(f) = filename { dl.filename = Some(f); }
            if let Some(s) = size { dl.size = Some(s); }
            if let Some(d) = downloaded { dl.downloaded = d; }
        }
        if rate.is_some() || eta.is_some() {
            let (old_rate, old_eta) = self.download_metrics.get(id).map(|m| *m.value()).unwrap_or((0, None));
            self.download_metrics.insert(*id, (rate.unwrap_or(old_rate), eta.or(old_eta)));
        }
    }

    pub async fn update_download_metadata_delta(&self, id: &Uuid, filename: Option<String>, size: Option<u64>, downloaded_delta: Option<u64>) {
        if let Some(mut dl) = self.downloads.get_mut(id) {
            if let Some(f) = filename { dl.filename = Some(f); }
            if let Some(s) = size { dl.size = Some(s); }
            if let Some(delta) = downloaded_delta { dl.downloaded += delta; }
        }
    }

    pub async fn mark_download_done(&self, id: &Uuid, final_filename: Option<String>, final_size: Option<u64>) {
        if let Some(mut dl) = self.downloads.get_mut(id) {
            dl.status = DownloadStatus::Done;
            dl.completed_at = Some(Utc::now());
            if let Some(f) = final_filename { dl.filename = Some(f); }
            if let Some(s) = final_size {
                dl.size = Some(s);
                dl.downloaded = s;
            }
            dl.cookies = None;
            dl.netscape_cookies = None;
        }
        self.download_metrics.remove(id);
    }

    pub async fn move_siphoned_file(&self, source: &str, destination: &str, daemon_id: Option<String>) -> Result<()> {
        let src_path = std::path::Path::new(source);
        let dest_path = std::path::Path::new(destination);
        
        // Ensure destination parent directory exists
        if let Some(parent) = dest_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        
        // Try renaming first, fallback to copy + delete across mount points
        if tokio::fs::rename(&src_path, &dest_path).await.is_err() {
            tokio::fs::copy(&src_path, &dest_path).await?;
            let _ = tokio::fs::remove_file(&src_path).await;
        }
        
        if let Some(id_str) = daemon_id {
            if let Ok(id) = uuid::Uuid::parse_str(&id_str) {
                let filename = dest_path.file_name().map(|f| f.to_string_lossy().to_string());
                let size = tokio::fs::metadata(&dest_path).await.map(|m| m.len()).ok();
                self.mark_download_done(&id, filename, size).await;
                
                // Persist update to DB immediately
                if let Some(dl) = self.downloads.get(&id) {
                    let _ = crate::db::queries::update_download(&self.db_pool, &*dl).await;
                }
            }
        }
        Ok(())
    }

    pub async fn handle_siphon_chunk(
        &self,
        id: Uuid,
        chunk_index: usize,
        is_last: bool,
        filename: String,
        total_size: u64,
        data: Vec<u8>,
    ) -> Result<()> {
        // Sanitize the filename to prevent path traversal (e.g. "../../../etc/passwd")
        let safe_filename = std::path::Path::new(&filename)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download")
            .to_string();

        let folder = {
            let mut dl = self.downloads.get_mut(&id).ok_or_else(|| anyhow!("Download {} not found", id))?;
            
            // If the download was already finished, failed, or cancelled, ignore new chunks
            if dl.status == DownloadStatus::Done || dl.status == DownloadStatus::Failed || dl.status == DownloadStatus::Cancelled {
                return Ok(());
            }
            
            if chunk_index == 0 {
                dl.downloaded = data.len() as u64;
                dl.filename = Some(safe_filename.clone());
                dl.size = Some(total_size);
                dl.status = DownloadStatus::Downloading;
            } else {
                dl.downloaded += data.len() as u64;
                dl.size = Some(total_size);
                dl.status = DownloadStatus::Downloading;
            }
            dl.folder.clone()
        };

        // Ensure target directory exists
        tokio::fs::create_dir_all(&folder).await?;

        // Write the chunk to file
        let file_path = std::path::Path::new(&folder).join(&safe_filename);
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(chunk_index == 0)
            .open(&file_path)
            .await?;

        if chunk_index > 0 {
            use tokio::io::AsyncSeekExt;
            file.seek(tokio::io::SeekFrom::End(0)).await?;
        }

        use tokio::io::AsyncWriteExt;
        file.write_all(&data).await?;

        if is_last {
            let downloaded = {
                let dl = self.downloads.get(&id).ok_or_else(|| anyhow!("Download {} not found", id))?;
                dl.downloaded
            };
            self.mark_download_done(&id, Some(safe_filename), Some(downloaded)).await;
            
            // Persist to DB immediately on completion
            if let Some(dl) = self.downloads.get(&id) {
                if let Err(e) = queries::update_download(&self.db_pool, &*dl).await {
                    eprintln!("Failed to update database for download {} on completion: {}", id, e);
                }
            }
        }

        Ok(())
    }

    pub async fn start_cdm_extractor(&self) -> Result<()> {
        let script_path = std::env::current_dir()
            .map(|d| d.join("scripts/cdm_extractor.py"))
            .unwrap_or_else(|_| std::path::PathBuf::from("scripts/cdm_extractor.py"));

        if !script_path.exists() {
            return Err(anyhow!("cdm_extractor.py script not found at {:?}", script_path));
        }

        tokio::spawn(async move {
            let mut cmd = tokio::process::Command::new("python3");
            cmd.arg(script_path)
               .arg("--output").arg("/tmp/jadm_cdm_keys.txt")
               .arg("--timeout").arg("120");

            println!("[Daemon CDM] Spawning CDM Extractor script...");
            match cmd.spawn() {
                Ok(mut child) => {
                    let _ = child.wait().await;
                    println!("[Daemon CDM] CDM Extractor script exited.");
                }
                Err(e) => {
                    eprintln!("[Daemon CDM] Failed to spawn cdm_extractor.py: {}", e);
                }
            }
        });

        // Trigger a system notification
        {
            let _ = notify_rust::Notification::new()
                .summary("JADMan CDM Key Extractor Active")
                .body("Frida hook is listening. Please play the DRM video in Chrome/Chromium to extract decryption keys!")
                .icon("dialog-information")
                .timeout(8000)
                .show();
        }

        Ok(())
    }
}

pub fn get_target_folder(base_folder: &str, mode: &str, engine: &str, url: &str, mime_type: Option<&str>) -> std::path::PathBuf {
    let mode_sub = match mode {
        "siphon" => "Siphon",
        "ghost" => match engine {
            "chrome_native" | "chrome" => "Ghost/ChromeNative",
            "ytdlp" | "yt-dlp" => "Ghost/Ytdlp",
            "camoufox" => "Ghost/Camoufox",
            "browser_fetch" => "Ghost/BrowserFetch",
            "debugger_capture" => "Ghost/DebuggerCapture",
            "webgl_capture" => "Ghost/WebGLCapture",
            _ => "Ghost/Ytdlp",
        },
        _ => "General",
    };

    // Determine category based on filename extension or mime type
    let filename = url.split('?').next().unwrap_or(url).split('/').next_back().unwrap_or("download").to_lowercase();
    
    // Improved extension detection to handle common double extensions
    let ext = if filename.ends_with(".tar.gz") {
        "tar.gz"
    } else if filename.ends_with(".tar.bz2") {
        "tar.bz2"
    } else if filename.ends_with(".tar.xz") {
        "tar.xz"
    } else {
        filename.split('.').next_back().unwrap_or("")
    };

    let mime = mime_type.unwrap_or("").to_lowercase();

    let category = match ext {
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "csv" | "rtf" => "Documents",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "tar.gz" | "tar.bz2" | "tar.xz" | "arj" | "sit" | "sitx" | "ace" => "Archives",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpe" | "mpeg" | "mpg" => "Videos",
        "mp3" | "wav" | "wma" | "aac" | "m4a" | "ogg" | "flac" | "aif" | "mpa" => "Audio",
        "exe" | "msi" | "apk" | "bin" | "run" | "sh" | "appimage" | "dmg" => "Programs",
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" | "ico" | "tiff" => "Images",
        _ => {
            if mime.contains("document") || mime.contains("pdf") || mime == "text/plain" || mime.contains("sheet") {
                "Documents"
            } else if mime.contains("zip") || mime.contains("compressed") || mime.contains("tar") || mime.contains("archive") {
                "Archives"
            } else if mime.contains("video") {
                "Videos"
            } else if mime.contains("audio") {
                "Audio"
            } else if mime.contains("octet-stream") && (ext == "exe" || ext == "bin" || ext == "apk") {
                "Programs"
            } else if mime.contains("image") {
                "Images"
            } else {
                "Others"
            }
        }
    };

    std::path::Path::new(base_folder).join(mode_sub).join(category)
}

pub fn get_unique_path(dest_dir: &std::path::Path, filename: &std::ffi::OsStr) -> std::path::PathBuf {
    let dest_path = dest_dir.join(filename);
    if !dest_path.exists() {
        return dest_path;
    }

    let stem = dest_path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let ext = dest_path.extension().unwrap_or_default().to_string_lossy().into_owned();

    let mut counter = 1;
    loop {
        let new_filename = if ext.is_empty() {
            format!("{} ({})", stem, counter)
        } else {
            format!("{} ({}).{}", stem, counter, ext)
        };
        let new_path = dest_dir.join(new_filename);
        if !new_path.exists() {
            return new_path;
        }
        counter += 1;
    }
}

pub fn has_completed_media_files(folder: &str) -> bool {
    let path = std::path::Path::new(folder);
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if !name_str.ends_with(".part") && !name_str.ends_with(".ytdl") {
                        let ext = std::path::Path::new(name_str.as_ref())
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        if matches!(ext.as_str(), "mp4" | "mkv" | "webm" | "mp3" | "m4a" | "flac" | "wav" | "avi" | "mov" | "3gp" | "ogg") {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

pub fn netscape_to_cookie_header(nc: &str) -> String {
    let mut parts = Vec::new();
    for line in nc.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 7 {
            let name = cols[5];
            let value = cols[6];
            parts.push(format!("{}={}", name, value));
        }
    }
    parts.join("; ")
}

pub async fn find_cookie_master_profile_id(domain: &str) -> Option<i64> {
    use sqlx::Connection;
    let home = std::env::var("HOME").ok()?;
    let db_path = std::path::Path::new(&home).join(".config/CookieJar/jar.db");
    if !db_path.exists() {
        return None;
    }
    
    let mut conn = sqlx::sqlite::SqliteConnection::connect(&format!("sqlite:{}", db_path.to_string_lossy()))
        .await
        .ok()?;
        
    let rows: Vec<(i64, String)> = sqlx::query_as("SELECT id, site FROM cookies")
        .fetch_all(&mut conn)
        .await
        .ok()?;

    for (id, site) in rows {
        let site_clean = site.trim_start_matches("www.").to_lowercase();
        let domain_clean = domain.trim_start_matches("www.").to_lowercase();
        if domain_clean.contains(&site_clean) || site_clean.contains(&domain_clean) {
            return Some(id);
        }
    }
    None
}

pub async fn decrypt_cookie_master_profile(id: i64, password: &str) -> Result<String> {
    let mut cmd = tokio::process::Command::new("cookie-jar");
    cmd.arg("use")
       .arg(id.to_string());
    cmd.env("COOKIE_JAR_PASSWORD", password);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    
    let output = cmd.output().await?;
    if output.status.success() {
        let cookies = String::from_utf8(output.stdout)?;
        Ok(cookies)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("cookie-jar decryption failed: {}", stderr.trim()))
    }
}

