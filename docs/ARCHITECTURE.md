# JADM — Just Another Download Manager
## Architecture Document

---

## 1. Overview

JADM is a keyboard-centric, terminal-native download manager built in Rust.
It orchestrates `aria2c`, `yt-dlp`, and `ffmpeg` through a persistent background
daemon, exposed to a ratatui TUI and a browser extension via a unified RPC interface.

**Design philosophy:**
- Daemon is the brain. TUI and extension are just clients.
- No GUI. No mouse required. No top/bottom bars.
- Everything is a keybind.
- Preview everything — images, video thumbnails, text, code — via Kitty protocol or chafa fallback.

---

## 2. Technology Stack

| Layer | Tool |
|---|---|
| Language | Rust (all components except browser extension) |
| TUI | `ratatui` |
| Image preview | `yazi-adaptor` crate or custom Kitty/chafa impl |
| Download engine | `aria2c` via JSON-RPC |
| Media downloader | `yt-dlp` subprocess |
| Post-processor | `ffmpeg` subprocess |
| Database | SQLite via `rusqlite` |
| TUI ↔ Daemon IPC | Unix socket (fast, local, secure) |
| Extension ↔ Daemon | HTTP/JSON (localhost only) |
| Config | TOML |
| Notifications | `notify-send` via `libnotify` |
| Browser extension | JavaScript (Manifest V3 Chrome / MV2 Firefox) |

---

## 3. Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Browser Extension                        │
│  (intercepts clicks, siphons streams, native messaging)     │
└──────────────────────────┬──────────────────────────────────┘
                           │ Native Messaging (stdio bridge)
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                   jadm-daemon (daemon)                      │
│                                                             │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │  aria2c RPC │  │  yt-dlp sub  │  │    Lua Plugins    │  │
│  │   client    │  │   process    │  │(Debrid, Wayback)  │  │
│  └──────┬──────┘  └──────┬───────┘  └─────────┬─────────┘  │
│         │                │                     │            │
│  ┌──────▼────────────────▼─────────────────────▼─────────┐  │
│  │               Download Queue Manager                  │  │
│  │     (routing, state, priority, retries, categories)   │  │
│  └──────────────────────────┬────────────────────────────┘  │
│                             │                               │
│  ┌──────────────────────────▼────────────────────────────┐  │
│  │                SQLite WAL Database                    │  │
│  │          (downloads, scheduler, cookiejar)            │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │  Scheduler   │  │  Clipboard   │  │  qBittorrent API  │  │
│  │  (in progr)  │  │   Monitor    │  │  & Media Stream   │  │
│  └──────────────┘  └──────────────┘  └───────────────────┘  │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │     Unix Socket RPC Server ($XDG_RUNTIME_DIR/jadm)   │   │
│  └──────────────────────────┬───────────────────────────┘   │
└─────────────────────────────┼───────────────────────────────┘
                              │ Unix socket
┌─────────────────────────────▼───────────────────────────────┐
│                       jadm-tui (TUI)                        │
│                                                             │
│  ┌────────────────────────────────────────────────────────┐  │
│  │                    ratatui renderer                    │  │
│  │                                                        │  │
│  │  ┌─────────────┬─────────────────┬──────────────────┐  │  │
│  │  │  Categories │  Download List  │  Detail/Preview  │  │  │
│  │  │   Panel     │     Panel       │     Panel        │  │  │
│  │  └─────────────┴─────────────────┴──────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌──────────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ Keybind          │  │   Preview    │  │  Daemon RPC  │   │
│  │ Dispatcher       │  │ (Syntect)    │  │  Client      │   │
│  └──────────────────┘  └──────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. Repository Structure

