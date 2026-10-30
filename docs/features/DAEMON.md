# JADMan Daemon Features

## Status: Core Engine

### Engine & Routing
- [x] **Automatic Routing:** Heuristics detect whether a URL should use `aria2c` (standard files) or `yt-dlp` (media/streams).
- [x] **Category Auto-Sorting:** Automatically classifies downloads into subfolders (`Documents/`, `Archives/`, `Videos/`, `Audio/`, `Programs/`, `Images/`) based on filename extension and MIME type.
- [x] **Persistence:** SQLite database (`jadm.db`) with WAL mode for session-resilient queue management.
- [x] **Format Detection:** RPC `Request::GetFormats` that invokes `yt-dlp -J` to query available video/audio qualities dynamically.
- [x] **In-Browser Siphon Reassembly:** Accepts binary chunks streamed from extension for encrypted streams and assembles them on disk.

### Connectivity
- [x] **Unix Domain Socket / Windows TCP:** High-performance socket RPC communication for `jadm-tui` and native messaging bridge.
- [x] **Native Messaging Bridge:** Stdio-based bridge with 4-byte length prefixing linking browser extensions to the daemon socket.
- [x] **Mock qBittorrent WebAPI (:6246):** Axum server implementing `/api/v2/*` endpoints for seamless integration with Sonarr, Radarr, and Prowlarr.
- [x] **Seekable Media Streaming (:6246):** `GET /stream/:id` serves active downloads with byte-range headers for instant VLC/mpv playback.
- [x] **Local SOCKS5 Proxy (:6247):** High-speed proxy with token authentication (`$XDG_RUNTIME_DIR/jadm_socks_token`) and tuned socket buffers.
- [x] **TLS MITM Engine:** Local Root CA generation (`rcgen`) and packet redirection for encrypted stream interception.

### Automation & Plugins
- [x] **Clipboard Monitor:** Automatically detects URLs copied to the clipboard via `arboard` and queues them if configured.
- [x] **Lua Plugin Engine (`plugins/` via `mlua`):** Event-driven worker pool:
  - Real-Debrid unrestrict API (`plugins/debrid/realdebrid.lua`)
  - Wayback Machine 404 recovery (`plugins/recovery/wayback.lua`)
  - Telegram bot alerts (`plugins/hooks/telegram.lua`)
  - Desktop notifications (`plugins/hooks/notify.lua`)
- [x] **CookieJar Integration:** Decrypts Netscape cookies from `~/.config/CookieJar/jar.db` using password from TUI/environment and pipes directly to `yt-dlp` via stdin.
- [x] **Widevine CDM Key Extractor:** Hooks `libwidevinecdm.so` via Frida (`scripts/cdm_extractor.py`) to extract AES keys.

### Planned Features
- [ ] **Scheduler Evaluation Loop:** Rule evaluation engine loop is in active development (rule triggers and SQLite schema are created).
- [ ] **Bandwidth Limiting:** Global and per-download speed caps.
- [ ] **Virtual File System (VFS):** FUSE mount exposing active downloads as local virtual folders.
