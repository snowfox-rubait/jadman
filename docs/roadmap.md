# JADMan Future Feature Roadmap

This document outlines future enhancements and research items discussed during development.

---

## ✅ Implemented (Current Session)

- **Partitioned Cookie Capture** — `chrome.cookies.getAll({ partitionKey: {} })` now captures CHIPS cookies including `cf_clearance`
- **Ghost Mode → extractor-args impersonation** — When mode is `ghost`, daemon passes `--extractor-args "generic:impersonate"` to yt-dlp
- **Real yt-dlp stderr surfacing** — Actual yt-dlp errors now appear in daemon logs AND the DB `error` field (instead of the generic "failed" message)
- **Mode propagation** — `mode` field flows from extension → popup.js → HTTP /add & /formats endpoints → queue manager → runner

---

## 🚧 Next Priority: In-Browser Download Mode ("Nuclear Option")

### Why This Is Needed
Cloudflare Bot Fight Mode v2 (and v3) **binds `cf_clearance` to the TLS fingerprint** of the session that solved the challenge. Even when:
- ✅ The extension correctly captures `cf_clearance` (now fixed)
- ✅ yt-dlp impersonates Chrome TLS (`--impersonate chrome`)
- ✅ Ghost mode enables `--extractor-args generic:impersonate`

...Cloudflare still returns 403 because the JA3/JA4 TLS fingerprint of `yt-dlp`/`curl-cffi` does not exactly match the real Chromium session that obtained the clearance token.

### Solution: Browser-Native Download (The "Capture Mode")
**Concept** (from old notes): If yt-dlp can't mimic the session, *use the actual session* to fetch the file.

**How it works:**
1. Extension intercepts the target URL (e.g. the `.mp4` direct link)
2. Extension uses `fetch()` inside the page context (which inherits the full browser TLS stack + all cookies) to download the file
3. Raw bytes are streamed/sent to the daemon via the existing `/siphon/upload` endpoint
4. The daemon writes the file to disk

**Advantage**: Zero Cloudflare detection, because *you ARE the browser*. No TLS mismatch, no cookie mismatch.

**Implementation Plan:**
1. ✅ Add a "Browser Download" button/option in `popup.js` for Ghost Mode
2. ✅ Implement `SiphonChunk` and `StopSiphon` RPC handlers in daemon Unix server ([unix.rs](file:///home/test/Projects/jadman/crates/jadm-daemon/src/rpc/unix.rs))
3. ✅ Stream chunks sequentially into target directories with auto-truncation on chunk `0` and append mode on subsequent chunks in `QueueManager` ([manager.rs](file:///home/test/Projects/jadman/crates/jadm-daemon/src/queue/manager.rs))
4. ✅ Test suite covering chunk siphoning correctness ([tests.rs](file:///home/test/Projects/jadman/crates/jadm-daemon/src/queue/tests.rs))

### Resiliency & Resume Support (Post-Power Outage)
- **Database Safety**: SQLite WAL mode ensures transactional safety, keeping the download queue state consistent.
- **Interrupted Job Recovery**: On daemon startup, jobs left in `Downloading` state are automatically reset to `Queued` to restart safely.
- **Native Resume**: `aria2c` and `yt-dlp` resume naturally from partial files on disk.
- **Browser-Native Resume Plan**: To support resuming browser-native downloads:
  1. Extension queries the daemon for the existing file size before starting.
  2. If the file exists, the extension initiates `fetch(url)` with a `Range: bytes={existingSize}-` header.
  3. The daemon skips truncation for chunk `0` when a resume request flag is active.

---

## 1. Domain Auto-Mapping (Local Machine Learning)
- **Goal**: Automatically match visited sites to the best JADMan mode (General, Siphon, Ghost, or Browser-Native).
- **Concept**: Lightweight, local ML model (small weights, tiny footprint) running in the browser extension.
- **Training Flow**:
  - Collect local telemetry: Track which mode the user selects for specific domains and whether download succeeds/fails.
  - Train locally over time (on-device learning) to predict optimal mode for new domains.
- **Note**: Whole separate rabbit hole — do not implement until core features are stable.

---

## 2. Decrypted Stream Interception (Ghost Mode)
- **Goal**: Capture content protected by hostile JavaScript players and custom encryption layers.
- **Concept**: Wrap `SourceBuffer.prototype.appendBuffer` at the API level. Since modern browsers require decrypted media segments before playing them back via HTML5 Video tags, grabbing buffers from this point yields raw, decrypted streams.
- **Applicable for**: Udemy, VdoCipher, custom eLearning platforms, college LMS systems.

---

## 3. Extension Footprint Masking (Ghost Mode)
- **Goal**: Avoid detection by scripts looking for anti-downloader extensions.
- **Concept**: Randomize CSS class names, HTML elements, and DOM IDs injected by JADMan on every load. Ensure extension assets (e.g. background hooks) cannot be probed by host pages.

---

## 4. Human-like Event Mimicking (Ghost Mode)
- **Goal**: Satisfy behavioral verification checks (e.g., SABR player heartbeats on YouTube).
- **Concept**: Mimic subtle, periodic mouse movements and player heartbeat pings to convince hostile telemetry servers of real human playback.

---

## 5. yt-dlp Version Update
> [!WARNING]
> The system yt-dlp is version `2026.03.17` which is 90+ days old. This causes yt-dlp itself to warn on every invocation. Update from the package manager or with `yt-dlp -U` to get the latest Cloudflare impersonation improvements.

---

## 6. Anti-DRM Research (Future/Exploratory)
- **Goal**: Handle Widevine L1 protected content (Netflix, Disney+, etc.)
- **Note**: These platforms decrypt in hardware (CPU/GPU chip), not browser RAM. Current siphon/ghost architecture cannot intercept this. Would require low-level OS hooks (outside scope for now).
