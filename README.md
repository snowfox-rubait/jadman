# JADMan — Just Another Download Manager

> The download manager Linux deserves. A full IDM replacement — and then some.

JADMan is a keyboard-driven, terminal-native download manager and stream interceptor, built in Rust.
It handles everything: segmented file downloads, video/audio from thousands of sites,
scheduled triggers, browser integration, and a fast TUI — all orchestrated by a persistent
background daemon.

No Electron. No GUI framework. No mouse required.

---

## Why JADMan?

Internet Download Manager (IDM) is the gold standard on Windows — and Linux has never had
a real answer to it. JADMan is that answer, and it goes further:

| Feature | IDM | JADMan |
|---|:---:|:---:|
| Segmented / multi-connection downloads | ✅ | ✅ |
| Browser integration (click & stream capture) | ✅ | ✅ |
| Schedule downloads | ✅ | 🟡 (In progress) |
| Video site downloads (YouTube, Twitch, etc.) | ❌ | ✅ |
| Media-aware triggers (start based on conditions) | ❌ | 🟡 (In progress) |
| Terminal-native TUI | ❌ | ✅ |
| Inline code/text syntax-highlighted preview | ❌ | ✅ |
| Seekable HTTP media streaming (VLC/mpv) | ❌ | ✅ |
| Arr automation suite (mock qBittorrent API) | ❌ | ✅ |
| Extensible Lua plugin system | ❌ | ✅ |
| Clipboard monitor | ✅ | ✅ |
| Batch grabber with smart link extraction | ✅ | ✅ |
| Fetch & WebGL interception (catches streams & blobs) | ❌ | ✅ |
| Native SOCKS5 proxy & TLS MITM engine | ❌ | ✅ |
| CookieJar password-protected cookie store | ❌ | ✅ |
| Fully open source, forever | ❌ | ✅ |

---

## Features

### Core Engine & Routing
- **Segmented downloads** via `aria2c` — split files into parallel connections (up to 16)
- **Video & audio downloads** via `yt-dlp` — supports YouTube, Twitch, Twitter/X, and thousands of supported extractors
- **Smart URL routing & auto-categorization** — automatically classifies URLs (Documents, Archives, Videos, Audio, Programs, Images) and assigns engines
- **In-progress media streaming** — `GET /stream/:id` serves active downloads with byte-range headers, letting you watch videos in VLC/mpv while downloading
- **Mock qBittorrent WebAPI** — listens on port 6246 emulating qBittorrent v2 (`/api/v2/*`) for out-of-the-box integration with Sonarr, Radarr, and Prowlarr
- **CookieJar Integration** — queries `~/.config/CookieJar/jar.db` and pipes decrypted Netscape cookies straight to `yt-dlp` via stdin

### Browser Extension & Interception
A multi-layered capture system connecting via **Native Messaging** (`com.jadm.jadm`):
- Intercepts browser download dialogs and reroutes them to JADMan
- Injects floating download badges onto HTML5 `<video>` and `<audio>` players across shadow DOM boundaries
- Siphons raw memory chunks from monkey-patched `fetch`/`XMLHttpRequest` and WebGL canvas contexts
- Sniffs network responses by MIME type and file size
- Batch grabber UI and right-click context menu toolbox for mass link selection
- OS-level URL interception via `jadman-handler.desktop`

### Lua Plugin Ecosystem (`plugins/`)
- **Real-Debrid resolver** (`realdebrid.lua`): Automatically unrestrict torrents and magnets into high-speed direct HTTP streams
- **Wayback Machine 404 recovery** (`wayback.lua`): Automatically queries the Internet Archive when downloads return HTTP 404
- **Telegram bot** (`telegram.lua`): Remote status polling and completion alerts
- **Desktop notifications** (`notify.lua`): Native desktop notifications via `notify-rust`

### TUI (Terminal UI)
Built with `ratatui` and `crossterm`. Three-panel layout: categories, download list, detail/preview.

