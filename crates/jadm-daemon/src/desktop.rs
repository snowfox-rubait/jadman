use anyhow::{Result, anyhow};
use std::process::Command;
use std::path::PathBuf;

pub async fn install_desktop_handler() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        println!("============================================================");
        println!("WARNING: This sets JADMan as your default system-wide browser.");
        println!("Every HTTP/HTTPS link clicked in any app will go through JADMan.");
        println!("To revert, run: jadm-daemon uninstall-desktop-handler");
        println!("============================================================");

        let home = std::env::var("HOME")?;
        let applications_dir = PathBuf::from(home).join(".local/share/applications");
        tokio::fs::create_dir_all(&applications_dir).await?;

        let desktop_file_path = applications_dir.join("jadman-handler.desktop");
        
        // Find the absolute path to the current executable
        let current_exe = std::env::current_exe()?;
        let exe_path_str = current_exe.to_string_lossy();

        let desktop_content = format!(
            "[Desktop Entry]\n\
            Version=1.0\n\
            Name=JADMan Smart Handler\n\
            GenericName=Download Manager\n\
            Comment=Intercepts media downloads and forwards HTML to the browser\n\
            Exec={} handle-url %U\n\
            Terminal=false\n\
            Type=Application\n\
            MimeType=x-scheme-handler/http;x-scheme-handler/https;\n\
            Categories=Network;FileTransfer;\n\
            NoDisplay=true",
            exe_path_str
        );

        tokio::fs::write(&desktop_file_path, desktop_content).await?;
        println!("Installed desktop file at {:?}", desktop_file_path);

        // Register with xdg-mime
        let status = tokio::process::Command::new("xdg-mime")
            .args(&["default", "jadman-handler.desktop", "x-scheme-handler/http"])
            .status()
            .await?;
        if !status.success() {
            eprintln!("Warning: Failed to set xdg-mime default for http");
        }

        let status = tokio::process::Command::new("xdg-mime")
            .args(&["default", "jadman-handler.desktop", "x-scheme-handler/https"])
            .status()
            .await?;
        if !status.success() {
            eprintln!("Warning: Failed to set xdg-mime default for https");
        }

        println!("Successfully registered JADMan as the default URL handler.");
    }
    #[cfg(not(target_os = "linux"))]
    {
        println!("Desktop handler integration is currently only supported on Linux platforms.");
    }
    Ok(())
}

pub async fn uninstall_desktop_handler() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        println!("Uninstalling JADMan desktop handler...");
        let home = std::env::var("HOME")?;
        let desktop_file_path = PathBuf::from(home).join(".local/share/applications/jadman-handler.desktop");
        
        let _ = tokio::fs::remove_file(desktop_file_path).await;
        
        println!("To fully restore your browser, please set it as default again.");
        println!("For example: xdg-mime default firefox.desktop x-scheme-handler/http x-scheme-handler/https");
    }
    #[cfg(not(target_os = "linux"))]
    {
        println!("Desktop handler integration is currently only supported on Linux platforms.");
    }
    Ok(())
}

