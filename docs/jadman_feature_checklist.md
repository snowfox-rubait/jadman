# 📥 JADMan Capability Checklist & Testing Matrix

Use this checklist to verify JADMan's features one by one and benchmark its performance against alternative tools (like IDM, Free Download Manager, etc.).

---

## 🔌 1. Browser Extension Capabilities

The browser extensions (Chrome MV3 & Firefox MV2) act as the frontend interceptor and stealth scanner.

- [ ] **Operation Modes**:
  - [ ] **General Mode**: Intercepts native browser download events and redirects standard static files directly to the JADMan daemon.
  - [ ] **Siphon Mode**: Intercepts video/audio streaming links, HLS (`.m3u8`), DASH (`.mpd`), and segment payloads.
  - [ ] **Ghost Mode**: Launches a stealth session. Hides overlays, blocks tracking hooks, mimics human pointer patterns, and uses stealth fetching.
- [ ] **Silent Observer Mode (Pause/Resume)**:
  - [ ] Toggle Pause stops all hook/interception functionality entirely (website sees no extension footprint) while the script remains active in the background.
- [ ] **DOM Injection & Anti-Removal Protection**:
  - [ ] Injects a custom overlay download button inside video players (YouTube, Vimeo, etc.).
  - [ ] **Closed Shadow DOM Traversing**: Pierces open and closed shadow roots recursively to find media tags.
  - [ ] **Overlay Persistence (MutationObserver)**: Auto-restores/unhides the download overlay if the page script deletes or hides it.
- [ ] **CORS Bypass tab-origin Fetching**:
  - [ ] Uses active tab origin cookies and headers to execute requests, streaming binary chunks to the background script via IPC.
- [ ] **Hidden Link Hunter**:
  - [ ] Scans the DOM for disabled, low-opacity, or countdown-locked anchor/button download links, listing them directly in the popup.

---

## ⚙️ 2. Daemon & Core Engine Capabilities

The daemon (`jadm-daemon`) handles queue management, SOCKS proxying, subprocess execution, and post-processing.

- [ ] **High-Speed Multi-Threaded Downloads**:
  - [ ] Launches `aria2c` with multi-threaded chunk retrieval for static files and simple HTTP streams.
- [ ] **Dynamic Stream Downloader**:
  - [ ] Launches `yt-dlp` for segmented HLS/DASH streaming video/audio downloads.
- [ ] **Memory-Only Cookie Decryption (Cookie Master Integration)**:
  - [ ] Auto-resolves download domains against Cookie Master's SQLite DB (`~/.config/CookieJar/jar.db`).
  - [ ] Decrypts keys using master password supplied via RPC or `COOKIE_JAR_PASSWORD` env var.
  - [ ] Pipes decrypted Netscape cookies directly into `yt-dlp`'s stdin (never writes plaintext cookies to `/tmp` or disk).
  - [ ] Converts Netscape columns to standard `Cookie` headers for `aria2c` downloads.
- [ ] **Database Persistence & Migrations**:
  - [ ] Uses `sqlx` and SQLite (`~/.local/share/jadm/jadm.db`) to log queue state, referer headers, playlist flags, formats, and progress.
- [ ] **Playlist Optimization**:
  - [ ] Automatically detects playlist structures and gives TUI/popup option to download single video vs. whole playlist.
  - [ ] Gracefully saves playlist download state as `Done` if canceled after at least one video completes.
- [ ] **Post-Processing (FFmpeg)**:
  - [ ] Auto-merges video streams with subtitles, chapters, and thumbnails.
  - [ ] Optional video compression using: `ffmpeg -i input -c:v libx264 -crf 18 -preset slow -pix_fmt yuv420p -c:a copy output.mp4`.
- [ ] **Security Referral Injection**:
  - [ ] Passes referring site URLs (`referer` header) to bypass hotlink blockades on hostile platforms (Vimeo embeds, X, Reddit).

---

## 🖥️ 3. JADMan TUI Client Capabilities

The Terminal User Interface (`jadm-tui`) is the control center for queue monitoring and configuration.

- [ ] **Real-time Queue Metrics**:
  - [ ] Tracks individual download rates, estimated time of arrival (ETA), size, and progression updates.
- [ ] **Secure Modal Input**:
  - [ ] Opens **Cookie Master Password** input panel (triggered with `c`/`C`).
  - [ ] Masks password text entry (renders as `*`) and caches the password securely inside the daemon's runtime memory.
- [ ] **Filter & Sorting Control**:
  - [ ] Filters downloads by status (Downloading, Paused, Done, Cancelled).
  - [ ] Sorts list by Rate, Size, Status, Date Added, or Time Left.
- [ ] **Keyboard Navigation & Help Bar**:
  - [ ] Header/footer navigation panels, active help panel overlays (triggered with `?`).

---

## 🔮 4. Capabilities Planned / Under Implementation

- [ ] **Alternative Ghost Browser Fallbacks**:
  - [ ] Script wrapper fallback: if `camoufox` stealth browser fails to import/execute, fallback to `cloakbrowser` or `puppeteer-stealth` dynamically.
- [ ] **Dynamic Behavioral Emulation**:
  - [ ] Auto-mimics human pointer pathways, page scroll sequences, and randomized packet intervals to avoid fingerprinting triggers on hostile host servers.

---

## 💡 5. Suggested Capabilities for Future Extension

- [ ] **Automated Script Hooks Runner (e.g. Lua/JS scripting engine)**:
  - [ ] Let users write lightweight trigger scripts. E.g., *"If current playing timestamp of player is > 90%, send RPC to prefetch the next file in the list"* (great for binge-watching on low-disk spaces).
- [ ] **Webhook integration (n8n friendly)**:
  - [ ] Dispatch HTTP POST requests to an n8n webhook URL on events like `DownloadStarted`, `DownloadCompleted`, or `DownloadFailed` with JSON payloads.
- [ ] **Active Bandwidth Scheduler**:
  - [ ] Schedule speed limits during day/night cycles or specific hours of the day using standard cron configurations.