```
┌─────────────────┬──────────────────────────────┬───────────────────────────┐
│   Categories    │         Download List         │     Detail / Preview      │
│                 │                               │                           │
│ ▶ Unfinished    │  archlinux.iso   2.7G  4.2M/s │  [########c      ]  45%  │
│   Videos        │  ubuntu.iso      1.1G    —    │                           │
│   Audio         │  paper.pdf       1.1M    —    │  Size       2.70 GB      │
│   Documents     │  song.mp3        8.2M  1.1M/s │  Rate       4.2 MB/s     │
│   Programs      │                               │  ETA        6m 32s       │
│                 │                               │  Engine     aria2c        │
│ ▶ Finished      │                               │                           │
│   ...           │                               │  [preview area]           │
└─────────────────┴──────────────────────────────┴───────────────────────────┘
```

- **Pacman-style progress bar** — `[########c    ]` with a moving mouth character
- **Inline syntax preview** — syntax-highlighted code/text inspection via `syntect`
- **Vim-style navigation** — fast keyboard navigation, single-key download controls
- **Stream URL copy** — press `v` on any download to copy its seekable streaming URL to the clipboard

### Security & Advanced Tools
- **Authenticated SOCKS5 Proxy** — local proxy on port 6247 secured with token authentication
- **TLS MITM Proxy** — dynamic root CA generation and packet redirection for encrypted stream sniffing
- **Widevine CDM Key Extractor** — Frida script (`scripts/cdm_extractor.py`) hooking `libwidevinecdm.so` to dump AES keys
- **Camoufox Stealth Browser** — anti-bot automation script (`scripts/download_camoufox.py`) for Cloudflare-protected sites

---

## Architecture

JADMan runs as a persistent background daemon (`jadm-daemon`) that client interfaces connect to.

```
Browser Extension  ──Native Messaging (stdio)──▶ jadm-daemon (jadm.sock)
TUI (jadm-tui)     ──Unix Domain Socket───────▶ jadm-daemon (jadm.sock)
Sonarr / Radarr    ──Mock qBittorrent API :6246─▶ jadm-daemon
VLC / mpv Players  ──HTTP Stream /stream/:id────▶ jadm-daemon
                                                    │
                                           aria2c / yt-dlp / SQLite WAL
                                           CookieJar / Lua Plugins
                                           SOCKS5 & TLS MITM Proxy
```

---

## Installation & Setup

JADMan is fully cross-platform and runs natively on **Linux**, **Windows**, and **macOS**. 

### 1. Install Core Dependencies
JADMan orchestrates high-speed downloads and stream stitching using `aria2`, `yt-dlp`, and `ffmpeg`. Install them for your platform:

*   **Windows**: Open PowerShell and run:
    ```powershell
    winget install aria2.aria2 yt-dlp.yt-dlp Gyan.FFmpeg
    ```
*   **macOS**: Open Terminal and run:
    ```bash
    brew install aria2 yt-dlp ffmpeg
    ```
*   **Linux**: Install via your package manager:
    ```bash
    sudo pacman -S aria2 yt-dlp ffmpeg   # Arch Linux
    sudo apt install aria2 yt-dlp ffmpeg # Debian/Ubuntu
    ```

### 2. Get JADMan
You can build JADMan from source (requires the Rust toolchain):
```bash
# Clone from GitHub (or Codeberg mirror: https://codeberg.org/snowfox-rubait-96/jadman.git)
git clone https://github.com/snowfox-rubait/jadman.git
cd jadman
cargo build --release
```
*   The compiled daemon will be at `target/release/jadm-daemon` (or `jadm-daemon.exe`).
*   The TUI client will be at `target/release/jadm-tui` (or `jadm-tui.exe`).

### 3. Register Browser Native Messaging Host
To allow browser extensions to send downloads to JADMan, register the manifest:
*   **Windows**: Open CMD/PowerShell in the folder containing `jadm-daemon.exe` and run:
    ```cmd
    jadm-daemon.exe install-native-manifest
    ```