```
jadm/
├── Cargo.toml                  # workspace
├── crates/
│   ├── jadm-daemon/            # jadmd binary
│   │   └── src/
│   │       ├── main.rs
│   │       ├── aria2/          # aria2c JSON-RPC client
│   │       │   ├── client.rs   # RPC calls (addUri, tellStatus, etc.)
│   │       │   └── types.rs    # aria2 response types
│   │       ├── ytdlp/          # yt-dlp subprocess manager
│   │       │   ├── runner.rs   # spawn, stream output, parse progress
│   │       │   └── detect.rs   # URL type detection (media vs file)
│   │       ├── ffmpeg/         # post-download hooks
│   │       │   └── hooks.rs    # remux, compress, convert
│   │       ├── queue/          # download queue
│   │       │   ├── manager.rs  # add, remove, prioritize, route
│   │       │   └── state.rs    # in-memory queue state
│   │       ├── scheduler/      # scheduler engine
│   │       │   ├── engine.rs   # trigger evaluation loop
│   │       │   └── triggers.rs # trigger types + conditions
│   │       ├── clipboard/      # clipboard monitor
│   │       │   └── monitor.rs  # poll clipboard, detect URLs
│   │       ├── rpc/            # IPC servers
│   │       │   ├── unix.rs     # Unix socket server (for TUI)
│   │       │   └── http.rs     # HTTP server (for browser extension)
│   │       ├── db/             # SQLite
│   │       │   ├── schema.rs   # migrations
│   │       │   └── queries.rs  # CRUD
│   │       └── notify/         # notifications
│   │           └── dispatcher.rs
│   │
│   ├── jadm-tui/               # jadm binary (TUI client)
│   │   └── src/
│   │       ├── main.rs
│   │       ├── app.rs          # top-level app state
│   │       ├── ui/
│   │       │   ├── layout.rs   # three-panel layout
│   │       │   ├── categories.rs
│   │       │   ├── list.rs     # download list + columns
│   │       │   └── detail.rs   # right panel
│   │       ├── input/
│   │       │   ├── handler.rs  # raw key events → actions
│   │       │   └── keymap.rs   # keybind definitions
│   │       ├── preview/
│   │       │   ├── engine.rs   # dispatch to right renderer
│   │       │   ├── image.rs    # Kitty protocol + chafa fallback
│   │       │   ├── text.rs     # syntax-highlighted text/code
│   │       │   └── video.rs    # ffmpeg thumbnail extraction
│   │       └── client/
│   │           └── rpc.rs      # Unix socket RPC client
│   │
│   └── jadm-common/            # shared types (no deps on other crates)
│       └── src/
│           ├── types.rs        # Download, Category, Status, etc.
│           └── protocol.rs     # RPC request/response message types
│
├── extension/                  # browser extension
│   ├── manifest.json
│   ├── background.js           # intercept + POST to daemon
│   └── popup.html              # optional settings popup
│
├── config/
│   └── default.toml            # default config template
│
└── docs/
    └── ARCHITECTURE.md         # this file
```

---

## 5. Data Flow

### Normal file download (e.g. .iso, .zip, .pdf)
```
User copies URL or extension intercepts click
→ POST {url, cookies, referer, user_agent} to jadmd HTTP :6246
→ jadmd: ytdlp/detect.rs checks URL → not a media page → route to aria2c
→ queue/manager.rs: adds to SQLite + in-memory queue
→ aria2/client.rs: calls aria2.addUri with cookies + segments config
→ aria2c: downloads in N segments
→ On completion: ffmpeg hook (if configured) → clamav hook → notify-send
→ queue state updated in SQLite
→ TUI polls via Unix socket → updates display
```

### Media URL (e.g. YouTube, Twitter, etc.)
```
Same entry point
→ ytdlp/detect.rs: recognized as media page → route to yt-dlp
→ ytdlp/runner.rs: spawns yt-dlp subprocess, streams stdout
→ Progress parsed from yt-dlp output → stored in queue state
→ On completion: ffmpeg remux/compress hook if configured → notify-send
```

### TUI polling loop
```
jadm TUI: every 500ms → Unix socket → GetQueue {}
→ jadmd returns current state of all downloads
→ ratatui re-renders only changed cells
```

---

## 6. RPC Protocol (jadm-common/protocol.rs)

All messages are JSON over Unix socket (TUI) or HTTP POST (extension).

