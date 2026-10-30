# 📥 JADMan Capability Checklist & Verification Matrix

Use this checklist to verify JADMan's features and benchmark its performance against alternative tools (like IDM, Free Download Manager, etc.).

---

## 🔌 1. Browser Extension Capabilities

The browser extensions (Chrome MV3 & Firefox MV3) act as the frontend interceptor and stealth scanner.

- [x] **Operation Modes**:
  - [x] **General Mode**: Intercepts native browser download events and redirects standard static files directly to the JADMan daemon.
  - [x] **Siphon Mode**: Intercepts video/audio streaming links, HLS (`.m3u8`), DASH (`.mpd`), and segment payloads.
  - [x] **Ghost Mode**: Launches a stealth session. Hides overlays, blocks tracking hooks, mimics human pointer patterns, and uses stealth fetching.
- [x] **Silent Observer Mode (Pause/Resume)**:
  - [x] Toggle Pause stops all hook/interception functionality entirely (website sees no extension footprint) while the script remains active in the background.
- [x] **DOM Injection & Anti-Removal Protection**:
  - [x] Injects a custom overlay download button inside video players (YouTube, Vimeo, etc.).
  - [x] **Closed Shadow DOM Traversing**: Pierces open and closed shadow roots recursively to find media tags.
  - [x] **Overlay Persistence (MutationObserver)**: Auto-restores/unhides the download overlay if the page script deletes or hides it.
- [x] **CORS Bypass tab-origin Fetching**:
  - [x] Uses active tab origin cookies and headers to execute requests, streaming binary chunks to the background script via IPC.
- [x] **Hidden Link Hunter**:
  - [x] Scans the DOM for disabled, low-opacity, or countdown-locked anchor/button download links, listing them directly in the popup.
- [x] **Right-Click Context Menu Toolbox**:
  - [x] Downloads targeted links/media, grabs selected links, or captures all page media/assets.

---

## ⚙️ 2. Daemon & Core Engine Capabilities

The daemon (`jadm-daemon`) handles queue management, SOCKS proxying, subprocess execution, and post-processing.

- [x] **High-Speed Multi-Threaded Downloads**:
  - [x] Launches `aria2c` with multi-threaded chunk retrieval for static files and simple HTTP streams.
- [x] **Dynamic Stream Downloader**:
  - [x] Launches `yt-dlp` for segmented HLS/DASH streaming video/audio downloads.
- [x] **Memory-Only Cookie Decryption (CookieJar Integration)**:
  - [x] Auto-resolves download domains against Cookie Master's SQLite DB (`~/.config/CookieJar/jar.db`).
  - [x] Decrypts keys using master password supplied via RPC or `COOKIE_JAR_PASSWORD` env var.
  - [x] Pipes decrypted Netscape cookies directly into `yt-dlp`'s stdin (never writes plaintext cookies to `/tmp` or disk).
  - [x] Converts Netscape columns to standard `Cookie` headers for `aria2c` downloads.
- [x] **Database Persistence & Migrations**:
  - [x] Uses SQLite WAL mode (`~/.local/share/jadm/jadm.db`) to log queue state, referer headers, playlist flags, formats, and progress.
- [x] **Playlist Optimization**:
  - [x] Automatically detects playlist structures and gives TUI/popup option to download single video vs. whole playlist.
  - [x] Gracefully saves playlist download state as `Done` if canceled after at least one video completes.
- [x] **Lua Plugin Engine (`plugins/`)**:
  - [x] Real-Debrid unrestrict API resolver (`plugins/debrid/realdebrid.lua`).
  - [x] Wayback Machine 404 recovery resolver (`plugins/recovery/wayback.lua`).
  - [x] Telegram bot alerts (`plugins/hooks/telegram.lua`).
  - [x] Desktop notifications (`plugins/hooks/notify.lua`).
- [x] **Mock qBittorrent WebAPI (:6246)**:
  - [x] Direct compatibility with Sonarr, Radarr, and Prowlarr.
- [x] **Seekable Media Streaming (:6246)**:
  - [x] Serves active downloads with HTTP range headers (`GET /stream/:id`) for instant VLC/mpv playback.
- [x] **SOCKS5 Proxy (:6247) & TLS MITM Engine**:
  - [x] Local authenticated proxy and dynamic root CA packet interception.
- [x] **Widevine L3 Key Extractor**:
  - [x] Frida hook script (`scripts/cdm_extractor.py`) dumping keys to `/tmp/jadm_cdm_keys.txt`.
- [x] **Security Referral Injection**:
  - [x] Passes referring site URLs (`referer` header) to bypass hotlink blockades on hostile platforms.

---

## 🖥️ 3. JADMan TUI Client Capabilities

The Terminal User Interface (`jadm-tui`) is the control center for queue monitoring and configuration.

- [x] **Real-time Queue Metrics**:
  - [x] Tracks individual download rates, estimated time of arrival (ETA), size, and progression updates.
- [x] **Secure Modal Input**:
  - [x] Opens **Cookie Master Password** input panel (triggered with `c`/`C`).
  - [x] Masks password text entry (renders as `*`) and caches the password securely inside the daemon's runtime memory.
- [x] **Stream URL Clipboard Copy**:
  - [x] Pressing `v`/`V` copies seekable streaming URL (`http://127.0.0.1:6246/stream/:id`) to clipboard.
- [x] **Syntax Preview Engine**:
  - [x] Real-time syntax-highlighted code and text preview via `syntect`.
- [x] **Category & Filter Control**:
  - [x] Organizes downloads dynamically by category (Documents, Archives, Videos, Audio, Programs, Images) and status tabs.
- [x] **Keyboard Navigation & Help Bar**:
  - [x] Vim-style navigation (`h/l`, `j/k`, `g/G`) and active help overlay (triggered with `?`).

---

## 🔮 4. Capabilities Planned / Under Implementation

- [ ] **Scheduler Evaluation Loop**:
  - [ ] Implement active trigger evaluation loop in `scheduler/engine.rs` to start downloads automatically on time/condition.
- [ ] **Alternative Ghost Browser Fallbacks**:
  - [ ] Script wrapper fallback: if `camoufox` stealth browser fails to import/execute, fallback to `cloakbrowser` or `puppeteer-stealth` dynamically.
- [ ] **Dynamic Behavioral Emulation**:
  - [ ] Auto-mimics human pointer pathways, page scroll sequences, and randomized packet intervals to avoid fingerprinting triggers on hostile host servers.
- [ ] **Interactive Search (`/`) & Multi-Select**:
  - [ ] Live filtering of the download list in TUI and multi-item batch selection.
- [ ] **Inline Terminal Image/Video Previews**:
  - [ ] Kitty graphics protocol with Chafa ASCII fallback in TUI preview panel.
- [ ] **Active Bandwidth Scheduler**:
  - [ ] Schedule speed limits during day/night cycles or specific hours of the day using standard cron configurations.
