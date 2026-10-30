# JADMan Optional Dependencies & Ecosystem Integration

JADMan is fully modular and supports a wide range of external tools, desktop utilities, and automation stacks to unlock its complete feature set. 

This document lists all optional dependencies, their purpose in JADMan, and how to install them.

---

## 1. Core Download Engines
These backends are used by the daemon to perform the actual download operations.

### `aria2`
*   **Purpose**: The primary multi-connection engine used to download HTTP/HTTPS files, magnet links (when debrid caching is not used), and generic downloads at maximum bandwidth.
*   **Arch Linux**: `sudo pacman -S aria2`
*   **Debian/Ubuntu**: `sudo apt install aria2`
*   **Fedora**: `sudo dnf install aria2`

### `yt-dlp`
*   **Purpose**: Extracts and downloads media streams (videos/playlists/audio) from YouTube, Twitch, Vimeo, and over 1,000 other websites.
*   **Arch Linux**: `sudo pacman -S yt-dlp` (or via AUR)
*   **Debian/Ubuntu**: `sudo apt install yt-dlp`
*   **Fedora**: `sudo dnf install yt-dlp`

### `ffmpeg`
*   **Purpose**: Video/audio post-processing and muxing. Required by `yt-dlp` to merge separate high-definition video and audio tracks, embed subtitles, write descriptions, or inject thumbnails/metadata.
*   **Arch Linux**: `sudo pacman -S ffmpeg`
*   **Debian/Ubuntu**: `sudo apt install ffmpeg`
*   **Fedora**: `sudo dnf install ffmpeg`

---

## 2. Desktop & Clipboard Utilities
These tools power desktop integration features like notifications and clipboard copy.

### `libnotify` / `notify-send`
*   **Purpose**: Exposes desktop notifications. Used by the JADMan Lua notifier script (`plugins/hooks/notify.lua`) to throw native popups upon download starts, successes, or failures.
*   **Arch Linux**: `sudo pacman -S libnotify`
*   **Debian/Ubuntu**: `sudo apt install libnotify-bin`
*   **Fedora**: `sudo dnf install libnotify`

### System Clipboard Manager
*   **Purpose**: Allows copying the range-request stream URLs to your system clipboard when pressing `v` on a download in the TUI.
*   **Wayland**: Install `wl-clipboard` (Arch: `sudo pacman -S wl-clipboard`)
*   **X11**: Install `xclip` or `xsel` (Arch: `sudo pacman -S xclip`)

---

## 3. Automation Stack (*Arr Stack)
These services automate movie/show searching, monitoring, and indexer parsing. You can run them locally or via Docker Compose.

### Prowlarr
*   **Purpose**: An indexer proxy that manages torrent and Usenet indexers and pushes them automatically to Sonarr, Radarr, Lidarr, and Readarr.
*   **Port**: `9696`

### Sonarr
*   **Purpose**: Automated download client for TV Shows and anime. Sends downloads to JADMan via the mock qBittorrent HTTP server.
*   **Port**: `8989`

### Radarr
*   **Purpose**: Automated download client for Movies.
*   **Port**: `7878`

### Lidarr
*   **Purpose**: Automated download client for Music.
*   **Port**: `8686`

### Running the *Arr Stack via Docker Compose
To launch the complete automation stack instantly, save the following to `docker-compose.yml` and run `docker compose up -d`:
```yaml
version: "3"
services:
  sonarr:
    image: lscr.io/linuxserver/sonarr:latest
    container_name: sonarr
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=Asia/Dhaka
    volumes:
      - ./config/sonarr:/config
      - ${HOME}/Downloads:/downloads
    ports:
      - 8989:8989
    restart: unless-stopped

  radarr:
    image: lscr.io/linuxserver/radarr:latest
    container_name: radarr
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=Asia/Dhaka
    volumes:
      - ./config/radarr:/config
      - ${HOME}/Downloads:/downloads
    ports:
      - 7878:7878
    restart: unless-stopped

  lidarr:
    image: lscr.io/linuxserver/lidarr:latest
    container_name: lidarr
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=Asia/Dhaka
    volumes:
      - ./config/lidarr:/config
      - ${HOME}/Downloads:/downloads
    ports:
      - 8686:8686
    restart: unless-stopped

  prowlarr:
    image: lscr.io/linuxserver/prowlarr:latest
    container_name: prowlarr
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=Asia/Dhaka
    volumes:
      - ./config/prowlarr:/config
    ports:
      - 9696:9696
    restart: unless-stopped
```

---

## 4. Premium Debrid Services
*   **Purpose**: Caches torrent downloads to premium gigabit servers and generates high-speed, direct HTTP download links. Essential for downloading torrents anonymously at maximum connection limits without seeding or VPNs.
*   **Supported Providers**: Real-Debrid, AllDebrid, Premiumize.
*   **Setup**: Generate an API token from your Debrid account settings and add it to `plugins/debrid/realdebrid.lua` inside JADMan's plugin directory.

---

## 5. Advanced & DRM Decryption Ecosystem
These optional tools power advanced sniffing, browser emulation, and Widevine DRM key extraction features.

### `frida` (Python Library & Tool)
*   **Purpose**: Used by the Frida CDM Key Extractor (`scripts/cdm_extractor.py`) inside the JADMan toolbox to hook into browser or media player processes and extract Widevine DRM decryption keys (Client ID and Private Key).
*   **Installation**:
    *   Install the Python package: `pip install frida frida-tools`
    *   On Windows, ensure you run your command prompt as Administrator when running the extractor.
    *   If targeting Android devices or emulators, ensure the matching `frida-server` binary is running on the target device.

### `camoufox` (Python Library & Stealth Browser)
*   **Purpose**: Used by the Camoufox engine bypass to scrape and navigate websites protected by advanced anti-bot shields (like Cloudflare, Akamai) using a fingerprint-spoofed stealth browser.
*   **Installation**:
    *   Install the Python library: `pip install camoufox`
    *   Download the browser binaries: `python scripts/download_camoufox.py`