### Requests
```json
{ "cmd": "AddDownload",    "url": "...", "folder": "...", "priority": 1, "cookies": "..." }
{ "cmd": "PauseDownload",  "id": "abc123" }
{ "cmd": "ResumeDownload", "id": "abc123" }
{ "cmd": "StopDownload",   "id": "abc123" }
{ "cmd": "DeleteDownload", "id": "abc123", "delete_file": false }
{ "cmd": "GetQueue" }
{ "cmd": "GetDownload",    "id": "abc123" }
{ "cmd": "SetSpeedLimit",  "bytes_per_sec": 1048576 }
{ "cmd": "Schedule",       "id": "abc123", "trigger": { ... } }
{ "cmd": "BatchAdd",       "urls": ["...", "..."] }
```

### Response (GetQueue)
```json
{
  "downloads": [
    {
      "id": "abc123",
      "filename": "archlinux.iso",
      "url": "https://...",
      "size": 2890137600,
      "downloaded": 1300561920,
      "percent": 45,
      "rate_bytes": 4404019,
      "eta_secs": 392,
      "status": "downloading",
      "category": "Programs",
      "folder": "/home/user/Downloads/Programs",
      "resumable": true,
      "connections": 8,
      "added_at": 1718100000,
      "completed_at": null,
      "engine": "aria2c"
    }
  ]
}
```

---

## 7. SQLite Schema

```sql
CREATE TABLE downloads (
    id            TEXT PRIMARY KEY,
    url           TEXT NOT NULL,
    filename      TEXT,
    size          INTEGER,
    downloaded    INTEGER DEFAULT 0,
    status        TEXT DEFAULT 'queued',
    -- queued | downloading | paused | done | failed | cancelled
    category      TEXT,
    folder        TEXT,
    resumable     BOOLEAN DEFAULT 0,
    connections   INTEGER DEFAULT 8,
    engine        TEXT DEFAULT 'aria2c',
    -- aria2c | ytdlp
    error         TEXT,
    added_at      INTEGER NOT NULL,
    completed_at  INTEGER,
    last_tried_at INTEGER
);

CREATE TABLE scheduler_rules (
    id            TEXT PRIMARY KEY,
    download_id   TEXT NOT NULL,
    trigger_type  TEXT NOT NULL,
    -- time | on_complete | media_position | storage_below | bandwidth_below
    trigger_data  TEXT NOT NULL,
    -- JSON blob specific to trigger_type
    created_at    INTEGER NOT NULL,
    fired_at      INTEGER,
    FOREIGN KEY (download_id) REFERENCES downloads(id) ON DELETE CASCADE
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- keys: default_folder, speed_limit, max_connections, proxy, theme, etc.
```

---

## 8. Scheduler Trigger Types

```json
{ "type": "time",             "at": "2025-06-12T02:00:00" }
{ "type": "on_complete",      "watch_id": "abc123" }
{ "type": "media_position",   "watch_file": "/path/to/ep1.mp4", "percent": 75 }
{ "type": "storage_below",    "path": "/home/user", "bytes": 5368709120 }
{ "type": "bandwidth_below",  "bytes_per_sec": 524288 }
```

`media_position` trigger requires a running mpv instance with
`--input-ipc-server=/tmp/mpvsocket`. The scheduler engine polls the socket
every 5 seconds for current playback position.

---

## 9. TUI Layout

```
┌─────────────────┬──────────────────────────────┬───────────────────────────┐
│   Categories    │         Download List         │     Detail / Preview      │
│                 │                               │                           │
│ ▶ Unfinished    │  Name            Size    Rate │  archlinux.iso            │
│   Videos        │                  Time    %    │                           │
│   Audio         │  archlinux.iso   2.7G  4.2M/s │  [########c      ]  45%  │
│   Documents     │                  6m32s   45%  │                           │
│   Programs      │  ubuntu.iso      1.1G    —    │  Size       2.70 GB      │
│   Compressed    │                  —       Q    │  Downloaded 1.21 GB      │
│   Other         │  paper.pdf       1.1M    —    │  Rate       4.2 MB/s     │
│                 │                  —      Done  │  ETA        6m 32s       │
│ ▶ Finished      │  song.mp3        8.2M  1.1M/s │  Added      Jun 11 14:32 │
│   Videos        │                  44s    71%   │  Last try   2 min ago    │
│   Audio         │                               │  Resumable  Yes          │
│   Documents     │                               │  Engine     aria2c       │
│   Programs      │                               │  Segments   8 / 8        │
│   Compressed    │                               │                           │
│   Other         │                               │  ──────────────────────  │
│                 │                               │  [preview area]          │
│                 │                               │  image / thumbnail /     │
│                 │                               │  text / code             │
└─────────────────┴──────────────────────────────┴───────────────────────────┘
```

