# JADMan System Interception Architecture

To achieve OS-level interception on Linux (the "IDM Moat"), we are building two major subsystems. Both bypass the limitations of browser extension APIs.

## 1. Smart Desktop Handler (XDG Integration)
Instead of just intercepting browser downloads, JADM will intercept **everything** you click across your entire desktop environment (Discord, Telegram, Terminal, etc.).

**How it works:**
1. JADM registers itself as the default handler for `http://` and `https://` in `~/.config/mimeapps.list`.
2. When you click a link anywhere, `xdg-open` hands the URL to `jadm-daemon handle-url <URL>`.
3. JADM instantly sends a `HEAD` request to the URL.
4. **The Split:**
   - If the response is `text/html`, JADM transparently passes the URL to your actual default browser (e.g., Firefox or Chrome).
   - If the response is a media file, archive, or binary (`video/*`, `application/zip`, `application/octet-stream`), JADM intercepts it and adds it to the download queue.

## 2. Transparent TLS MITM Proxy (The Network Moat)
This is the holy grail of download interception. It catches downloads initiated *inside* the browser before the browser can even save them to disk.

**How it works:**
1. **CA Generation:** JADM generates a unique local Root Certificate Authority (CA) and registers it in your system trust store and NSS DB (`~/.pki/nssdb`).
2. **Redirection:** We configure `iptables` or `nftables` to redirect all outbound TCP port 80/443 traffic from your user account to a local JADM proxy port (e.g., `8080`), explicitly exempting the JADM daemon itself to prevent infinite loops.
3. **Dynamic TLS:** When the browser tries to connect to `youtube.com`, the JADM proxy intercepts the connection, reads the SNI, dynamically generates a leaf certificate for `youtube.com` signed by our local CA, and completes the TLS handshake.
4. **Interception:** The proxy forwards the HTTP request to the real server. If the server replies with `Content-Disposition: attachment` or a massive binary payload, JADM intercepts the headers, steals the session cookies, sends a fake "Download Intercepted by JADM" HTML page back to the browser, and hands the real download task to the Aria2/yt-dlp backend.

---

**Implementation Order:**
I will begin by building the **Smart Desktop Handler** right now, as it requires zero root privileges and immediately improves quality of life. Then we will move on to the TLS MITM Proxy.
