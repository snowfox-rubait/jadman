# JADMan System Interception Architecture

To achieve OS-level interception on Linux (the "IDM Moat"), JADMan implements two major subsystems. Both bypass the limitations of browser extension APIs.

## 1. Smart Desktop Handler (XDG Integration)
Instead of just intercepting browser downloads, JADMan intercepts links clicked across the entire desktop environment (Discord, Telegram, Terminal, chat apps, etc.).

**Implementation (`crates/jadm-daemon/src/desktop.rs`):**
1. JADMan registers `jadman-handler.desktop` as the handler for `http://` and `https://` via `xdg-mime`.
2. When a link is clicked anywhere on the desktop, `xdg-open` forwards the URL to `jadm-daemon handle-url <URL>`.
3. JADMan sends an asynchronous `HEAD` request to inspect the `Content-Type` header:
   - **HTML / Web Pages (`text/html`)**: JADMan transparently passes the URL to the user's real browser (e.g., Firefox, Chrome, Brave) via `gtk-launch` or `xdg-open`.
   - **Media, Archives, & Binaries (`video/*`, `audio/*`, `application/zip`, `application/octet-stream`)**: JADMan intercepts the URL and sends `Request::AddDownload` to the local daemon socket.

## 2. Transparent TLS MITM Proxy (The Network Moat)
Catches downloads initiated directly inside applications before data hits the disk.

**Implementation (`crates/jadm-daemon/src/proxy/`):**
1. **CA Generation (`cert.rs`):** JADMan generates a local Root Certificate Authority (CA) using `rcgen` and installs it into system trust stores and user NSS databases (`~/.pki/nssdb`).
2. **Redirection (`iptables.rs`):** Configures `iptables`/`nftables` rules to route outbound HTTP/HTTPS traffic to the local proxy port, exempting the daemon's own UID to prevent loops.
3. **Dynamic TLS (`mitm.rs`):** Intercepts outbound TLS handshakes, dynamically mints leaf certificates signed by the local CA, and negotiates TLS.
4. **Interception:** Inspects HTTP responses; if `Content-Disposition: attachment` or media streams are detected, cookies and headers are extracted and enqueued in `QueueManager`.

---

**Status:** Both the **Smart Desktop Handler** and the **TLS MITM Proxy** modules are implemented in `crates/jadm-daemon`.