*   **Linux / macOS**: Open terminal in the build output folder and run:
    ```bash
    ./jadm-daemon install-native-manifest
    ```

### 4. Install Browser Extension
1.  Open your browser (Chrome/Brave/Edge) and go to `chrome://extensions`.
2.  Enable **Developer mode** (toggle switch in the top-right corner).
3.  Click **Load unpacked** (top-left) and select the `extension/chrome/` folder inside the repository.
*(For Firefox, go to `about:debugging#/runtime/this-firefox`, click **Load Temporary Add-on**, and select any file inside `extension/firefox/`)*

---

## Usage

1.  **Start the background daemon**:
    *   **Linux / macOS**: `jadm-daemon &` (or run in background/systemd).
    *   **Windows**: Launch `jadm-daemon.exe` (runs in background shell).
2.  **Open TUI Console**:
    *   Run `jadm-tui` (or `jadm-tui.exe` on Windows) to view the queue and control downloads via keyboard shortcuts.

---

## Keybindings (TUI)

### Active Keybindings

| Key | Action |
|---|---|
| `h / l` | Move focus between panels (Categories / Downloads / Detail) |
| `j / k` | Move selection up / down in download list |
| `g / G` | Jump to top / bottom of list |
| `a` | Add download URL (opens input prompt) |
| `p` | Pause selected download |
| `r` | Resume selected download |
| `s` | Stop selected download |
| `d` | Remove download from queue list |
| `D` | Remove download and permanently delete file from disk |
| `v / V` | Copy seekable stream URL (`http://127.0.0.1:6246/stream/:id`) to clipboard for VLC/mpv |
| `c / C` | Open Cookie Master password modal to decrypt stored cookies |
| `?` | Toggle help overlay |
| `q` | Quit TUI |

### Planned Keybindings (In Active Development)
- `A` — Batch add multiple URLs
- `o / O` — Open completed file / containing folder in file manager
- `t` — Interactive schedule / condition trigger editor
- `/` — Live fuzzy search / filter
- `m*` — Column sorting submenu (`mn` by name, `ms` by size, `md` by date, etc.)

---

## Configuration

Config lives at `~/.config/jadm/config.toml`. A default template is generated on first run.

```toml
[general]
default_folder = "~/Downloads"
max_connections = 8
speed_limit = 0          # 0 = unlimited

[folders]
Videos     = "~/Downloads/Videos"
Audio      = "~/Downloads/Audio"
Documents  = "~/Downloads/Documents"
Programs   = "~/Downloads/Programs"

[ytdlp]
format = "bestvideo+bestaudio/best"
embed_subs = true

[ffmpeg]
auto_remux = false
auto_compress = false
```

---

## License

JADMan is licensed under the **GNU General Public License v3.0 (GPL-3.0)**.

See [`LICENSE`](LICENSE) for the full license text.

---

## Roadmap

- [x] Daemon architecture + aria2c integration
- [x] yt-dlp subprocess runner + URL detection
- [x] Download queue + SQLite WAL persistence
- [x] Unix socket & Windows TCP RPC
- [x] Browser extensions (Chrome MV3 & Firefox MV2) via Native Messaging
- [x] Full TUI with syntax-highlighted code/text preview
- [x] Mock qBittorrent WebAPI on port 6246 (Sonarr, Radarr, Prowlarr)
- [x] Seekable HTTP media streaming (`/stream/:id`)
- [x] CookieJar AES decryption integration
- [x] Lua plugin engine (Real-Debrid, Wayback Machine, Telegram alerts, desktop notifications)
- [x] Local SOCKS5 proxy (port 6247) & dynamic TLS MITM proxy
- [x] Frida Widevine L3 decryption key extractor
- [ ] Scheduler engine evaluation loop
- [ ] Terminal image/video inline preview (Kitty/Chafa)
- [ ] Packaged releases (AUR, .deb, Flatpak)
- [ ] mpv IPC media position trigger
- [ ] Remote daemon access (SSH tunnel)

---

*JADMan is not affiliated with IDM or Tonec Inc.*
