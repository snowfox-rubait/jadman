use anyhow::{Result, anyhow};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use regex::Regex;
use std::sync::OnceLock;

static RE_PROGRESS: OnceLock<Regex> = OnceLock::new();
static RE_STREAMING: OnceLock<Regex> = OnceLock::new();

pub struct ProgressUpdate {
    pub percent: u8,
    pub total_size: Option<u64>,
    pub rate_bytes: u64,
    pub eta_secs: Option<u64>,
    pub filename: Option<String>,
    pub downloaded_bytes: Option<u64>,
}

pub async fn run_ytdlp(url: &str, folder: &str, format: Option<String>, cookies: Option<String>, netscape_cookies: Option<String>, user_agent: Option<String>, ghost_mode: bool, write_subs: bool, embed_thumbnail: bool, embed_chapters: bool, live_from_start: bool, compress_video: bool, download_playlist: bool, referer: Option<String>, write_description: bool, on_spawn: impl FnOnce(u32), mut on_progress: impl FnMut(ProgressUpdate)) -> Result<()> {
    let mut cmd = Command::new(get_ytdlp_path());
    cmd.arg("--newline")
       .arg("--progress")
       .arg("--continue")      // always resume partial downloads
       .arg("-o").arg(format!("{}/%(title)s.%(ext)s", folder));

    if download_playlist {
        cmd.arg("--yes-playlist");
    } else {
        cmd.arg("--no-playlist");
    }
 
    if let Some(ref r) = referer {
        cmd.arg("--referer").arg(r);
    }

    if live_from_start {
        cmd.arg("--live-from-start");
    }

    if let Some(f) = format {
        cmd.arg("-f").arg(f);
    }

    if write_subs {
        cmd.arg("--write-subs")
           .arg("--write-auto-subs")
           .arg("--embed-subs");
    }

    if embed_thumbnail {
        cmd.arg("--embed-thumbnail")
           .arg("--write-thumbnail");
    }

    if embed_chapters {
        cmd.arg("--embed-chapters")
           .arg("--write-info-json");
    }

    if write_description {
        cmd.arg("--write-description");
    }

    if compress_video {
        let exec_cmd = r#"sh -c 'ffmpeg -y -i "$1" -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p -c:a copy "${1%.*}.compressed.mp4" && rm "$1" && mv "${1%.*}.compressed.mp4" "${1%.*}.mp4"' dummy {}"#;
        cmd.arg("--exec").arg(format!("after_move:{}", exec_cmd));
    }

    let mut _cookie_file_guard = None;
    if let Some(nc) = netscape_cookies {
        let nc_trimmed = nc.trim();
        if !nc_trimmed.is_empty() {
            if let Ok(mut temp) = tempfile::NamedTempFile::new() {
                use std::io::Write;
                if temp.write_all(nc_trimmed.as_bytes()).is_ok() {
                    cmd.arg("--cookies").arg(temp.path());
                    _cookie_file_guard = Some(temp);
                }
            }
        }
    }

    if _cookie_file_guard.is_none() {
        if let Some(c) = cookies {
            let c_trimmed = c.trim();
            if !c_trimmed.is_empty() {
                cmd.arg("--add-header").arg(format!("Cookie:{}", c_trimmed));
            }
        }
    }
    
    if let Some(ref ua) = user_agent {
        cmd.arg("--user-agent").arg(ua);
    }

    let impersonate_target = get_impersonate_target(user_agent.as_deref());
    cmd.arg("--impersonate").arg(impersonate_target);

    // Ghost Mode: enable generic impersonation extractor args for Cloudflare-protected direct URLs.
    // This tells yt-dlp's generic extractor to use the curl-cffi TLS impersonation engine
    // even for direct file URLs (not just recognized platform extractors).
    if ghost_mode {
        cmd.arg("--extractor-args").arg("generic:impersonate");
        let token = crate::config::get_socks_token()?;
        cmd.arg("--proxy").arg(format!("socks5://jadm:{}@127.0.0.1:6247", token));
        eprintln!("[JADMan Ghost] Activating generic impersonation and SOCKS5 proxy for: {}", url);
    }

    cmd.arg("--");
    cmd.arg(url);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    if let Some(pid) = child.id() {
        on_spawn(pid);
    }
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let re_progress = RE_PROGRESS.get_or_init(|| {
        Regex::new(r"\[download\]\s+([\d.]+)%\s+of\s+~?([\d.]+)(KiB|MiB|GiB|B)\s+at\s+([\d.]+)(KiB/s|MiB/s|GiB/s|B/s)\s+ETA\s+(\d+:?\d*:?\d*)").unwrap()
    });
    let re_streaming = RE_STREAMING.get_or_init(|| {
        Regex::new(r"\[download\]\s+([\d.]+)(KiB|MiB|GiB|B)\s+at\s+([\d.]+)(KiB/s|MiB/s|GiB/s|B/s)\s+\(([\d:]+)\)").unwrap()
    });
    static RE_FFMPEG: OnceLock<Regex> = OnceLock::new();
    let re_ffmpeg = RE_FFMPEG.get_or_init(|| {
        Regex::new(r"frame=\s*\d+\s+fps=\s*[\d.]+\s+q=[-\d.]+\s+size=\s*([\d.]+)(KiB|MiB|GiB|B)\s+time=[\d:.]+\s+bitrate=\s*([\d.]+)(kbits/s|mbits/s|bits/s|kb/s|mb/s)\s+speed=\s*([\d.]+)x").unwrap()
    });

    #[derive(Debug)]
    enum LineSource {
        Stdout(String),
        Stderr(String),
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    let tx_out = tx.clone();
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = tx_out.send(LineSource::Stdout(line)).await;
        }
    });

    let tx_err = tx.clone();
    let stderr_handle = tokio::spawn(async move {
        let mut lines = Vec::new();
        let mut err_reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = err_reader.next_line().await {
            eprintln!("[yt-dlp stderr] {}", line);
            let _ = tx_err.send(LineSource::Stderr(line.clone())).await;
            lines.push(line);
        }
        lines
    });

    // Drop original sender so channel closes when spawned tasks drop their senders
    drop(tx);

    while let Some(msg) = rx.recv().await {
        match msg {
            LineSource::Stdout(line) => {
                let line_trimmed = line.trim();
                let mut extracted_file = None;
                if line_trimmed.starts_with("[download] Destination:") {
                    extracted_file = Some(line_trimmed["[download] Destination:".len()..].trim().to_string());
                } else if line_trimmed.contains("Merging formats into") {
                    if let Some(pos) = line_trimmed.find("Merging formats into") {
                        let suffix = line_trimmed[pos + "Merging formats into".len()..].trim();
                        let cleaned = suffix.trim_matches('"').trim_matches('\'').to_string();
                        extracted_file = Some(cleaned);
                    }
                } else if line_trimmed.contains("Converting video from") && line_trimmed.contains("to") {
                    if let Some(pos) = line_trimmed.rfind(" to ") {
                        let suffix = line_trimmed[pos + " to ".len()..].trim();
                        let cleaned = suffix.trim_matches('"').trim_matches('\'').to_string();
                        extracted_file = Some(cleaned);
                    }
                }

                if let Some(path_str) = extracted_file {
                    let path = std::path::Path::new(&path_str);
                    if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
                        on_progress(ProgressUpdate {
                            percent: 0,
                            total_size: None,
                            rate_bytes: 0,
                            eta_secs: None,
                            filename: Some(fname.to_string()),
                            downloaded_bytes: None,
                        });
                    }
                }

                if let Some(caps) = re_progress.captures(&line) {
                    let percent: f64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0.0);
                    let size_val: f64 = caps.get(2).unwrap().as_str().parse().unwrap_or(0.0);
                    let size_unit = caps.get(3).unwrap().as_str();
                    let rate_val: f64 = caps.get(4).unwrap().as_str().parse().unwrap_or(0.0);
                    let rate_unit = caps.get(5).unwrap().as_str();
                    let eta_str = caps.get(6).unwrap().as_str();
                    
                    let total_size = parse_size(size_val, size_unit);
                    let rate_bytes = parse_rate(rate_val, rate_unit);
                    let eta_secs = parse_eta(eta_str);
                    let downloaded_bytes = ((percent / 100.0) * total_size as f64) as u64;
                    
                    on_progress(ProgressUpdate {
                        percent: percent as u8,
                        total_size: Some(total_size),
                        rate_bytes,
                        eta_secs,
                        filename: None,
                        downloaded_bytes: Some(downloaded_bytes),
                    });
                } else if let Some(caps) = re_streaming.captures(&line) {
                    let size_val: f64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0.0);
                    let size_unit = caps.get(2).unwrap().as_str();
                    let rate_val: f64 = caps.get(3).unwrap().as_str().parse().unwrap_or(0.0);
                    let rate_unit = caps.get(4).unwrap().as_str();
                    
                    let downloaded_bytes = parse_size(size_val, size_unit);
                    let rate_bytes = parse_rate(rate_val, rate_unit);
                    
                    on_progress(ProgressUpdate {
                        percent: 0,
                        total_size: None,
                        rate_bytes,
                        eta_secs: None,
                        filename: None,
                        downloaded_bytes: Some(downloaded_bytes),
                    });
                }
            }
            LineSource::Stderr(line) => {
                if let Some(caps) = re_ffmpeg.captures(&line) {
                    let size_val: f64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0.0);
                    let size_unit = caps.get(2).unwrap().as_str();
                    let rate_val: f64 = caps.get(3).unwrap().as_str().parse().unwrap_or(0.0);
                    let rate_unit = caps.get(4).unwrap().as_str();
                    
                    let downloaded_bytes = parse_size(size_val, size_unit);
                    let rate_bytes = match rate_unit {
                        "kbits/s" | "kb/s" => (rate_val * 1000.0 / 8.0) as u64,
                        "mbits/s" | "mb/s" => (rate_val * 1000000.0 / 8.0) as u64,
                        _ => (rate_val / 8.0) as u64,
                    };
                    
                    on_progress(ProgressUpdate {
                        percent: 0,
                        total_size: None,
                        rate_bytes,
                        eta_secs: None,
                        filename: None,
                        downloaded_bytes: Some(downloaded_bytes),
                    });
                }
            }
        }
    }

    let status = child.wait().await?;
    let stderr_lines = stderr_handle.await.unwrap_or_default();
    
    if status.success() {
        Ok(())
    } else {
        // Surface the actual yt-dlp error for diagnosis
        let err_msg = stderr_lines.iter()
            .filter(|l| l.contains("ERROR"))
            .next_back()
            .cloned()
            .unwrap_or_else(|| format!("yt-dlp failed with status {}", status));
        Err(anyhow!("{}", err_msg))
    }
}

