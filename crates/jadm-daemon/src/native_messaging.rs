use anyhow::Result;
use jadm_common::protocol::{Request, Response};
use std::io::{self, Read, Write};
#[cfg(unix)]
use tokio::net::UnixStream;
#[cfg(not(unix))]
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use directories::ProjectDirs;

/// Reads a 4-byte length prefix from stdin, then reads the JSON message.
fn read_native_message() -> Result<Option<Vec<u8>>> {
    let mut length_bytes = [0u8; 4];
    let mut stdin = io::stdin();
    match stdin.read_exact(&mut length_bytes) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }

    let length = u32::from_ne_bytes(length_bytes) as usize;
    if length > 10 * 1024 * 1024 { // 10MB limit
        return Err(anyhow::anyhow!("Message too large"));
    }

    let mut message_bytes = vec![0u8; length];
    stdin.read_exact(&mut message_bytes)?;
    Ok(Some(message_bytes))
}

/// Writes a JSON message to stdout prefixed by a 4-byte length.
fn write_native_message(message: &[u8]) -> Result<()> {
    let length = message.len() as u32;
    let mut stdout = io::stdout();
    stdout.write_all(&length.to_ne_bytes())?;
    stdout.write_all(message)?;
    stdout.flush()?;
    Ok(())
}

pub async fn run_native_host() -> Result<()> {
    // Determine the path to the Unix IPC socket
    let proj_dirs = ProjectDirs::from("com", "jadm", "jadm")
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

    // Connect to the daemon (spawn if not running)
    #[cfg(unix)]
    let mut stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(_) => {
            let exe_path = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("/usr/bin/jadm-daemon"));
            let mut cmd = std::process::Command::new(&exe_path);
            cmd.stdin(std::process::Stdio::null());
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
            
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
            
            match cmd.spawn() {
                Ok(_) => {
                    // Give it a moment to initialize the socket and check/spawn aria2c
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    UnixStream::connect(&socket_path).await
                        .map_err(|e| anyhow::anyhow!("Failed to connect to JADMan daemon after auto-launch: {}", e))?
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to auto-spawn JADMan daemon: {}", e));
                }
            }
        }
    };

    #[cfg(not(unix))]
    let mut stream = match TcpStream::connect("127.0.0.1:6245").await {
        Ok(s) => s,
        Err(_) => {
            let exe_path = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("jadm-daemon"));
            let mut cmd = std::process::Command::new(&exe_path);
            cmd.stdin(std::process::Stdio::null());
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
            
            match cmd.spawn() {
                Ok(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    TcpStream::connect("127.0.0.1:6245").await
                        .map_err(|e| anyhow::anyhow!("Failed to connect to JADMan daemon after auto-launch: {}", e))?
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to auto-spawn JADMan daemon: {}", e));
                }
            }
        }
    };
    
    // We split the stream to handle read/write simultaneously
    let (reader, mut writer) = stream.split();
    let mut unix_reader = BufReader::new(reader).lines();

    // Native messaging loop
    loop {
        // Read a message from Chrome
        let msg_bytes = match tokio::task::spawn_blocking(read_native_message).await?? {
            Some(bytes) => bytes,
            None => break, // EOF, Chrome disconnected
        };

        // Forward to the Unix socket (append newline)
        let mut to_send = msg_bytes.clone();
        to_send.push(b'\n');
        writer.write_all(&to_send).await?;

        // Wait for response from the Unix socket
        if let Some(line) = unix_reader.next_line().await? {
            // Forward response back to Chrome
            write_native_message(line.as_bytes())?;
        }
    }

    Ok(())
}

pub fn install_native_manifest() -> Result<()> {
    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_string_lossy();

    // Chrome/Chromium/Brave manifest format (uses allowed_origins)
    let chrome_manifest = serde_json::json!({
        "name": "com.jadm.jadm",
        "description": "JADMan Native Messaging Host",
        "path": exe_path_str,
        "type": "stdio",
        "allowed_origins": [
            "chrome-extension://ipiefkjcicogeoepimgebinafoelhbhd/"
        ]
    });

    // Firefox manifest format (uses allowed_extensions)
    let firefox_manifest = serde_json::json!({
        "name": "com.jadm.jadm",
        "description": "JADMan Native Messaging Host",
        "path": exe_path_str,
        "type": "stdio",
        "allowed_extensions": [
            "jadm@snowfox.com"
        ]
    });

    let chrome_str = serde_json::to_string_pretty(&chrome_manifest)?;
    let firefox_str = serde_json::to_string_pretty(&firefox_manifest)?;

    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var("APPDATA")?;
        let jadman_dir = std::path::PathBuf::from(app_data).join("JADMan");
        std::fs::create_dir_all(&jadman_dir)?;

        let chrome_manifest_path = jadman_dir.join("com.jadm.jadm.json");
        let firefox_manifest_path = jadman_dir.join("com.jadm.jadm_firefox.json");

        std::fs::write(&chrome_manifest_path, &chrome_str)?;
        std::fs::write(&firefox_manifest_path, &firefox_str)?;

        // Register in Windows Registry via reg.exe command (needs no extra dependency crates)
        let _ = std::process::Command::new("reg")
            .args(&["add", "HKCU\\Software\\Google\\Chrome\\NativeMessagingHosts\\com.jadm.jadm", "/ve", "/t", "REG_SZ", "/d", &chrome_manifest_path.to_string_lossy(), "/f"])
            .status();
        let _ = std::process::Command::new("reg")
            .args(&["add", "HKCU\\Software\\Mozilla\\NativeMessagingHosts\\com.jadm.jadm", "/ve", "/t", "REG_SZ", "/d", &firefox_manifest_path.to_string_lossy(), "/f"])
            .status();

        println!("Native messaging manifest registered in Windows Registry.");
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME")?;
        let is_mac = cfg!(target_os = "macos");

        let (chrome_dir, chromium_dir, brave_dir, firefox_dir) = if is_mac {
            let base = std::path::PathBuf::from(&home).join("Library/Application Support");
            (
                base.join("Google/Chrome/NativeMessagingHosts"),
                base.join("Chromium/NativeMessagingHosts"),
                base.join("BraveSoftware/Brave-Browser/NativeMessagingHosts"),
                base.join("Mozilla/NativeMessagingHosts"),
            )
        } else {
            let base = std::path::PathBuf::from(&home).join(".config");
            (
                base.join("google-chrome/NativeMessagingHosts"),
                base.join("chromium/NativeMessagingHosts"),
                base.join("BraveSoftware/Brave-Browser/NativeMessagingHosts"),
                std::path::PathBuf::from(&home).join(".mozilla/native-messaging-hosts"),
            )
        };

        for dir in &[chrome_dir, chromium_dir, brave_dir] {
            if let Ok(_) = std::fs::create_dir_all(dir) {
                let manifest_path = dir.join("com.jadm.jadm.json");
                let _ = std::fs::write(manifest_path, &chrome_str);
            }
        }

        if let Ok(_) = std::fs::create_dir_all(&firefox_dir) {
            let manifest_path = firefox_dir.join("com.jadm.jadm.json");
            let _ = std::fs::write(manifest_path, &firefox_str);
        }

        println!("Native messaging manifest installed to Chrome/Chromium/Brave/Firefox directories.");
    }

    Ok(())
}
