mod db;
mod aria2;
mod ytdlp;
mod queue;
mod rpc;
mod notify;
mod scheduler;
mod ffmpeg;
mod clipboard;
mod config;
mod desktop;
mod native_messaging;
mod proxy;

use std::sync::Arc;
use crate::db::schema::init_db;
use crate::queue::manager::QueueManager;
use crate::config::Config;

use crate::rpc::unix::UnixRpcServer;
use crate::scheduler::engine::SchedulerEngine;
use crate::clipboard::monitor::ClipboardMonitor;
use sqlx::sqlite::SqlitePoolOptions;
use anyhow::Result;
use tokio::time::{sleep, Duration};
use directories::ProjectDirs;
use std::fs;
use crate::proxy::ca::CertificateAuthority;
use crate::proxy::server::ProxyServer;
use crate::proxy::network::NetworkIntercepter;
use crate::proxy::trust::TrustManager;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "install-desktop-handler" => {
                if let Err(e) = desktop::install_desktop_handler().await {
                    eprintln!("Failed to install desktop handler: {}", e);
                    std::process::exit(1);
                }
                return Ok(());
            }
            "uninstall-desktop-handler" => {
                if let Err(e) = desktop::uninstall_desktop_handler().await {
                    eprintln!("Failed to uninstall desktop handler: {}", e);
                    std::process::exit(1);
                }
                return Ok(());
            }
            "install-native-manifest" => {
                if let Err(e) = crate::native_messaging::install_native_manifest() {
                    eprintln!("Failed to install native manifest: {}", e);
                    std::process::exit(1);
                }
                return Ok(());
            }
            "native-host" => {
                if let Err(e) = crate::native_messaging::run_native_host().await {
                    eprintln!("Native host error: {}", e);
                    std::process::exit(1);
                }
                return Ok(());
            }
            "handle-url" => {
                if args.len() > 2 {
                    return crate::desktop::handle_url(&args[2]).await;
                } else {
                    eprintln!("Usage: jadm-daemon handle-url <URL>");
                    std::process::exit(1);
                }
            }
            _ => {
                // If it is called by Chrome or Firefox, it will have arguments.
                // Chrome passes: chrome-extension://<id>/
                // Firefox passes: <path-to-manifest> <extension-id>
                // Check if any argument starts with chrome-extension:// or matches the Firefox extension ID.
                if args.iter().any(|arg| arg.starts_with("chrome-extension://") || arg == "jadm@snowfox.com" || arg.contains("com.jadm.jadm.json")) {
                    if let Err(e) = crate::native_messaging::run_native_host().await {
                        eprintln!("Native host error: {}", e);
                        std::process::exit(1);
                    }
                    return Ok(());
                }
            }
        }
    }

    println!("Starting jadm-daemon...");

    // 1. Initialize DB
    let proj_dirs = ProjectDirs::from("com", "jadm", "jadm")
        .ok_or_else(|| anyhow::anyhow!("Could not determine project directories"))?;
    let data_dir = proj_dirs.data_dir();
    fs::create_dir_all(data_dir)?;
    
    let db_path = data_dir.join("jadm.db");
    if !db_path.exists() {
        std::fs::File::create(&db_path)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&db_path, std::fs::Permissions::from_mode(0o600));
    }
    let db_url = format!("sqlite://{}", db_path.to_string_lossy());
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url).await?;
        
    sqlx::query("PRAGMA journal_mode = WAL;").execute(&pool).await?;
        
    init_db(&pool).await?;
    println!("Database initialized at {}", db_path.to_string_lossy());

    // 1.5 Load Config
    let config = Arc::new(Config::load().await?);

    // 1.6 Initialize Proxy Intercept Layer
    let mut intercepter = None;
    let mut proxy_handle = None;
    
    if config.proxy.enabled {
        println!("Initializing Proxy Intercept Layer...");
        let ca = Arc::new(CertificateAuthority::load_or_generate(data_dir.to_path_buf())?);
        
        if config.proxy.install_ca {
            let ca_cert_path = data_dir.join("ca.cert.pem");
            let trust_manager = TrustManager::new(ca_cert_path);
            if let Err(e) = trust_manager.install() {
                eprintln!("Failed to install Root CA: {}", e);
            }
        }

        if config.proxy.setup_network {
            let net_intercepter = NetworkIntercepter::new(config.proxy.port, config.proxy.mark);
            if let Err(e) = net_intercepter.setup() {
                eprintln!("Failed to setup network interception: {}", e);
            } else {
                intercepter = Some(net_intercepter);
            }
        }

        let proxy_server = ProxyServer::new(config.proxy.port, ca, config.proxy.mark);
        proxy_handle = Some(tokio::spawn(async move {
            if let Err(e) = proxy_server.run().await {
                eprintln!("Proxy Server error: {}", e);
            }
        }));
    }

    // 2. Initialize Aria2 Client
    let aria2_url = "http://127.0.0.1:6800/jsonrpc".to_string();
    let aria2_secret = None;
    let aria2_client: Arc<dyn crate::aria2::client::Aria2ClientTrait> = Arc::new(crate::aria2::client::Aria2Client::new(aria2_url, aria2_secret));

    // 3. Initialize Queue Manager
    let queue_manager = Arc::new(QueueManager::new(pool, aria2_client, config.max_concurrent_downloads));
    queue_manager.load_from_db().await?;

    // 4. Initialize Scheduler Engine
    let scheduler_engine = Arc::new(SchedulerEngine::new(queue_manager.clone()));
    let sched_engine_clone = scheduler_engine.clone();
    let scheduler_handle = tokio::spawn(async move {
        if let Err(e) = sched_engine_clone.run().await {
            eprintln!("Scheduler Engine error: {}", e);
        }
    });

    // 5. Start RPC Server
    let socket_path = proj_dirs.runtime_dir()
        .map(|d| d.join("jadm.sock"))
        .unwrap_or_else(|| {
            #[cfg(unix)]
            { std::path::PathBuf::from("/tmp/jadm.sock") }
            #[cfg(not(unix))]
            { std::env::temp_dir().join("jadm.sock") }
        });
    
    // Ensure parent directory exists for socket
    if let Some(parent) = socket_path.parent() {
        let is_temp = parent == std::path::Path::new("/tmp") || parent == std::env::temp_dir();
        if !is_temp {
            fs::create_dir_all(parent)?;
        }
    }

    let socket_path_str = socket_path.to_string_lossy().to_string();
    let rpc_server = UnixRpcServer::new(queue_manager.clone(), config.clone(), socket_path_str);
    
    let rpc_handle = tokio::spawn(async move {
        if let Err(e) = rpc_server.run().await {
            eprintln!("RPC Server error: {}", e);
        }
    });

    // Start SOCKS5 Proxy Server on port 6247
    let socks_token = uuid::Uuid::new_v4().to_string().replace("-", "");
    let token_path = proj_dirs.runtime_dir()
        .map(|d| d.join("jadm_socks_token"))
        .unwrap_or_else(|| {
            #[cfg(unix)]
            { std::path::PathBuf::from("/tmp/jadm_socks_token") }
            #[cfg(not(unix))]
            { std::env::temp_dir().join("jadm_socks_token") }
        });

    let tmp_path = token_path.with_extension("tmp");
    let token_clone = socks_token.clone();
    let tmp_clone = tmp_path.clone();
    let dest_clone = token_path.clone();

    tokio::task::spawn_blocking(move || {
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp_clone)
                .expect("Failed to create secure token file");
            use std::io::Write;
            f.write_all(token_clone.as_bytes()).unwrap();
        }
        #[cfg(not(unix))]
        {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_clone)
                .expect("Failed to create token file");
            use std::io::Write;
            f.write_all(token_clone.as_bytes()).unwrap();
        }
        std::fs::rename(&tmp_clone, &dest_clone).unwrap();
    }).await.unwrap();

    let socks_server = rpc::socks::Socks5Server::new(6247, socks_token);
    let socks_handle = tokio::spawn(async move {
        if let Err(e) = socks_server.run().await {
            eprintln!("SOCKS5 Server error: {}", e);
        }
    });

    // 6. Start Clipboard Monitor
    let clipboard_monitor = ClipboardMonitor::new(queue_manager.clone());
    let clipboard_handle = tokio::spawn(async move {
        if let Err(e) = clipboard_monitor.run().await {
            eprintln!("Clipboard Monitor error: {}", e);
        }
    });

    // 7. Background Polling & Cleanup Loop
    let qm_poll = queue_manager.clone();
    let poll_handle = tokio::spawn(async move {
        loop {
            if let Err(e) = qm_poll.tick().await {
                eprintln!("Polling error: {}", e);
            }
            sleep(Duration::from_millis(1000)).await;
        }
    });

    let qm_cleanup = queue_manager.clone();
    let cleanup_handle = tokio::spawn(async move {
        loop {
            // Wait 1 hour between cleanups
            sleep(Duration::from_secs(3600)).await;
            if let Err(e) = qm_cleanup.cleanup_old_history(30).await {
                eprintln!("Cleanup error: {}", e);
            }
        }
    });

    println!("jadm-daemon is running.");
    
    // Set up signal handling for cleanup
    let shutdown_signal = async {
        #[cfg(unix)]
        {
            let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
            let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = sigint.recv() => println!("Received SIGINT"),
                _ = sigterm.recv() => println!("Received SIGTERM"),
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
            println!("Received Ctrl-C / Shutdown signal");
        }
    };

    // Wait for critical tasks or signal
    tokio::select! {
        res = rpc_handle => res?,
        res = socks_handle => res?,
        res = poll_handle => res?,
        res = cleanup_handle => res?,
        res = scheduler_handle => res?,
        res = clipboard_handle => res?,
        res = async { proxy_handle.unwrap_or(tokio::spawn(async { loop { sleep(Duration::from_secs(3600)).await; } })).await }, if proxy_handle.is_some() => res?,
        _ = shutdown_signal => {}
    }

    // Cleanup
    if let Some(net_intercepter) = intercepter {
        if let Err(e) = net_intercepter.teardown() {
            eprintln!("Failed to teardown network interception: {}", e);
        }
    }

    println!("jadm-daemon shutting down.");
    Ok(())
}