fn parse_size(val: f64, unit: &str) -> u64 {
    match unit {
        "KiB" => (val * 1024.0) as u64,
        "MiB" => (val * 1024.0 * 1024.0) as u64,
        "GiB" => (val * 1024.0 * 1024.0 * 1024.0) as u64,
        _ => val as u64,
    }
}

fn parse_rate(val: f64, unit: &str) -> u64 {
    match unit {
        "KiB/s" => (val * 1024.0) as u64,
        "MiB/s" => (val * 1024.0 * 1024.0) as u64,
        "GiB/s" => (val * 1024.0 * 1024.0 * 1024.0) as u64,
        _ => val as u64,
    }
}

fn parse_eta(eta_str: &str) -> Option<u64> {
    let parts: Vec<&str> = eta_str.split(':').collect();
    match parts.len() {
        1 => parts[0].parse().ok(),
        2 => {
            let m: u64 = parts[0].parse().unwrap_or(0);
            let s: u64 = parts[1].parse().unwrap_or(0);
            Some(m * 60 + s)
        }
        3 => {
            let h: u64 = parts[0].parse().unwrap_or(0);
            let m: u64 = parts[1].parse().unwrap_or(0);
            let s: u64 = parts[2].parse().unwrap_or(0);
            Some(h * 3600 + m * 60 + s)
        }
        _ => None,
    }
}

