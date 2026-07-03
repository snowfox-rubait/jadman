use axum::{
    routing::{get, post},
    Router, Json, extract::State,
    http::{StatusCode, HeaderMap, HeaderValue, header},
    response::IntoResponse,
    extract::Multipart,
};
use axum::extract::FromRequest;
use std::sync::Arc;
use crate::queue::manager::{QueueManager, AddDownloadParams};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use jadm_common::types::DownloadStatus;

#[derive(Clone)]
pub struct HttpRpcServer {
    queue_manager: Arc<QueueManager>,
}

pub async fn run_http_server(queue_manager: Arc<QueueManager>, config: Arc<crate::config::Config>) -> anyhow::Result<()> {
    let state = HttpRpcServer {
        queue_manager,
    };

    let app = Router::new()
        .route("/api/v2/auth/login", post(login_handler))
        .route("/api/v2/app/webapiVersion", get(webapi_version_handler))
        .route("/api/v2/torrents/info", get(torrents_info_handler))
        .route("/api/v2/torrents/add", post(torrents_add_handler))
        .route("/api/v2/torrents/delete", post(torrents_delete_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:6246").await?;
    println!("Mock qBittorrent HTTP server listening on http://127.0.0.1:6246...");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn login_handler() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_static("SID=JADMAN_SESSION_MOCK_ID; Path=/; HttpOnly;")
    );
    (StatusCode::OK, headers, "Ok.")
}

async fn webapi_version_handler() -> &'static str {
    "2.0"
}

#[derive(Serialize)]
struct QbittorrentInfo {
    hash: String,
    name: String,
    size: i64,
    progress: f64,
    dlspeed: i64,
    state: String,
    save_path: String,
    category: String,
}

async fn torrents_info_handler(
    State(state): State<HttpRpcServer>,
) -> Json<Vec<QbittorrentInfo>> {
    let downloads = state.queue_manager.get_queue();
    let mut list = Vec::new();
    for dl in downloads {
        let hash = format!("00000000{}", dl.download.id.simple());
        let size = dl.download.size.unwrap_or(0) as i64;
        let progress = dl.percent as f64 / 100.0;
        
        let state_str = match dl.download.status {
            DownloadStatus::Downloading => "downloading",
            DownloadStatus::Paused => "paused",
            DownloadStatus::Queued => "stalledDL",
            DownloadStatus::Done => "completed",
            DownloadStatus::Failed => "error",
            DownloadStatus::Cancelled => "paused",
        }.to_string();

        list.push(QbittorrentInfo {
            hash,
            name: dl.download.filename.clone().unwrap_or_else(|| "Unknown".to_string()),
            size,
            progress,
            dlspeed: dl.rate_bytes as i64,
            state: state_str,
            save_path: dl.download.folder.clone(),
            category: dl.download.category.clone().unwrap_or_default(),
        });
    }
    Json(list)
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct TorrentsAddForm {
    urls: Option<String>,
    savepath: Option<String>,
    category: Option<String>,
}

async fn torrents_add_handler(
    State(state): State<HttpRpcServer>,
    headers: HeaderMap,
    body: axum::body::Body,
) -> impl IntoResponse {
    let content_type = headers.get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let default_folder = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()) + "/Downloads";
    let mut savepath = default_folder;
    let mut category = None;
    let mut urls = Vec::new();

    if content_type.contains("multipart/form-data") {
        let req = axum::http::Request::builder()
            .header(header::CONTENT_TYPE, content_type)
            .body(body)
            .unwrap();
        if let Ok(mut multipart) = Multipart::from_request(req, &state).await {
            while let Ok(Some(field)) = multipart.next_field().await {
                let name = field.name().unwrap_or("").to_string();
                if name == "savepath" {
                    if let Ok(val) = field.text().await {
                        savepath = val;
                    }
                } else if name == "category" {
                    if let Ok(val) = field.text().await {
                        category = Some(val);
                    }
                } else if name == "urls" {
                    if let Ok(val) = field.text().await {
                        urls.extend(val.lines().map(|s| s.trim().to_string()));
                    }
                } else if name == "torrents" {
                    if let Ok(bytes) = field.bytes().await {
                        if !bytes.is_empty() {
                            let temp_dir = std::env::temp_dir();
                            let torrent_path = temp_dir.join(format!("jadman_{}.torrent", Uuid::new_v4()));
                            if std::fs::write(&torrent_path, &bytes).is_ok() {
                                urls.push(torrent_path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
    } else {
        use http_body_util::BodyExt;
        if let Ok(bytes) = body.collect().await.map(|c| c.to_bytes()) {
            for (k, v) in url::form_urlencoded::parse(&bytes) {
                if k == "urls" {
                    urls.extend(v.lines().map(|s| s.trim().to_string()));
                } else if k == "savepath" {
                    savepath = v.into_owned();
                } else if k == "category" {
                    category = Some(v.into_owned());
                }
            }
        }
    }

    for url in urls {
        if url.is_empty() { continue; }
        let params = AddDownloadParams {
            url,
            folder: savepath.clone(),
            category: category.clone(),
            format: None,
            mime_type: None,
            cookies: None,
            write_subs: false,
            embed_thumbnail: false,
            embed_chapters: false,
            netscape_cookies: None,
            user_agent: None,
            ghost_mode: false,
            engine: Some("aria2".to_string()),
            live_support: false,
            live_from_start: false,
            compress_video: false,
            download_playlist: false,
            referer: None,
            write_description: false,
        };
        let _ = state.queue_manager.add_download(params).await;
    }

    StatusCode::OK
}

async fn torrents_delete_handler(
    State(state): State<HttpRpcServer>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let mut hashes_str = String::new();
    let mut delete_files = false;
    
    for (k, v) in url::form_urlencoded::parse(&body) {
        if k == "hashes" {
            hashes_str = v.into_owned();
        } else if k == "deleteFiles" {
            delete_files = v == "true";
        }
    }
    
    let hashes: Vec<&str> = hashes_str.split(',').collect();
    let downloads = state.queue_manager.get_queue();
    
    for hash in hashes {
        let hash = hash.trim().to_lowercase();
        if hash.is_empty() { continue; }
        
        for dl in &downloads {
            let dl_hash = format!("00000000{}", dl.download.id.simple());
            if dl_hash.to_lowercase() == hash {
                println!("[JADMan HTTP] Deleting download '{}' (hash: {})", dl.download.id, hash);
                let _ = state.queue_manager.delete_download(dl.download.id, delete_files).await;
                break;
            }
        }
    }
    
    StatusCode::OK
}
