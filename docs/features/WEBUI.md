# JADMan HTTP & Web Services

## Status: Operational Services & Planned WebUI

Port `6246` on `jadm-daemon` hosts the daemon's HTTP services layer. Rather than serving an HTML dashboard, it currently provides critical backend automation and media streaming endpoints, with a standalone WebUI planned for future release.

---

## 1. ⚙️ Active HTTP Services (Port 6246)

### A. Mock qBittorrent WebAPI (`/api/v2/*`)
Emulates the qBittorrent v2 WebAPI to allow automation services like the **\*Arr Stack** (Sonarr, Radarr, Lidarr, Prowlarr) to use JADMan as an IDM-grade download client:
- `POST /api/v2/auth/login`: Session cookie authentication (`SID=JADMAN_SESSION_MOCK_ID`).
- `GET /api/v2/app/webapiVersion`: Returns API version (`2.0`).
- `GET /api/v2/torrents/info`: Serializes the current active/completed downloads into qBittorrent torrent objects.
- `POST /api/v2/torrents/add`: Accepts magnet links, URLs, and torrent tasks and dispatches them to the queue manager and Debrid resolver.
- `POST /api/v2/torrents/delete`: Deletes active tasks and associated files.

### B. Seekable HTTP Media Streaming (`/stream/:id`)
- Exposes active and finished video/audio downloads via `tower_http::services::ServeFile`.
- Fully supports HTTP byte-range requests (`Range: bytes=...`, `206 Partial Content`), allowing players like **VLC** or **mpv** to seek and stream files while they are still downloading.
- Pressing `v` in the TUI instantly copies this streaming URL to the clipboard.

---

## 2. 🌐 Static Landing Website (`website/`)

- A hand-crafted, dependency-free landing page located in `website/` (`index.html`, `style.css`, `script.js`).
- Designed for zero-cost deployment to Cloudflare Pages.
- Fetches the latest binary assets directly from GitHub Releases API and detects the user's operating system.

---

## 3. 🔮 Planned Features (Standalone WebUI Dashboard)

- [ ] **Embedded WebUI Dashboard:** Serve an optional lightweight, dark-mode web dashboard from `jadm-daemon` on `http://127.0.0.1:6246` with three-panel layout matching the TUI.
- [ ] **Drag-and-Drop Ingress:** Drop download URLs, torrents, or metalink files directly into the browser to enqueue.
- [ ] **Web-based Settings Manager:** Configure categories, connections, and engine parameters from a web interface.