**Progress bar style (pacman-inspired):**
```
[########c         ]  45%
[################c ]  89%
[##################]  100%
```
`c` = pacman mouth character, moves with the fill edge. Disappears at 100%.

No top bar. No bottom bar. No borders on outer edges (inner dividers only).

---

## 10. Keybindings Reference

### Navigation
| Key | Action |
|---|---|
| `h / l` | Move focus between panels |
| `j / k` | Move up / down in list |
| `J / K` | Scroll preview panel up / down |
| `gg` | Jump to top |
| `G` | Jump to bottom |
| `Tab` | Cycle panel focus |
| `/` | Fuzzy search in current panel |
| `?` | Help overlay |
| `q` | Quit |

### Download Control
| Key | Action |
|---|---|
| `s` | Stop selected |
| `S` | Stop all |
| `p` | Pause selected |
| `rr` | Resume selected |
| `ra` | Resume all |
| `R` | Redownload selected (no submenu) |

### Selection
| Key | Action |
|---|---|
| `space` | Toggle select (checkbox, non-contiguous) |
| `v` | Visual select mode (range, contiguous) |

### Delete (`d` submenu)
| Key | Action |
|---|---|
| `dd` | Delete from list only |
| `dD` | Delete from list + source file |
| `da` | Delete all completed from list |
| `Esc` | Cancel |

### Add Downloads
| Key | Action |
|---|---|
| `a` | Add single URL |
| `A` | Batch add (multi-line input) |
| `i` | Import URLs from .txt file |

### Sort (`m` submenu)
| Key | Action |
|---|---|
| `mn` | Sort by name |
| `ms` | Sort by size |
| `md` | Sort by date added |
| `mt` | Sort by time left |
| `mr` | Sort by transfer rate |
| `mS` | Sort by status |
| `m/` | Reverse current sort |

### File Actions
| Key | Action |
|---|---|
| `o` | Open completed file |
| `O` | Open containing folder |
| `c` | Copy URL of selected |
| `e` | Edit download (URL, folder, connections) |
| `I` | Full info overlay |
| `L` | Logs overlay for selected |
| `t` | Set schedule / trigger for selected |
| `C` | Toggle clipboard monitor |
| `:` | Command palette |

---

## 11. URL Routing Logic (ytdlp/detect.rs)

```
Input URL
│
├── Does URL end in known extension? (.zip .iso .exe .pdf .mp4 .mkv etc.)
│   └── YES → aria2c
│
├── Is domain in yt-dlp extractor list?
│   └── YES → yt-dlp
│
├── Try yt-dlp with --simulate (dry run, no download)
│   ├── SUCCESS → yt-dlp
│   └── FAIL    → aria2c
│
└── Final fallback → aria2c
```

---

## 12. Preview Engine (jadm-tui/preview/)

```
Selected download
│
├── Status = downloading / queued → show progress bar + metadata only
│
└── Status = done → detect file type
    ├── Image (.jpg .png .gif .webp .svg)
    │   ├── Terminal supports Kitty protocol? → render inline
    │   └── Fallback → chafa ASCII render
    │
    ├── Video (.mp4 .mkv .webm etc.)
    │   ├── Extract thumbnail via ffmpeg (-ss 00:00:05 -frames:v 1)
    │   └── Render thumbnail same as image
    │
    ├── Audio (.mp3 .flac .ogg)
    │   └── Show embedded album art if present, else waveform text art
    │
    ├── Text / Code (.txt .md .rs .py .js .sh etc.)
    │   └── Render with syntax highlighting (bat or syntect)
    │
    ├── PDF
    │   └── Extract first page as image via poppler → render as image
    │
    └── Binary / unknown → show hex dump preview (first 256 bytes)
```

