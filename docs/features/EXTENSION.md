# JADMan Extension Features

## Status: Active Development (Chrome MV3 & Firefox MV3)

### Interception & Detection
- [x] **Standard Interception:** Catches common file extensions (`.zip`, `.rar`, `.iso`, `.tar`, `.exe`, etc.) from standard browser downloads.
- [x] **Network Sniffer:** Monitors MIME types in the background to detect hidden media streams (MP4, WebM, M3U8, DASH).
- [x] **YouTube & Twitch Support:** Handles video/audio streams, shorts, and live streams, updating media URLs dynamically across in-page SPA navigation.
- [x] **Shadow DOM Traversing:** Recursively pierces open and closed shadow DOM roots to discover hidden `<video>` and `<audio>` tags.
- [x] **Native Messaging Bridge:** Communicates directly with `jadm-daemon` via stdio using `chrome.runtime.connectNative("com.jadm.jadm")`.
- [x] **Cookie Passing:** Automatically extracts Netscape cookies (including CHIPS/partitioned cookies) and sends them to the daemon for authenticated downloads.
- [x] **Siphon Chunk Streaming:** Injects `siphon_hook.js` to monkey-patch `fetch` and `XMLHttpRequest`, slicing stream buffers into 256KB chunks and streaming them to the daemon.
- [x] **WebGL Canvas Capture:** Records raw video frames from WebGL canvas contexts when video playback is handled via Wasm/Canvas rendering.

### User Interface & Toolbox
- [x] **Floating Media Bar:** Configurable badge overlay injected into video players for one-click downloading.
- [x] **Right-Click Context Menu Toolbox:** Native browser context menu options:
  - *Download this Media/Link*
  - *Download Selected Links*
  - *Grab all Page Media (Videos/Audio/Docs)*
  - *Grab all Website Assets (Images/CSS/JS)*
  - *Button Position Submenu*
- [x] **Quality Selector Popup:** Popup modal fetching available audio/video formats and resolutions dynamically.
- [x] **Batch Grabber:** Dedicated grabber window for filtering, selecting, and bulk-downloading discovered links.

### Planned Features
- [ ] **Advanced Grabber Filters:** Enhanced regex and date-based filtering.
- [ ] **Dynamic Behavioral Mimicry:** Randomized pointer movements and delay intervals for high-adversary bot detection evasion.
