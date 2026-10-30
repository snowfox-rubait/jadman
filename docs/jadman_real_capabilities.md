# JADMan Codebase Audit: Real Capabilities vs Stubs & Claims

This report documents the verified technical reality of the JADMan codebase at commit [`161c1c3`](file:///home/test/Projects/jadman/). Every subsystem was audited line-by-line across Rust crates, browser extensions, Lua scripts, and auxiliary tools.

---

## 1. Executive Summary

| Subsystem / Feature | Claimed / Documented | Actual Implementation Status | Key Evidence |
| :--- | :--- | :--- | :--- |
| **Multi-Engine Routing** | aria2c, yt-dlp, Camoufox, Browser Fetch | **Fully Functional** | [`crates/jadm-daemon/src/queue/manager.rs`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/queue/manager.rs#L300-L400) |
| **Aria2c Auto-Spawn** | Automatic local RPC management | **Fully Functional** | Spawns `aria2c --enable-rpc` on port 6800 automatically on startup. |
| **yt-dlp Integration** | Progress streaming, format detection, live start, subtitle & chapter embedding | **Fully Functional** | [`crates/jadm-daemon/src/ytdlp/runner.rs`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/ytdlp/runner.rs) parses JSON formats & streams line-by-line progress. |
| **Seekable Media Streaming** | Stream in-progress video to VLC/mpv | **Fully Functional** | Axum route `/stream/:id` using `tower_http::services::ServeFile` with byte-range headers. |
| **Arr Stack (Sonarr/Radarr) Integration** | qBittorrent WebAPI v2 emulation | **Fully Functional** | Implements `/api/v2/*` endpoints on port 6246 ([`rpc/http.rs`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/rpc/http.rs#L80-L160)). |
| **Browser Native Messaging** | Bridge Chrome/Firefox to daemon socket | **Fully Functional** | [`crates/jadm-daemon/src/native_messaging.rs`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/native_messaging.rs) + auto-manifest generation. |
| **Browser Media Sniffing & Siphoning** | In-browser segment stitching, monkey-patched XHR/fetch siphoning | **Fully Functional** | 838 lines in `background.js`, 1507 lines in `content.js`, 255 lines in `siphon_hook.js`. |
| **OS-Level URL Interception** | Become default browser and route downloads | **Fully Functional (Linux)** | [`crates/jadm-daemon/src/desktop.rs`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/desktop.rs) generates `.desktop`, probes via HEAD, falls back to `gtk-launch`. |
| **CookieJar Integration** | Auto-decrypt authenticated browser cookies | **Fully Functional** | SQLite lookup in `~/.config/CookieJar/jar.db` + invokes `cookie-jar use` with `COOKIE_JAR_PASSWORD`. |
| **Lua Worker & Plugin System** | Dynamic plugins running in background thread pool | **Fully Functional** | [`crates/jadm-daemon/src/lua/mod.rs`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/lua/mod.rs) (383 lines) via `mlua`; real plugins for Real-Debrid, Wayback Machine, Telegram. |
| **SOCKS5 Proxy Server** | Custom proxy with token authentication | **Fully Functional** | [`crates/jadm-daemon/src/rpc/socks.rs`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/rpc/socks.rs) (196 lines) on port 6247. |
| **TLS MITM Proxy Engine** | Dynamic cert generation and trust store install | **Fully Functional** | [`crates/jadm-daemon/src/proxy/`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/proxy/) with `rcgen`, `tokio-rustls`, `iptables`/`nftables`. |
| **Clipboard Monitor** | Auto-grab copied URLs | **Fully Functional** | [`crates/jadm-daemon/src/clipboard/monitor.rs`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/clipboard/monitor.rs) polls clipboard every 1s via `arboard`. |
| **Widevine L3 CDM Key Extraction** | Extract encryption keys from browser | **Fully Functional** | [`scripts/cdm_extractor.py`](file:///home/test/Projects/jadman/scripts/cdm_extractor.py) (254 lines) hooks `OEMCrypto_DeriveKeysFromSessionKey` via Frida. |
| **Scheduler Engine** | Time-of-day and rule-based triggering | **Stub / Incomplete** | [`crates/jadm-daemon/src/scheduler/engine.rs`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/scheduler/engine.rs) is an empty 60s sleep loop. |
| **Standalone FFmpeg Hooks** | Post-download thumbnail & remux hooks | **Stub / Commented Out** | [`crates/jadm-daemon/src/ffmpeg/hooks.rs`](file:///home/test/Projects/jadman/crates/jadm-daemon/src/ffmpeg/hooks.rs) code is commented out (yt-dlp handles its own remuxing). |
| **TUI Media Preview** | Terminal rendering for Images & Videos | **Placeholder** | [`crates/jadm-tui/src/preview/engine.rs`](file:///home/test/Projects/jadman/crates/jadm-tui/src/preview/engine.rs) displays static text message; only text preview works. |
| **FUSE / VFS Mount** | Virtual filesystem download presentation | **Not Implemented** | Described only as future roadmap in `ARCHITECTURE.md`. |

---

## 2. In-Depth Component Analysis

### A. Download Engines & Orchestration
1. **`aria2c`**: Auto-detects whether an RPC daemon is running at `127.0.0.1:6800`. If not, spawns `aria2c --enable-rpc --rpc-listen-all=true --rpc-allow-origin-all=true` in the background. Tracks download progress via standard JSON-RPC queries.
2. **`yt-dlp`**:
   - Parses stdout lines with regex matching `[download]  XX.X% of ~?XX.XXMiB at XX.XXMiB/s ETA XX:XX`.
   - Supports `--live-from-start`, `--write-subs`, `--embed-subs`, `--embed-thumbnail`, `--embed-chapters`, `--write-description`, `--user-agent`, and `--add-header Referer:<ref>`.
   - `get_formats()` executes `yt-dlp -J <url>` to extract available resolutions and codecs into structured objects.
3. **`Camoufox`**: Spawns `python3 scripts/download_camoufox.py --url <url> --folder <folder> --user-agent <ua> --cookies-file <cf>`. If `yt-dlp` fails under "ghost mode", the queue manager automatically fails over to Camoufox.
4. **Browser In-Memory Siphoning**: The browser extension intercepts media chunks in JavaScript, serializes them, and sends them across the native messaging bridge using `Request::SiphonChunk`. The daemon reassembles and flushes them to disk.

### B. Network Ingress & Integrations
1. **Mock qBittorrent WebAPI**:
   - Responds to `POST /api/v2/auth/login` with `Ok.` and sets `SID=...` cookie.
   - Responds to `GET /api/v2/app/webapiVersion` with `v2.8.19`.
   - Maps `GET /api/v2/torrents/info` to internal downloads for seamless Sonarr/Radarr tracking.
   - Handles `POST /api/v2/torrents/add` (both `urls` and multipart `torrents` file upload).
2. **Media Stream Server**:
   - `GET /stream/:id` locates the active or downloaded file on disk.
   - Serves the file with full HTTP range support using Tower-HTTP.
   - In `jadm-tui`, pressing `v` copies `http://127.0.0.1:6246/stream/<id>` to the system clipboard for immediate playback in mpv or VLC.
3. **SOCKS5 Proxy with Token Auth**:
   - Custom SOCKS5 server listening on port 6247.
   - Requires SOCKS5 username/password authentication where the password is a 32-character hexadecimal UUID token stored with `0600` permissions.
4. **TLS MITM Proxy**:
   - Dynamic root CA certificate and RSA private key generation via `rcgen`.
   - Automatically installs certificate into Linux `/etc/pki/ca-trust` / `/usr/local/share/ca-certificates` and Firefox/Chrome NSS databases (`certutil`).
   - Supports `iptables` and `nftables` redirection rules.

### C. Lua Plugin System
Running via `mlua` in a dedicated background worker:
- **`realdebrid.lua`**: Listens for magnet links or `.torrent` files, requests Real-Debrid API unrestrict link, and queues high-speed direct HTTP downloads.
- **`wayback.lua`**: Intercepts failed downloads and queries the Internet Archive Wayback Machine API (`https://archive.org/wayback/available?url=...`) to retrieve archived snapshots.
- **`telegram.lua`**: Polls a configured Telegram bot token for download commands and sends status updates when downloads finish.
- **`stealth.lua`**: Applies download bandwidth throttling and random jitter to evade detection.
- **`notify.lua`**: Dispatches system desktop notifications on download completion via `notify_rust`.

### D. Frida Widevine L3 Key Extractor
- [`scripts/cdm_extractor.py`](file:///home/test/Projects/jadman/scripts/cdm_extractor.py) is a complete script using Frida.
- Hooks Chromium processes, locates `libwidevinecdm.so`, and hooks `OEMCrypto_DeriveKeysFromSessionKey` to dump AES content keys (`KEY:<key_id_hex>:<key_hex>`) to `/tmp/jadm_cdm_keys.txt`.
- Exposed directly through IPC commands `Request::CdmStart` and `Request::CdmGetKeys`.

### E. What is NOT Implemented (Stubs & Placeholders)
1. **Scheduler Engine**:
   - `crates/jadm-daemon/src/scheduler/engine.rs` contains only:
     ```rust
     pub async fn run(&self) -> Result<()> {
         loop {
             // Placeholder: Check rules and start/stop downloads
             sleep(Duration::from_secs(60)).await;
         }
     }
     ```
   - Time-based scheduling, bandwidth quotas, or mpv playback triggers are not wired up.
2. **Standalone FFmpeg Hooks**:
   - Functions `extract_thumbnail` and `remux_to_mp4` in `crates/jadm-daemon/src/ffmpeg/hooks.rs` are commented out.
3. **TUI Image/Video Previews**:
   - `crates/jadm-tui/src/preview/engine.rs` returns `[Image — preview not available in terminal]` and `[Video — preview not available in terminal]`. Text files (`.txt`, `.md`, `.rs`, `.json`, etc.) have full Syntect syntax highlighting.
