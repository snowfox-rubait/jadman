# JADMan Advanced Bypass Roadmap

This document outlines theoretical, state-of-the-art bypasses for zero-trust client verification, anti-bot mechanisms, and encrypted media systems.

---

## 1. WebGL Canvas Capture (Bypass for Wasm Rendering)
> **Status**: Prototype implemented in extension (`extension/chrome/siphon_hook.js` & `content.js`).

### Adversary Defense
The server compiles its video decoding logic into a mutated WebAssembly (Wasm) module. The video streams are decoded inside the Wasm linear heap and rendered directly to a `<canvas>` element using WebGL, completely bypassing the HTML5 media player and MSE hooks.

### JADMan Bypass
* **WebGL Frame Interception**: Monkey-patch `WebGLRenderingContext.prototype.texImage2D` and `gl.drawArrays`/`gl.drawElements` in a shadow-DOM execution scope.
* **Extraction**: Intercept the video frame textures right before they are drawn on screen, reading the pixel buffers directly out of GPU memory (`gl.readPixels`) and piping them to the daemon to compile them back into an MP4 file.

---

## 2. EME Key Extraction (Frida CDM Hooking)
> **Status**: Implemented (`scripts/cdm_extractor.py` and `Request::CdmStart`).

### Adversary Defense
Media segments are encrypted using Widevine or PlayReady DRM. The browser passes segments to the operating system's Content Decryption Module (CDM), which decrypts them inside a secure CPU/GPU enclave.

### JADMan Bypass
* **CDM Binary Hooking**: In desktop environments, run a Frida hook script attached to the browser's execution thread.
* **Extraction**: Hook key exchanges inside the CDM library memory space (e.g. `libwidevinecdm.so` or `widevinecdm.dll`) to dump Widevine L3 decryption keys as they load, letting the daemon decrypt the HLS/DASH segments offline.

---

## 3. Active TCP-State Syncing (TCP Fingerprint Bypass)
> **Status**: SOCKS5 proxy active (`crates/jadm-daemon/src/rpc/socks.rs`); kernel socket option tuning planned.

### Adversary Defense
Passive OS fingerprinting engines (like `p0f` or Akamai's network layer) analyze packet RTT, MTU sizes, and TCP Window Scale settings. If a Linux downloader (yt-dlp) sends a request claiming to be Windows Chrome, the server detects the mismatch and blocks it.

### JADMan Bypass
* **Socks5 TCP Tuner**: Run a local SOCKS5 proxy in the daemon that handles outgoing downloader connections.
* **State Modification**: Dynamically modify the Linux kernel's network socket options (e.g. `IP_TTL`, `TCP_MAXSEG`, and socket window parameters) for that connection on the fly to match the exact profile of the spoofed OS/browser.

---

## 4. ES6 Proxy API Mimicry (DOM Introspection Bypass)

### Adversary Defense
Hostile scripts inspect the webpage's DOM prototypes to see if core APIs (like `fetch`, `Object`, `WebSocket`) have been modified or wrapped by extension scripts, blacklisting the browser if any alterations are detected.

### JADMan Bypass
* **Shadow ES6 Proxies**: Instead of standard function wrapping, use ES6 `Proxy` wrappers.
* **Native Code Emulation**: Override `.toString()` on the proxy to return the exact string signature of native code (e.g., `function fetch() { [native code] }`) and ensure getters return standard prototype chains.

---

## 5. Debugger Network Capture (No-Connection Bypass)
> **Status**: Prototype implemented in browser extension (`extension/chrome/`).

### Adversary Defense
The server pins authorization credentials to the exact TCP/TLS session established by the browser tab. Any secondary connection attempt (even with identical cookies and TLS fingerprints) is immediately rejected.

### JADMan Bypass
* **Debugger Hooking**: Attach Chrome's `chrome.debugger` API to the page's network agent.
* **Extraction**: Instead of making new connections, read the raw HTTP response bodies directly from the browser's *active* socket connections as they are received by the tab, writing the streams directly to the daemon.

---

## 6. Stealth DOM Overlay Host (In-Viewport Visual Masking)
> **Status**: Implemented via closed shadow DOM and prototype caching in `content.js`.

### Adversary Defense
Hostile scripts monitor the DOM structure (via `MutationObserver` or `document.querySelectorAll('*')`) to detect UI additions injected by video grabbers or download manager extensions. If they find any unrecognized nodes, class names, or IDs, they shut down playback or restrict access.

### JADMan Bypass
* **Closed Shadow DOM Rendering**: All in-viewport elements injected by JADMan (such as download selection buttons, media indicators, or toast overlays) are nested within a closed shadow root. To the page's scripts, the shadow root is invisible and query methods like `element.shadowRoot` return `null`.
* **Prototype Caching**: To prevent detection via page-level monkey-patching of browser APIs, JADMan caches native constructors (such as `Element.prototype.attachShadow`, `Node.prototype.appendChild`, and `document.createElement`) at `document_start` before any page-level scripts load, using only the clean, original native methods to inject the UI.
* **Event Retargeting Evasion**: User clicks inside the closed shadow DOM are captured and stopped at the boundary so that global page-level event handlers do not capture click patterns.