pub async fn handle_url(url: &str) -> Result<()> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        eprintln!("JADMan: unsupported scheme, ignoring: {}", url);
        return Ok(());
    }

    println!("JADMan Smart Handler inspecting URL: {}", url);

    // First check if it's a known media extension just by string
    let lower_url = url.to_lowercase();
    let is_media_ext = lower_url.ends_with(".mp4") || lower_url.ends_with(".mkv") || lower_url.ends_with(".zip") || lower_url.ends_with(".iso") || lower_url.ends_with(".tar.gz");

    let mut should_download = is_media_ext;

    if !should_download {
        // If not obvious, do a quick HEAD request
        let client = reqwest::Client::new();
        match client.head(url).send().await {
            Ok(resp) => {
                if let Some(ct) = resp.headers().get(reqwest::header::CONTENT_TYPE) {
                    if let Ok(ct_str) = ct.to_str() {
                        let ct_lower = ct_str.to_lowercase();
                        if ct_lower.starts_with("video/") || 
                           ct_lower.starts_with("audio/") || 
                           ct_lower.contains("application/octet-stream") ||
                           ct_lower.contains("application/zip") ||
                           ct_lower.contains("application/x-") {
                            should_download = true;
                        }
                    }
                }
                
                if let Some(content_disp) = resp.headers().get(reqwest::header::CONTENT_DISPOSITION) {
                    if let Ok(cd) = content_disp.to_str() {
                        if cd.to_lowercase().contains("attachment") {
                            should_download = true;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("HEAD request failed: {}", e);
                // Fall back to browser on error
            }
        }
    }

    if should_download {
        println!("Intercepted! Sending to JADMan daemon...");
        // Send to local JADMan daemon via Unix socket to avoid CORS/HTTP bypass issues
        let proj_dirs = directories::ProjectDirs::from("com", "jadm", "jadm")
            .ok_or_else(|| anyhow::anyhow!("Could not determine project directories"))?;
        let socket_path = proj_dirs.runtime_dir()
            .map(|d| d.join("jadm.sock"))
            .unwrap_or_else(|| {
                #[cfg(unix)]
                {
                    std::path::PathBuf::from(format!("/run/user/{}/jadm/jadm.sock", unsafe { libc::geteuid() }))
                }
                #[cfg(not(unix))]
                {
                    std::env::temp_dir().join("jadm.sock")
                }
            });
            
        let payload = serde_json::json!({
            "cmd": "AddDownload",
            "url": url
        });
        
        match tokio::net::UnixStream::connect(&socket_path).await {
            Ok(mut stream) => {
                use tokio::io::AsyncWriteExt;
                let mut data = serde_json::to_vec(&payload).unwrap();
                data.push(b'\n');
                if let Err(e) = stream.write_all(&data).await {
                    eprintln!("Failed to write to Unix socket: {}", e);
                } else {
                    println!("Download successfully queued in JADMan.");
                }
            }
            Err(e) => {
                eprintln!("Failed to queue download. Is jadm-daemon running? (Socket error: {})", e);
            }
        }
    } else {
        println!("Not a direct download. Forwarding to browser...");
        open_in_real_browser(url)?;
    }

    Ok(())
}

fn open_in_real_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        // We cannot use xdg-open because it would loop back to us.
        // Read ~/.config/mimeapps.list to find the actual handler for text/html
        let home = std::env::var("HOME")?;
        let mime_path = PathBuf::from(home).join(".config/mimeapps.list");
        
        let mut real_browser = String::new();
        
        if let Ok(content) = std::fs::read_to_string(&mime_path) {
            let mut in_default_apps = false;
            for line in content.lines() {
                let line = line.trim();
                if line == "[Default Applications]" {
                    in_default_apps = true;
                    continue;
                }
                if line.starts_with('[') {
                    in_default_apps = false;
                    continue;
                }
                if in_default_apps && line.starts_with("text/html=") {
                    let apps = line.trim_start_matches("text/html=");
                    for app in apps.split(';') {
                        if !app.is_empty() && app != "jadman-handler.desktop" {
                            real_browser = app.to_string();
                            break;
                        }
                    }
                    if !real_browser.is_empty() {
                        break;
                    }
                }
            }
        }
        
        // If not found, try sensible defaults
        let desktop_file = if real_browser.is_empty() {
            "firefox.desktop".to_string()
        } else {
            real_browser.clone()
        };
        
        // Launch using gtk-launch which correctly parses the .desktop file's Exec line
        println!("Spawning browser via desktop entry: {}", desktop_file);
        if let Err(_) = Command::new("gtk-launch").arg(&desktop_file).arg(url).spawn() {
            eprintln!("gtk-launch not found or failed to spawn. Please ensure gtk3 is installed.");
            eprintln!("Or open this URL manually: {}", url);
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        println!("Spawning real browser via macos open command...");
        Command::new("open").arg(url).spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        println!("Spawning real browser via Windows rundll32 FileProtocolHandler...");
        Command::new("rundll32").args(&["url.dll,FileProtocolHandler", url]).spawn()?;
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        eprintln!("URL routing unsupported on this OS. URL: {}", url);
    }
    
    Ok(())
}
