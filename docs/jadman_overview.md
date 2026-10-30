# JADMan (Just Another Download Manager) - Architecture & System Overview

JADMan is a modular, high-performance download management ecosystem written in Rust. It decouples background task execution, protocol interception, and hardware optimization into a standalone daemon (`jadm-daemon`), driven by an interactive terminal interface (`jadm-tui`) and browser companion extensions.

---

## 1. System Architecture

```mermaid
graph TD
    subgraph Browser & External Clients
        EXT[Browser Extension\nChrome / Firefox]
        ARR[Arr Stack\nSonarr / Radarr / Prowlarr]
        MEDIA[Media Players\nVLC / mpv]
        DESK[OS URL Interceptor\njadman-handler.desktop]
    end

    subgraph IPC & Network Ingress
        NATIVE[Native Messaging Host\nstdio 4-byte prefix]
        HTTP_MOCK[Mock qBittorrent API\n:6246 /api/v2/*]
        HTTP_STREAM[Media Stream Server\n:6246 /stream/:id]
        SOCKS[SOCKS5 Proxy\n:6247 Token Auth]
        UNIX_SOCK[Unix / TCP Socket\njadm.sock]
    end

    subgraph Core Daemon (jadm-daemon)
        QM[Queue Manager\nConcurrency & SQLite WAL]
        LUA[Lua Worker Thread Pool\nmlua]
        CLIP[Clipboard Monitor\narboard]
        PROXY[TLS MITM Engine\nrcgen + tokio-rustls]
    end

    subgraph Engines & Subsystems
        ARIA2[aria2c RPC Daemon\nMulti-connection & Torrents]
        YTDLP[yt-dlp Runner\nVideo / Streams / Formats]
        CAMOUFOX[Camoufox Runner\nAnti-bot Stealth Browser]
        SIPHON[In-Browser Chunk Siphoner\nEncrypted Stream Reassembly]
        COOKIE[CookieJar Database\nAES Decryption]
        FRIDA[CDM Key Extractor\nFrida Widevine Hook]
    end

    subgraph User Interface
        TUI[jadm-tui\nRatatui + Crossterm]
    end

    EXT -->|Native stdio| NATIVE
    NATIVE --> UNIX_SOCK
    ARR -->|qBittorrent v2 API| HTTP_MOCK
    MEDIA -->|HTTP Range Requests| HTTP_STREAM
    DESK -->|Unix Socket| UNIX_SOCK
    TUI -->|JSON-RPC| UNIX_SOCK

    HTTP_MOCK --> QM
    HTTP_STREAM --> QM
    UNIX_SOCK --> QM
    CLIP --> QM
    PROXY --> QM

    QM --> ARIA2
    QM --> YTDLP
    QM --> CAMOUFOX
    QM --> SIPHON
    QM --> COOKIE
    QM --> LUA
    QM --> FRIDA
```

---

## 2. Component Directory

| Component | Path | Language / Technology | Description |
| :--- | :--- | :--- | :--- |
| **Common Protocol** | [`crates/jadm-common/`](file:///home/test/Projects/jadman/crates/jadm-common/) | Rust | Shared data models, RPC request/response schemas, engine enums, and download states. |
| **Core Daemon** | [`crates/jadm-daemon/`](file:///home/test/Projects/jadman/crates/jadm-daemon/) | Rust (Tokio, SQLx, Axum, Tower) | Background service handling queue management, IPC, network proxying, clipboard sniffing, and engine orchestration. |
| **Terminal UI** | [`crates/jadm-tui/`](file:///home/test/Projects/jadman/crates/jadm-tui/) | Rust (Ratatui, Crossterm, Syntect) | Interactive terminal dashboard with vim navigation, live stats, queue controls, and syntax-highlighted previews. |
| **Browser Extension** | [`extension/`](file:///home/test/Projects/jadman/extension/) | JavaScript (Chrome MV3, Firefox MV3) | Page media sniffing, floating action badges, chunk siphoning, CORS-bypassing background fetch, and native messaging. |
| **Lua Plugins** | [`plugins/`](file:///home/test/Projects/jadman/plugins/) | Lua (`mlua`) | Event hooks and providers (Real-Debrid resolver, Wayback Machine fallback, Telegram bot alerts, desktop notifications). |
| **Automation Scripts** | [`scripts/`](file:///home/test/Projects/jadman/scripts/) | Python 3 (Frida, Playwright) | Widevine L3 decryption key dumping (`cdm_extractor.py`) and stealth browser downloading (`download_camoufox.py`). |

---

## 3. Communication Protocols

### Local IPC (Unix Socket / TCP)
- **Linux / macOS**: Unix domain socket at `$XDG_RUNTIME_DIR/jadm/jadm.sock` (or `/tmp/jadm.sock`).
- **Windows**: TCP socket listening on `127.0.0.1:6245`.
- **Format**: Newline-delimited JSON messages conforming to [`Request`](file:///home/test/Projects/jadman/crates/jadm-common/src/protocol.rs#L14-L100) and [`Response`](file:///home/test/Projects/jadman/crates/jadm-common/src/protocol.rs#L103-L119).

### HTTP RPC & Media Streaming (Port 6246)
- **qBittorrent Compatibility**: Direct API emulation for `/api/v2/auth/login`, `/api/v2/app/webapiVersion`, `/api/v2/torrents/info`, `/api/v2/torrents/add`, and `/api/v2/torrents/delete`. Integrates out of the box with Prowlarr, Sonarr, and Radarr.
- **Media Streaming**: `GET /stream/:id` serves active or completed downloads using `tower_http::services::ServeFile`, enabling instant seekable playback in VLC or mpv before a download completes.

### SOCKS5 Proxy (Port 6247)
- Authenticated local SOCKS5 proxy requiring a cryptographically secure token stored at `$XDG_RUNTIME_DIR/jadm_socks_token` with file permissions `0600`.
- Applied socket buffer optimizations (`SO_RCVBUF` / `SO_SNDBUF` at 1MB).

### Native Messaging Host
- Standard stdio-based native messaging bridge with 4-byte length prefixing.
- Auto-installed manifests supported for Google Chrome, Chromium, Brave, and Mozilla Firefox.