pub fn get_ytdlp_path() -> std::path::PathBuf {
    if let Ok(path) = which::which("yt-dlp") {
        path
    } else {
        std::path::PathBuf::from("yt-dlp")
    }
}

pub fn get_impersonate_target(user_agent: Option<&str>) -> &'static str {
    if let Some(ua) = user_agent {
        let ua = ua.to_lowercase();
        if ua.contains("edg") {
            "edge"
        } else if ua.contains("firefox") {
            "firefox"
        } else if ua.contains("safari") && !ua.contains("chrome") && !ua.contains("chromium") {
            "safari"
        } else {
            "chrome"
        }
    } else {
        "chrome"
    }
}

pub async fn get_formats(
    url: &str,
    cookies: Option<String>,
    netscape_cookies: Option<String>,
    user_agent: Option<String>,
    mode: Option<String>,
    referer: Option<String>,
) -> Result<Vec<jadm_common::protocol::FormatInfo>> {
    let mut cmd = Command::new(get_ytdlp_path());
    cmd.arg("-J") // Dump JSON metadata
       .arg("--no-playlist");
 
    if let Some(ref r) = referer {
        cmd.arg("--referer").arg(r);
    }

    let ghost_mode = mode.as_deref() == Some("ghost");

    let mut _cookie_file_guard = None;
    if let Some(nc) = netscape_cookies {
        let nc_trimmed = nc.trim();
        if !nc_trimmed.is_empty() {
            if let Ok(mut temp) = tempfile::NamedTempFile::new() {
                use std::io::Write;
                if temp.write_all(nc_trimmed.as_bytes()).is_ok() {
                    cmd.arg("--cookies").arg(temp.path());
                    _cookie_file_guard = Some(temp);
                }
            }
        }
    }

    if _cookie_file_guard.is_none() {
        if let Some(c) = cookies {
            let c_trimmed = c.trim();
            if !c_trimmed.is_empty() {
                cmd.arg("--add-header").arg(format!("Cookie:{}", c_trimmed));
            }
        }
    }
    
    if let Some(ref ua) = user_agent {
        cmd.arg("--user-agent").arg(ua);
    }

    let impersonate_target = get_impersonate_target(user_agent.as_deref());
    cmd.arg("--impersonate").arg(impersonate_target);

    if ghost_mode {
        cmd.arg("--extractor-args").arg("generic:impersonate");
        let token = crate::config::get_socks_token()?;
        cmd.arg("--proxy").arg(format!("socks5://jadm:{}@127.0.0.1:6247", token));
    }

    cmd.arg("--");
    cmd.arg(url);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    
    let mut stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
    let mut buffer = Vec::new();
    use tokio::io::AsyncReadExt;
    stdout.read_to_end(&mut buffer).await?;
    
    let status = child.wait().await?;
    if !status.success() {
        return Err(anyhow!("yt-dlp exited with status {}", status));
    }

    let json_val: serde_json::Value = serde_json::from_slice(&buffer)?;
    
    let formats = if let Some(formats_arr) = json_val.get("formats").and_then(|f| f.as_array()) {
        let mut list = Vec::new();
        for f in formats_arr {
            let id = f.get("format_id")
                .and_then(|v| v.as_str().map(|s| s.to_string())
                             .or_else(|| v.as_i64().map(|n| n.to_string())))
                .unwrap_or_default();
            
            if id.is_empty() {
                continue;
            }

            let ext = f.get("ext")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let height = f.get("height").and_then(|v| v.as_i64());
            let width = f.get("width").and_then(|v| v.as_i64());
            let resolution = f.get("resolution")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| {
                    if let (Some(w), Some(h)) = (width, height) {
                        format!("{}x{}", w, h)
                    } else if height.is_some() {
                        format!("{}p", height.unwrap())
                    } else {
                        "audio only".to_string()
                    }
                });

            let note = f.get("format_note")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .or_else(|| f.get("note").and_then(|v| v.as_str().map(|s| s.to_string())))
                .unwrap_or_else(|| {
                    let vcodec = f.get("vcodec").and_then(|v| v.as_str()).unwrap_or("none");
                    let acodec = f.get("acodec").and_then(|v| v.as_str()).unwrap_or("none");
                    if vcodec == "none" {
                        format!("audio ({})", acodec)
                    } else {
                        format!("video ({})", vcodec)
                    }
                });

            list.push(jadm_common::protocol::FormatInfo {
                id,
                resolution,
                ext,
                note,
            });
        }
        list
    } else {
        Vec::new()
    };

    Ok(formats)
}
