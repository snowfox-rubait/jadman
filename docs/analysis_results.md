# JADMan Implementation Audit Report

This report maps the planning documents and notes from `~/Documents/Haw/JADMan/` against the current codebase status. It categorizes requirements into **Fully Implemented**, **Partially Done**, and **Untouched/New**, outlining what has been accomplished and what remains on the long-term roadmap.

---

## 1. 🟢 Fully Implemented and Running
These features, once listed as incomplete or planned in your historical notes, are now fully written, compiled, and integrated into the active JADMan pipeline:

### 🎛️ Terminal UI (TUI) & Layout
*   **Columns & Metadata**: The middle panel now correctly displays `Name`, `Size`, `Rate`, `ETA`, and `Percent` metrics dynamically.
*   **Keybindings**: Full input handlers are implemented for key navigation (`j/k`, `h/l`, `gg/G`, `Tab`, `q`), queue controls (`p` for pause, `r`/`rr` for resume, `s` for stop, `d`/`dd` for list deletion, `D`/`dD` for disk deletion), and the Cookie Master dialog (`c`).
*   **Categories & Details**: The categories list is dynamic and populated based on download metadata. The detail panel shows a formatted progress bar, files list, and detailed logs.

### ⚙️ Daemon Queue & Core Engines
*   **yt-dlp Integration**: The engine is fully implemented. It handles format selections, live-stream endpoints, and dynamic parameter propagation (e.g., `--write-description`, subtitles, thumbnails, chapters).
*   **Aria2c RPC & Subprocess management**: Fully integrated with the daemon; handlers exist for pausing, resuming, and deleting downloads.
*   **Cookie Master Integration**: Fully implemented. The daemon queries the localized CookieJar DB (`~/.config/CookieJar/jar.db`) on domain matching, runs `cookie-jar use` in-memory using the password forwarded securely from the TUI, and injects Netscape formats for `yt-dlp` and raw `Cookie` headers for `aria2c`.
*   **Async Process Execution**: Synchronous blocking process commands (such as inside engine detection) have been converted to `tokio::process::Command` to prevent thread blocks.

### 🔌 Browser Extension & Siphoning
*   **Cookies Decryption**: Automatically queries and reads Chrome cookies to bypass authentication paywalls.
*   **Siphon Segment Recorder**: Hooks `fetch`/`XHR`, detects segments, overlays recording HUD inside the browser tab, and pipes chunk arrays to the daemon for final media stitching.
*   **Canvas & Debugger Captures**: Both **WebGL GPU capture** and **Chrome Debugging Protocol stream socket mining** are wired as active ghost engines.
*   **Injected Player Icons**: Injected download icons dynamically update their media source parameters upon page navigation (specifically on YouTube/Twitch) to avoid caching stale video links.

### 🔑 DRM Key Extraction
*   **Frida Hooking**: Widevine CDM extraction hook logic works by attaching to Chrome PIDs and dumping raw decryption keys to `/tmp/jadm_cdm_keys.txt`.

---

## 2. 🟡 Partially / Half Done
These items are structurally supported by the daemon or exist in a basic form, but require further polishing or user scripts to be fully complete:

*   **Torrent & \*Arr Stack Integration**:
    *   *Status*: We've created the asynchronous, thread-safe Lua scripting engine in the daemon (`ScriptManager`). The daemon supports resolving magnets/torrents via Lua hooks (`debrid_resolve`) and executing lifecycle triggers (`on_download_complete`).
    *   *Remaining*: The concrete user scripts mimicking qBittorrent's Web UI HTTP API (to allow Sonarr/Radarr to add torrents to JADMan) and translating those commands to debrid HTTP download tasks are left to user-land scripting (`~/.config/jadm/plugins/debrid/`).
*   **Live Stream Auto-Detection**:
    *   *Status*: Default engine heuristics handle `.m3u8` and `.mpd` links automatically.
    *   *Remaining*: The extension popup still includes the manual "Live video support" checkbox as a fallback; fully automated format stream detection is on the roadmap.

---

## 3. 🔴 New / Haven't Touched Yet
These features from the Vision and problems files represent untouched roadmap targets:

*   **Wayback Machine 404 Recovery (Link Graveyard)**:
    *   *Status*: We implemented the Lua thread-safe worker signature `recovery_resolve` to support Internet Archive queries.
    *   *Remaining*: The automatic invocation inside the daemon `QueueManager`'s HTTP error-handling loop (to automatically trigger recovery on `404` or connection errors) has not been wired up yet.
*   **FUSE Mount / Virtual File System**:
    *   *Status*: Proposed in the overall vision document.
    *   *Remaining*: Mounting active/partial downloads as virtual folders to enable playback (e.g. streaming VLC in sequential chunk order) remains untouched.
*   **Smart Bandwidth "Stealth" Mode**:
    *   *Status*: Proposed in vision.
    *   *Remaining*: Dynamic system-level QoS or router communication to throttle downloads only when gaming, video conferencing, or high-latency tasks are active.
*   **Mobile Companion / Telegram Bot**:
    *   *Status*: Proposed remote snapping interfaces.
    *   *Remaining*: Not implemented.
*   **EdgeCourseBD & DeviantArt Scrapers**:
    *   *Status*: Site-specific download issues noted in `problems.md`.
    *   *Remaining*: Can now be solved by adding custom scraper Lua scripts to `~/.config/jadm/plugins/debrid/` rather than hardcoding site extractors in Rust.

---

## 4. 🔕 Redundant / Abandoned Features
These concepts from your notes have been bypassed or replaced by better architectural solutions:

*   **Windows Shell / Context Menu Integration**: Bypassed. The system is designed and optimized specifically for Linux Hyprland/TUI workflows.
*   **SearXNG Search Integration**: Out-of-scope for the core task of download siphoning and queue management.
*   **JADMan UI Pause/Resume Buttons**: Replaced. Removed from the download popup window as they are handled natively by the extension panel and TUI, removing visual clutter.