---

## 13. Config File (config/default.toml)

```toml
[general]
default_folder = "~/Downloads"
clipboard_monitor = false
max_connections = 8
speed_limit = 0          # 0 = unlimited, bytes/sec

[folders]
Videos     = "~/Downloads/Videos"
Audio      = "~/Downloads/Audio"
Documents  = "~/Downloads/Documents"
Programs   = "~/Downloads/Programs"
Compressed = "~/Downloads/Compressed"
Other      = "~/Downloads/Other"

[aria2]
rpc_port    = 6800
rpc_secret  = ""         # set this
split       = 8
min_split_size = "1M"

[ytdlp]
format      = "bestvideo+bestaudio/best"
embed_subs  = true
output_template = "%(title)s.%(ext)s"

[ffmpeg]
auto_remux  = false      # remux to mp4 after download
auto_compress = false    # re-encode to h265 after download
crf         = 28

[notifications]
enabled     = true
on_complete = true
on_fail     = true

[antivirus]
enabled     = false
command     = "clamscan --remove=no"

[proxy]
enabled     = false
url         = ""         # socks5://127.0.0.1:9050 for Tor

[daemon]
unix_socket = "/run/user/1000/jadm.sock"
http_port   = 6246       # for browser extension
```

---

## 14. Build Order

1. `jadm-common` — types + protocol (no external deps, builds first)
2. `jadm-daemon` — depends on jadm-common
3. `jadm-tui` — depends on jadm-common
4. `extension/` — independent, JS only

### Phase 1 (MVP)
- [x] jadm-common types
- [x] aria2c RPC client
- [x] yt-dlp subprocess runner + URL detector
- [x] Download queue manager
- [x] SQLite schema + basic queries
- [x] Unix socket RPC server
- [x] Basic three-panel TUI
- [x] Core keybinds: add, pause, resume, stop, delete

### Phase 2
- [ ] standalone ffmpeg hooks (handled internally by yt-dlp)
- [x] Preview engine (syntax-highlighted code/text via syntect)
- [ ] Scheduler engine evaluation loop (triggers stored in SQLite)
- [x] Clipboard monitor
- [x] Notifications
- [x] Sorting + filtering (dynamic categories & status tabs)

### Phase 3
- [x] Browser extension (Chrome MV3 & Firefox MV2 with Native Messaging)
- [x] Proxy / SOCKS5 support (port 6247) & TLS MITM engine
- [x] Mock qBittorrent WebAPI on port 6246 (Sonarr, Radarr, Prowlarr)
- [x] Seekable HTTP media streaming (`/stream/:id`)
- [x] CookieJar AES decryption integration
- [x] Lua plugin engine (`plugins/` via `mlua`)
- [x] Frida Widevine L3 decryption key extractor
- [ ] Antivirus hook (clamscan)
- [ ] Omarchy theme sync
- [ ] mpv IPC listener for media_position trigger
- [ ] Terminal image/video inline preview (Kitty/chafa)

---

## 15. External Dependencies (Rust crates)

```toml
[workspace.dependencies]
tokio        = { version = "1", features = ["full"] }  # async runtime
serde        = { version = "1", features = ["derive"] }
serde_json   = "1"
rusqlite     = { version = "0.31", features = ["bundled"] }
ratatui      = "0.26"
reqwest      = { version = "0.12", features = ["json"] }  # aria2 RPC
axum         = "0.7"  # HTTP server for extension endpoint
tokio-stream = "0.1"
notify-rust  = "4"    # notifications
which        = "6"    # detect if aria2c/yt-dlp/ffmpeg are installed
arboard      = "3"    # clipboard monitor
syntect      = "5"    # syntax highlighting in preview
```
