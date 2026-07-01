// Preserve native APIs immediately before page scripts can tamper with them
const nativeAttachShadow = Element.prototype.attachShadow;
const nativeFetch = window.fetch;
const nativeAddEventListener = window.addEventListener;
const nativeCreateElement = document.createElement;
const nativeAppendChild = Node.prototype.appendChild;
const nativeRemove = Element.prototype.remove || function() { if (this.parentNode) this.parentNode.removeChild(this); };
const nativeCaptureStream = HTMLCanvasElement.prototype.captureStream || HTMLCanvasElement.prototype.mozCaptureStream;

// Dynamic private communication token for main-world/isolated-world stealth bridge
const JADMAN_COMM_TOKEN = 'jadman_' + Math.floor(Math.random() * 10000000).toString(36);
if (document.documentElement) {
    document.documentElement.setAttribute('data-jadman-token', JADMAN_COMM_TOKEN);
}

// Randomized DOM classes/IDs for footprint masking
const RANDOM_PREFIX = 'jadm_' + Math.floor(Math.random() * 1000000).toString(36);
const FLOATING_MEDIA_BTN_ID = RANDOM_PREFIX + '_float_btn';
const SELECTION_BTN_ID = RANDOM_PREFIX + '_sel_btn';
const ICON_CLASS = RANDOM_PREFIX + '_icon';

// Array to track active wrapper components for media icons
const activeMediaWrappers = [];

let isPaused = false;
let stealthShadowRoot = null;
function getStealthShadowRoot() {
    if (stealthShadowRoot) return stealthShadowRoot;
    
    // Create a randomized host element
    const hostTagName = 'x-' + RANDOM_PREFIX + '-host';
    const host = nativeCreateElement.call(document, hostTagName);
    host.style.cssText = 'position: static; display: block; width: 0; height: 0; border: none; padding: 0; margin: 0;';
    
    // Attach closed shadow root using native cached prototype
    stealthShadowRoot = nativeAttachShadow.call(host, { mode: 'closed' });
    
    // Append the host to the document body or documentElement (to make sure it's there even if body doesn't exist yet)
    if (document.body) {
        nativeAppendChild.call(document.body, host);
    } else {
        const observer = new MutationObserver(() => {
            if (document.body) {
                nativeAppendChild.call(document.body, host);
                observer.disconnect();
            }
        });
        observer.observe(document.documentElement, { childList: true, subtree: true });
    }
    
    return stealthShadowRoot;
}

const IDM_EXTENSIONS = [
    "3gp", "7z", "aac", "ace", "aif", "apk", "arj", "asf", "avi", "bin", "bz2", "exe", "gz", "gzip", 
    "img", "iso", "lzh", "m4a", "m4v", "mkv", "mov", "mp3", "mp4", "mpa", "mpe", "mpeg", "mpg", 
    "msi", "msu", "ogg", "ogv", "pdf", "plj", "pps", "ppt", "qt", "ra", "rar", "rm", "rmvb", 
    "sea", "sit", "sitx", "tar", "tif", "tiff", "wav", "wma", "wmv", "z", "zip",
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "ico", "doc", "docx", "xls", "xlsx", "txt", "csv", "rtf"
];

let floatingSelectionButton = null;
let discoveryToast = null;
let globalFloatingMediaButton = null;
let currentIconPosition = "top-right";

let activeWebGLRecorder = null;
let activeWebGLStream = null;
let activeWebGLOverlay = null;
let isWebGLRecordingStopped = false;

function stopWebGLRecording() {
    isWebGLRecordingStopped = true;
    if (activeWebGLRecorder && activeWebGLRecorder.state !== "inactive") {
        try {
            activeWebGLRecorder.stop();
        } catch(e) {}
    }
    if (activeWebGLStream) {
        try {
            activeWebGLStream.getTracks().forEach(track => track.stop());
        } catch(e) {}
        activeWebGLStream = null;
    }
    if (activeWebGLOverlay) {
        try {
            activeWebGLOverlay.remove();
        } catch(e) {}
        activeWebGLOverlay = null;
    }
}

let activeSegmentRecorderId = null;
let activeSegmentOverlay = null;
let isSegmentRecordingStopped = false;

function startSegmentRecording(daemonId, filename, url) {
    activeSegmentRecorderId = daemonId;
    isSegmentRecordingStopped = false;

    // Reset document variables
    document.documentElement.setAttribute('data-jadman-record', 'true');
    document.documentElement.setAttribute('data-jadman-record-id', daemonId);
    document.documentElement.setAttribute('data-jadman-record-index', '0');

    // Create a beautiful recording status overlay (using stealth shadow root)
    const overlay = document.createElement("div");
    overlay.style.cssText = `
        position: fixed;
        bottom: 20px;
        right: 20px;
        background: rgba(17, 17, 17, 0.95);
        color: #fff;
        border: 2px solid #00ff88;
        border-radius: 12px;
        padding: 12px 18px;
        display: flex;
        align-items: center;
        gap: 12px;
        z-index: 2147483647;
        box-shadow: 0 4px 15px rgba(0,0,0,0.5);
        backdrop-filter: blur(4px);
        pointer-events: auto !important;
    `;

    const dot = document.createElement("div");
    dot.style.cssText = `
        width: 10px;
        height: 10px;
        background: #00ff88;
        border-radius: 50%;
        animation: jadm-pulse 1s infinite alternate;
    `;

    const text = document.createElement("span");
    text.innerText = "JADMan Recording Segments... 0 chunks";
    text.style.fontFamily = "sans-serif";
    text.style.fontSize = "13px";
    text.style.fontWeight = "bold";

    const stopBtn = document.createElement("button");
    stopBtn.innerText = "Stop & Save";
    stopBtn.style.cssText = `
        background: #ff3333;
        color: #fff;
        border: none;
        border-radius: 6px;
        padding: 6px 12px;
        cursor: pointer;
        font-weight: bold;
        font-family: sans-serif;
        font-size: 12px;
        transition: background 0.2s;
    `;
    stopBtn.onmouseover = () => stopBtn.style.background = "#cc0000";
    stopBtn.onmouseout = () => stopBtn.style.background = "#ff3333";
    
    stopBtn.onclick = () => {
        stopSegmentRecording();
    };

    activeSegmentOverlay = overlay;

    overlay.appendChild(dot);
    overlay.appendChild(text);
    overlay.appendChild(stopBtn);
    
    const root = getStealthShadowRoot();
    nativeAppendChild.call(root, overlay);

    // Watch for chunk updates
    const observer = new MutationObserver(() => {
        const index = document.documentElement.getAttribute('data-jadman-record-index') || '0';
        text.innerText = `JADMan Recording Segments... ${index} chunks`;
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-jadman-record-index'] });
    
    overlay._observer = observer;
}

function stopSegmentRecording() {
    isSegmentRecordingStopped = true;
    document.documentElement.removeAttribute('data-jadman-record');
    document.documentElement.removeAttribute('data-jadman-record-id');
    document.documentElement.removeAttribute('data-jadman-record-index');

    if (activeSegmentOverlay) {
        if (activeSegmentOverlay._observer) {
            activeSegmentOverlay._observer.disconnect();
        }
        try {
            activeSegmentOverlay.remove();
        } catch(e) {}
        activeSegmentOverlay = null;
    }

    if (activeSegmentRecorderId) {
        chrome.runtime.sendMessage({
            cmd: 'SiphonChunk',
            daemon_id: activeSegmentRecorderId,
            chunk_index: 999999,
            is_last: true,
            filename: 'captured_stream.mp4',
            total_size: 0,
            data: []
        });
        activeSegmentRecorderId = null;
    }
}


// 1. Force enable right-click with extreme prejudice
function unblockRightClick() {
    const events = ["contextmenu", "copy", "cut", "paste", "selectstart"];
    events.forEach(name => {
        window.addEventListener(name, function(e) {
            e.stopPropagation();
        }, true);
    });
    // Neuter any inline handlers
    document.oncontextmenu = null;
    if (document.body) document.body.oncontextmenu = null;
}
unblockRightClick();

// Siphon hook code now executed via siphon_hook.js

// 3. LISTEN FOR SIPHONED DATA
window.addEventListener(JADMAN_COMM_TOKEN, (event) => {
    if (isPaused) return;
    const { url, type, data, mime, priority } = event.detail || {};
    if (!chrome.runtime?.id) return;
    
    if (type === 'MEDIA_DETECTED') {
        const { mediaId, src, tagName } = data;
        const el = document.querySelector(`[data-jadman-media-id="${mediaId}"]`);
        if (el) {
            let container = el.closest('.player, [class*="player"], [id*="player"], .video-container');
            if (!container) container = el.parentElement;
            if (container) {
                const isBlob = src && src.startsWith("blob:");
                const isGrabberFallback = isBlob || !src;
                injectMediaIcon(container, isGrabberFallback ? null : src, isGrabberFallback);
            }
        }
        return;
    }

    if (type === 'APPEND_BUFFER' && data) {
        // Send raw chunk to background if in recording mode
        const activeRecordId = document.documentElement.getAttribute('data-jadman-record-id');
        const chunkIndexStr = document.documentElement.getAttribute('data-jadman-record-index') || '0';
        const chunkIndex = parseInt(chunkIndexStr, 10);
        document.documentElement.setAttribute('data-jadman-record-index', (chunkIndex + 1).toString());
        
        chrome.runtime.sendMessage({
            cmd: 'SiphonChunk',
            daemon_id: activeRecordId,
            chunk_index: chunkIndex,
            is_last: false,
            filename: 'captured_stream.mp4',
            total_size: 0,
            data: Array.from(new Uint8Array(data))
        });
        return;
    }

    chrome.runtime.sendMessage({
        action: 'siphoned_data',
        url: url,
        mime: mime,
        priority: priority,
        size: data ? (data.byteLength || data.size || data.length) : 0
    });
});

let currentDownloadMode = "general";

// Human-like Micro-Interaction Event Mimicking
function startHumanMimicry() {
    if (currentDownloadMode !== "ghost") return;
    
    setInterval(() => {
        try {
            if (document.hidden) return;

            // 1. Mouse move mimicry
            const x = Math.floor(Math.random() * window.innerWidth);
            const y = Math.floor(Math.random() * window.innerHeight);
            const mouseEvent = new MouseEvent('mousemove', {
                clientX: x,
                clientY: y,
                bubbles: true,
                cancelable: true,
                view: window
            });
            document.dispatchEvent(mouseEvent);

            // 2. Subtle scroll mimicry (scroll down 1px and back up)
            if (Math.random() > 0.6) {
                window.scrollBy(0, 1);
                setTimeout(() => window.scrollBy(0, -1), 100);
            }
            
            // 3. Heartbeat for video elements
            const videos = document.querySelectorAll('video');
            videos.forEach(v => {
                if (!v.paused) {
                    const hoverEvent = new MouseEvent('mouseover', { bubbles: true });
                    v.dispatchEvent(hoverEvent);
                }
            });
        } catch(e) {}
    }, 12000); // Every 12 seconds
}

function updatePauseState(paused) {
    if (paused) {
        document.documentElement.setAttribute('data-jadman-paused', 'true');
        
        if (floatingSelectionButton) {
            try { nativeRemove.call(floatingSelectionButton); } catch(e) { floatingSelectionButton.remove(); }
            floatingSelectionButton = null;
        }
        if (discoveryToast) {
            try { nativeRemove.call(discoveryToast); } catch(e) { discoveryToast.remove(); }
            discoveryToast = null;
        }
        if (globalFloatingMediaButton) {
            try { nativeRemove.call(globalFloatingMediaButton); } catch(e) { globalFloatingMediaButton.remove(); }
            globalFloatingMediaButton = null;
        }
        
        while (activeMediaWrappers.length > 0) {
            const item = activeMediaWrappers.pop();
            try { nativeRemove.call(item.wrapper); } catch(e) { item.wrapper.remove(); }
            item.targetElement.dataset[RANDOM_PREFIX + 'Injected'] = "false";
        }
        
        if (stealthShadowRoot) {
            const host = stealthShadowRoot.host;
            if (host) {
                try { nativeRemove.call(host); } catch(e) { host.remove(); }
            }
            stealthShadowRoot = null;
        }
    } else {
        document.documentElement.removeAttribute('data-jadman-paused');
        scanMedia();
    }
}

chrome.storage.local.get(['buttonPosition', 'downloadMode', 'isPaused'], (res) => {
    isPaused = !!res.isPaused;
    if (res.buttonPosition) currentIconPosition = res.buttonPosition;
    if (res.downloadMode) {
        currentDownloadMode = res.downloadMode;
        startHumanMimicry();
    }
    
    document.documentElement.setAttribute('data-jadman-mode', currentDownloadMode);
    updatePauseState(isPaused);
});

chrome.storage.onChanged.addListener((changes, area) => {
    if (area === 'local' && changes.isPaused) {
        isPaused = !!changes.isPaused.newValue;
        updatePauseState(isPaused);
    }
});

function createSelectionButton() {
    const btn = nativeCreateElement.call(document, "button");
    btn.id = SELECTION_BTN_ID;
    btn.innerText = "Download Selection with JADMan";
    btn.style.cssText = `position: absolute; z-index: 2147483647; background: #ff0000; color: white; border: none; padding: 6px 14px; border-radius: 4px; cursor: pointer; font-weight: bold; font-family: sans-serif; font-size: 13px; box-shadow: 0 4px 15px rgba(0,0,0,0.6); display: none; border: 1px solid rgba(255,255,255,0.2);`;
    const root = getStealthShadowRoot();
    nativeAppendChild.call(root, btn);
    return btn;
}

function updateGlobalFloatingButton(count) {
    if (window.self !== window.top) return; 
    if (currentDownloadMode === "ghost") {
        if (globalFloatingMediaButton) globalFloatingMediaButton.style.display = "none";
        return;
    }
    
    if (count > 0) {
        if (!globalFloatingMediaButton) {
            globalFloatingMediaButton = nativeCreateElement.call(document, "div");
            globalFloatingMediaButton.style.cssText = `position: fixed; right: -5px; top: 50%; transform: translateY(-50%); z-index: 2147483647; background: rgba(0, 0, 0, 0.85); color: #00ff00; border: 2px solid #00ff00; padding: 10px 15px; border-radius: 8px 0 0 8px; cursor: pointer; font-weight: bold; font-family: sans-serif; font-size: 14px; box-shadow: -2px 0 10px rgba(0,255,0,0.3); transition: right 0.2s, opacity 0.2s; opacity: 0.6; pointer-events: auto !important;`;
            
            globalFloatingMediaButton.onmouseenter = () => { globalFloatingMediaButton.style.opacity = "1"; globalFloatingMediaButton.style.right = "0px"; };
            globalFloatingMediaButton.onmouseleave = () => { globalFloatingMediaButton.style.opacity = "0.6"; globalFloatingMediaButton.style.right = "-5px"; };
            
            globalFloatingMediaButton.onclick = (e) => {
                e.preventDefault(); e.stopPropagation();
                if (chrome.runtime?.id) chrome.runtime.sendMessage({ action: "open_grabber" });
            };
            const root = getStealthShadowRoot();
            nativeAppendChild.call(root, globalFloatingMediaButton);
        }
        globalFloatingMediaButton.innerHTML = `📥 ${count} Media Found`;
        globalFloatingMediaButton.style.display = "block";
    } else {
        if (globalFloatingMediaButton) globalFloatingMediaButton.style.display = "none";
    }
}

function showDiscoveryToast(count) {
    if (!chrome.runtime?.id) return; 
    if (window.self !== window.top) return; 
    if (currentDownloadMode === "ghost") return; 

    if (discoveryToast) {
        try {
            nativeRemove.call(discoveryToast);
        } catch(e) {
            discoveryToast.remove();
        }
        discoveryToast = null;
    }
    discoveryToast = nativeCreateElement.call(document, "div");
    discoveryToast.style.cssText = `position: fixed; bottom: 20px; right: 20px; z-index: 2147483647; background: #222; color: white; border: 1px solid #ff0000; padding: 10px 15px; border-radius: 8px; font-family: sans-serif; font-size: 12px; box-shadow: 0 4px 15px rgba(0,0,0,0.5); cursor: pointer; animation: fadeIn 0.3s;`;
    discoveryToast.innerHTML = `<b>JADMan:</b> Found <b>${count}</b> media stream(s) <span style="margin-left: 10px; color: #ff0000;">Download All »</span>`;
    discoveryToast.onclick = () => {
        if (chrome.runtime?.id) chrome.runtime.sendMessage({ action: "open_grabber" });
    };
    const root = getStealthShadowRoot();
    nativeAppendChild.call(root, discoveryToast);
    
    const currentToast = discoveryToast;
    setTimeout(() => {
        if (discoveryToast === currentToast) {
            try {
                nativeRemove.call(discoveryToast);
            } catch(e) {
                discoveryToast.remove();
            }
            discoveryToast = null;
        }
    }, 8000);
}

function isDownloadable(url) {
    if (!url || typeof url !== "string") return false;
    try {
        const lower = url.toLowerCase();
        if (IDM_EXTENSIONS.some(ext => lower.endsWith("." + ext)) || /\.r\d{2}$/.test(lower)) return true;
        if (lower.includes("youtube.com/watch") || lower.includes("youtu.be/")) return true;
        return false;
    } catch(e) { return false; }
}

function getLinksFromSelection() {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return [];
    const links = [];
    document.querySelectorAll("a").forEach(a => {
        try {
            if (selection.containsNode(a, true)) {
                if (a.href && !a.href.includes(window.location.href + "#")) {
                    links.push({ url: a.href, text: a.innerText.trim() || a.href.split('/').pop() });
                }
            }
        } catch(e) {}
    });
    const selectedText = (selection.toString() || "").trim();
    if (selectedText.startsWith("http")) {
        if (!links.some(l => l.url === selectedText)) {
            links.push({ url: selectedText, text: selectedText.split('/').pop() });
        }
    }
    return links;
}

document.addEventListener("mouseup", (e) => {
    if (isPaused) return;
    if (currentDownloadMode === "ghost") return;
    setTimeout(() => {
        const links = getLinksFromSelection();
        if (links.length > 0) {
            if (!floatingSelectionButton) floatingSelectionButton = createSelectionButton();
            floatingSelectionButton.style.left = `${e.pageX + 10}px`;
            floatingSelectionButton.style.top = `${e.pageY + 10}px`;
            floatingSelectionButton.style.display = "block";
            floatingSelectionButton.onclick = (ev) => {
                ev.preventDefault(); ev.stopPropagation();
                if (!chrome.runtime?.id) return;
                chrome.storage.local.set({ grabbedLinks: links }, () => {
                    chrome.runtime.sendMessage({ action: "open_grabber" });
                    floatingSelectionButton.style.display = "none";
                });
            };
        } else {
            if (floatingSelectionButton) floatingSelectionButton.style.display = "none";
        }
    }, 50);
});

document.addEventListener("mousedown", (e) => {
    if (floatingSelectionButton) {
        const root = floatingSelectionButton.getRootNode();
        if (root && root.host && e.target !== root.host) {
            floatingSelectionButton.style.display = "none";
        }
    }
});

document.addEventListener("click", (e) => {
    if (isPaused) return;
    if (currentDownloadMode === "ghost") return;
    const link = e.target.closest("a");
    if (!link || !link.href || !chrome.runtime?.id) return;
    const url = link.href.toLowerCase();
    const text = (link.innerText || "").toLowerCase();
    const isNavigation = url.includes("youtube.com/watch") || url.includes("youtu.be/");
    if (isDownloadable(url) && !isNavigation) {
        if (text.includes("download") || text.includes("save") || link.hasAttribute("download") || [".zip", ".rar", ".exe", ".msi"].some(ext => url.includes(ext))) {
            if (e.button === 0 && !e.ctrlKey && !e.metaKey && !e.shiftKey && !e.altKey) {
                e.preventDefault(); e.stopPropagation();
                chrome.runtime.sendMessage({ action: "request_download", url: link.href });
            }
        }
    }
}, true);

chrome.runtime.onMessage.addListener((message) => {
    if (message.action === "media_discovered") {
        if (chrome.runtime?.id) {
            chrome.runtime.sendMessage({ action: "get_tab_discovery" }, (response) => {
                if (response && response.urls && response.urls.length > 0) {
                    showDiscoveryToast(response.urls.length);
                }
            });
        }
    } else if (message.action === "update_position") {
        currentIconPosition = message.position;
        activeMediaWrappers.forEach(item => {
            applyIconPosition(item.icon, message.position);
        });
    } else if (message.action === "start_webgl_capture") {
        console.log("[JADMan Content Script] Received start_webgl_capture message:", message);
        let canvas = findCanvas();
        if (!canvas) {
            console.log("[JADMan Content Script] Canvas not found immediately. Polling...");
            let attempts = 0;
            const interval = setInterval(() => {
                canvas = findCanvas();
                attempts++;
                if (canvas || attempts > 30) {
                    clearInterval(interval);
                    if (canvas) {
                        console.log("[JADMan Content Script] Canvas found after polling:", canvas);
                        startRecordingWebGL(canvas, message);
                    } else {
                        console.error("[JADMan Content Script] No canvas found after 30 attempts.");
                        alert("No canvas element found to capture in this tab!");
                    }
                }
            }, 100);
        } else {
            console.log("[JADMan Content Script] Canvas found immediately:", canvas);
            startRecordingWebGL(canvas, message);
        }
    } else if (message.action === "stop_webgl_capture") {
        console.log("[JADMan Content Script] Received programmatic stop_webgl_capture message.");
        stopWebGLRecording();
    } else if (message.action === "start_segment_recording") {
        console.log("[JADMan Content Script] Received start_segment_recording message:", message);
        startSegmentRecording(message.daemonId, message.filename, message.url);
    } else if (message.action === "stop_segment_recording") {
        console.log("[JADMan Content Script] Received programmatic stop_segment_recording message.");
        stopSegmentRecording();
    } else if (message.action === "start_browser_fetch") {
        console.log("[JADMan Content Script] Received start_browser_fetch message:", message);
        const { url, daemonId, folder } = message;
        (async () => {
            try {
                const response = await nativeFetch(url);
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status} ${response.statusText}`);
                }
                const totalSize = parseInt(response.headers.get('content-length') || '0', 10);
                const contentDisp = response.headers.get('content-disposition') || '';
                
                let filename = 'download';
                if (contentDisp) {
                    const match = contentDisp.match(/filename\*?=["']?(?:UTF-8'')?([^"';\n]+)["']?/i);
                    if (match && match[1]) {
                        filename = decodeURIComponent(match[1]);
                    } else {
                        const matchSimple = contentDisp.match(/filename=["']?([^"';\n]+)["']?/i);
                        if (matchSimple && matchSimple[1]) {
                            filename = matchSimple[1];
                        }
                    }
                }
                if (filename === 'download') {
                    filename = url.split('/').pop().split('?')[0] || 'download';
                }
                
                const reader = response.body.getReader();
                let chunkIndex = 0;
                let downloaded = 0;
                
                while (true) {
                    const { done, value } = await reader.read();
                    const isLast = done;
                    const chunkData = done ? new Uint8Array(0) : value;
                    
                    chrome.runtime.sendMessage({
                        cmd: "SiphonChunk",
                        daemon_id: daemonId,
                        chunk_index: chunkIndex,
                        is_last: isLast,
                        filename: filename,
                        total_size: totalSize,
                        data: Array.from(chunkData)
                    });
                    
                    if (done) break;
                    chunkIndex++;
                    downloaded += chunkData.length;
                }
                console.log(`[JADMan Content Script] Browser fetch download completed: ${filename}`);
            } catch(err) {
                console.error("[JADMan Content Script] Browser fetch error:", err);
                chrome.runtime.sendMessage({ cmd: "StopDownload", id: daemonId });
            }
        })();
    } else if (message.action === "hunt_links") {
        const links = [];
        
        // Scan standard anchors
        const anchors = document.querySelectorAll('a');
        anchors.forEach(a => {
            const href = a.href || '';
            const text = (a.innerText || '').trim();
            const downloadAttr = a.getAttribute('download');
            
            const matchesExt = /\.(pdf|zip|rar|tar|gz|mp4|webm|mp3|m4a|dmg|exe|apk|epub)$/i.test(href);
            const matchesText = /download|get|save/i.test(text) || /download/i.test(a.className) || /download/i.test(a.id);
            
            if (href && (matchesExt || matchesText || downloadAttr)) {
                links.push({
                    url: href,
                    text: text || downloadAttr || href.split('/').pop().split('?')[0] || "Download Link",
                    source: "Anchor"
                });
            }
        });
        
        // Scan buttons that might trigger downloads
        const buttons = document.querySelectorAll('button');
        buttons.forEach(btn => {
            const text = (btn.innerText || '').trim();
            const onclick = btn.getAttribute('onclick') || '';
            
            const matchesText = /download|get|save/i.test(text) || /download/i.test(btn.className) || /download/i.test(btn.id);
            
            if (matchesText) {
                links.push({
                    url: onclick ? (onclick.match(/https?:\/\/[^\s'"]+/) || [''])[0] : '',
                    text: text || "Download Button",
                    source: "Button",
                    actionable: true
                });
            }
        });

        sendResponse({ links: links.slice(0, 50) });
    } else if (message.action === "start_browser_fetch") {
        console.log("[JADMan Content Script] Received start_browser_fetch message:", message);
        const { url, daemonId, folder } = message;
        (async () => {
            try {
                const response = await nativeFetch(url);
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status} ${response.statusText}`);
                }
                const totalSize = parseInt(response.headers.get('content-length') || '0', 10);
                const contentDisp = response.headers.get('content-disposition') || '';
                
                let filename = 'download';
                if (contentDisp) {
                    const match = contentDisp.match(/filename\*?=["']?(?:UTF-8'')?([^"';\n]+)["']?/i);
                    if (match && match[1]) {
                        filename = decodeURIComponent(match[1]);
                    } else {
                        const matchSimple = contentDisp.match(/filename=["']?([^"';\n]+)["']?/i);
                        if (matchSimple && matchSimple[1]) {
                            filename = matchSimple[1];
                        }
                    }
                }
                if (filename === 'download') {
                    filename = url.split('/').pop().split('?')[0] || 'download';
                }
                
                const reader = response.body.getReader();
                let chunkIndex = 0;
                let downloaded = 0;
                
                while (true) {
                    const { done, value } = await reader.read();
                    const isLast = done;
                    const chunkData = done ? new Uint8Array(0) : value;
                    
                    chrome.runtime.sendMessage({
                        cmd: "SiphonChunk",
                        daemon_id: daemonId,
                        chunk_index: chunkIndex,
                        is_last: isLast,
                        filename: filename,
                        total_size: totalSize,
                        data: Array.from(chunkData)
                    });
                    
                    if (done) break;
                    chunkIndex++;
                    downloaded += chunkData.length;
                }
                console.log(`[JADMan Content Script] Browser fetch download completed: ${filename}`);
            } catch(err) {
                console.error("[JADMan Content Script] Browser fetch error:", err);
                chrome.runtime.sendMessage({ cmd: "StopDownload", id: daemonId });
            }
        })();
    }
});



function findCanvas() {
    const canvases = Array.from(document.querySelectorAll('canvas'));
    if (canvases.length === 0) return null;
    // Sort by visible area (width * height) descending
    canvases.sort((a, b) => (b.offsetWidth * b.offsetHeight) - (a.offsetWidth * a.offsetHeight));
    return canvases[0];
}

function startRecordingWebGL(canvas, message) {
    const daemonId = message.daemonId;
    const pageTitle = (document.title || "webgl_capture").replace(/[^a-z0-9]/gi, '_').toLowerCase();
    const filename = pageTitle + "_capture.webm";

    console.log("[JADMan Content Script] Initializing recording. Filename:", filename, "Daemon ID:", daemonId);

    if (!nativeCaptureStream) {
        console.error("[JADMan Content Script] nativeCaptureStream is missing!");
        alert("Your browser does not support canvas stream capture.");
        return;
    }
    const stream = nativeCaptureStream.call(canvas, 30); // 30 fps
    
    // Setup MediaRecorder
    let options = { mimeType: 'video/webm' };
    if (typeof MediaRecorder !== 'undefined') {
        if (MediaRecorder.isTypeSupported('video/webm;codecs=vp9')) {
            options = { mimeType: 'video/webm;codecs=vp9' };
        } else if (MediaRecorder.isTypeSupported('video/webm;codecs=vp8')) {
            options = { mimeType: 'video/webm;codecs=vp8' };
        }
    }
    console.log("[JADMan Content Script] MediaRecorder options selected:", options);

    let recorder;
    try {
        recorder = new MediaRecorder(stream, options);
    } catch(e) {
        console.error("[JADMan Content Script] Failed to create MediaRecorder with options:", e);
        try {
            recorder = new MediaRecorder(stream);
        } catch(err) {
            console.error("[JADMan Content Script] Failed to create default MediaRecorder:", err);
            alert("Failed to start MediaRecorder: " + err.message);
            return;
        }
    }

    activeWebGLRecorder = recorder;
    activeWebGLStream = stream;
    isWebGLRecordingStopped = false;

    let chunkIndex = 0;

    // We store chunks and upload them
    recorder.ondataavailable = async (event) => {
        if (event.data && event.data.size > 0) {
            const currentChunkIndex = chunkIndex++;
            const isLast = isWebGLRecordingStopped || recorder.state === "inactive";
            console.log("[JADMan Content Script] Chunk available. Index:", currentChunkIndex, "Size:", event.data.size, "isLast:", isLast);
            
            // Upload chunk
            const arrayBuffer = await event.data.arrayBuffer();
            uploadWebGLChunk(daemonId, filename, currentChunkIndex, isLast, arrayBuffer);
        } else {
            console.log("[JADMan Content Script] Chunk available but empty.");
        }
    };



    // Show UI Overlay inside Closed Shadow Root
    const overlay = document.createElement("div");
    overlay.style.cssText = `
        position: fixed;
        top: 20px;
        left: 50%;
        transform: translateX(-50%);
        z-index: 2147483647;
        background: rgba(20, 20, 20, 0.95);
        border: 2px solid #ff3333;
        border-radius: 12px;
        padding: 12px 24px;
        display: flex;
        align-items: center;
        gap: 16px;
        box-shadow: 0 8px 32px rgba(0,0,0,0.8);
        color: #fff;
        font-family: sans-serif;
        font-size: 14px;
        font-weight: bold;
        pointer-events: auto !important;
    `;

    const dot = document.createElement("div");
    dot.style.cssText = `
        width: 12px;
        height: 12px;
        background: #ff3333;
        border-radius: 50%;
        animation: jadm-pulse 1s infinite alternate;
    `;

    // Add keyframes style for pulsing dot if not present
    if (!document.getElementById("jadm-pulse-style")) {
        const style = document.createElement("style");
        style.id = "jadm-pulse-style";
        style.textContent = `
            @keyframes jadm-pulse {
                from { opacity: 0.3; transform: scale(0.8); }
                to { opacity: 1; transform: scale(1.2); }
            }
        `;
        document.head.appendChild(style);
    }

    const text = document.createElement("span");
    text.innerText = "JADMan WebGL Recording... 00:00";

    const stopBtn = document.createElement("button");
    stopBtn.innerText = "Stop & Save";
    stopBtn.style.cssText = `
        background: #ff3333;
        color: #fff;
        border: none;
        border-radius: 6px;
        padding: 6px 12px;
        cursor: pointer;
        font-weight: bold;
        font-family: sans-serif;
        font-size: 12px;
        transition: background 0.2s;
    `;
    stopBtn.onmouseover = () => stopBtn.style.background = "#cc0000";
    stopBtn.onmouseout = () => stopBtn.style.background = "#ff3333";
    
    stopBtn.onclick = () => {
        stopWebGLRecording();
    };

    activeWebGLOverlay = overlay;

    overlay.appendChild(dot);
    overlay.appendChild(text);
    overlay.appendChild(stopBtn);
    
    const root = getStealthShadowRoot();
    nativeAppendChild.call(root, overlay);

    // Start timer
    let seconds = 0;
    const timer = setInterval(() => {
        if (isWebGLRecordingStopped) {
            clearInterval(timer);
            return;
        }
        seconds++;
        const mins = Math.floor(seconds / 60).toString().padStart(2, '0');
        const secs = (seconds % 60).toString().padStart(2, '0');
        text.innerText = `JADMan WebGL Recording... ${mins}:${secs}`;
    }, 1000);

    // Start MediaRecorder (slice into 1 second intervals)
    recorder.start(1000);
}

async function uploadWebGLChunk(daemonId, filename, chunkIndex, isLast, arrayBuffer) {
    try {
        const DAEMON_URL = "http://127.0.0.1:6246";
        console.log("[JADMan Content Script] Uploading WebGL chunk to daemon...", chunkIndex, "Length:", arrayBuffer.byteLength);
        await chrome.runtime.sendMessage({
            cmd: "SiphonChunk",
            daemon_id: daemonId,
            chunk_index: chunkIndex,
            is_last: isLast,
            filename: filename,
            total_size: arrayBuffer.byteLength,
            data: Array.from(new Uint8Array(arrayBuffer))
        });
        console.log("[JADMan Content Script] Chunk upload response success.");
    } catch(e) {
        console.error("[JADMan Content Script] Failed to upload WebGL chunk:", e);
    }
}


function applyIconPosition(icon, position) {
    icon.style.top = ""; icon.style.bottom = ""; icon.style.left = ""; icon.style.right = ""; icon.style.transform = "";
    switch(position) {
        case "top-left": icon.style.top = "10px"; icon.style.left = "10px"; break;
        case "bottom-right": icon.style.bottom = "10px"; icon.style.right = "10px"; break;
        case "bottom-left": icon.style.bottom = "10px"; icon.style.left = "10px"; break;
        case "center": icon.style.top = "50%"; icon.style.left = "50%"; icon.style.transform = "translate(-50%, -50%)"; break;
        default: icon.style.top = "10px"; icon.style.right = "10px"; break; 
    }
}

function injectMediaIcon(targetElement, mediaUrl, isGrabberFallback = false, attempt = 0) {
    if (currentDownloadMode === "ghost") return;
    if (!targetElement || targetElement.tagName === "BODY" || attempt > 5) return;
    
    const wrapperTagName = 'x-' + RANDOM_PREFIX + '-wrap';
    const existingWrapper = targetElement.querySelector(wrapperTagName);
    if (targetElement.dataset[RANDOM_PREFIX + 'Injected'] === "true") {
        if (existingWrapper) {
            return;
        } else {
            targetElement.dataset[RANDOM_PREFIX + 'Injected'] = "false";
        }
    }
    
    const style = window.getComputedStyle(targetElement);
    if (style.position === "static") {
        targetElement.style.position = "relative";
    }

    // Create wrapper element with a randomized tag name
    const wrapper = nativeCreateElement.call(document, wrapperTagName);
    wrapper.style.cssText = `position: absolute; top: 0; left: 0; width: 100%; height: 100%; pointer-events: none; z-index: 2147483647;`;
    
    // Attach closed shadow root using native attachShadow
    const shadow = nativeAttachShadow.call(wrapper, { mode: 'closed' });

    const icon = nativeCreateElement.call(document, "div");
    icon.className = ICON_CLASS;
    icon.innerHTML = "📥";
    icon.style.cssText = `position: absolute; z-index: 2147483647 !important; background: rgba(0, 255, 0, 0.1); color: #00ff00; border: 2px solid #00ff00; padding: 4px 8px; border-radius: 50%; cursor: pointer; font-size: 16px; opacity: 0.6; transition: all 0.3s; pointer-events: auto !important; box-shadow: 0 0 10px rgba(0,255,0,0.5); display: flex; align-items: center; justify-content: center; backdrop-filter: blur(2px);`;
    
    applyIconPosition(icon, currentIconPosition);
    
    const show = () => { icon.style.opacity = "1"; icon.style.background = "rgba(0, 0, 0, 0.85)"; icon.innerHTML = "📥 JADMan"; icon.style.borderRadius = "4px"; };
    const hide = () => { icon.style.opacity = "0.6"; icon.style.background = "rgba(0, 255, 0, 0.1)"; icon.innerHTML = "📥"; icon.style.borderRadius = "50%"; };

    icon.addEventListener("mouseenter", show);
    icon.addEventListener("mouseleave", hide);

    icon.onclick = (e) => { 
        e.preventDefault(); e.stopPropagation(); 
        if (chrome.runtime?.id) {
            if (isGrabberFallback) {
                chrome.runtime.sendMessage({ action: "open_grabber" });
            } else {
                const mime = targetElement.tagName === "VIDEO" ? "video/mp4" : (targetElement.tagName === "AUDIO" ? "audio/mpeg" : null);
                chrome.runtime.sendMessage({ action: "request_download", url: mediaUrl, mime: mime }); 
            }
        }
    };
    
    nativeAppendChild.call(shadow, icon);
    nativeAppendChild.call(targetElement, wrapper);
    targetElement.dataset[RANDOM_PREFIX + 'Injected'] = "true";

    const observer = new MutationObserver(() => {
        if (!targetElement.contains(wrapper)) {
            try {
                nativeAppendChild.call(targetElement, wrapper);
            } catch (e) {}
        }
        if (wrapper.style.display === 'none' || wrapper.style.visibility === 'hidden') {
            wrapper.style.display = '';
            wrapper.style.visibility = '';
        }
        if (icon.style.display === 'none' || icon.style.visibility === 'hidden' || icon.style.opacity === '0') {
            icon.style.display = '';
            icon.style.visibility = '';
            icon.style.opacity = '0.6';
        }
    });
    observer.observe(targetElement, { childList: true, subtree: true, attributes: true, attributeFilter: ['style', 'class'] });

    const wrapperItem = { wrapper, targetElement, icon };
    activeMediaWrappers.push(wrapperItem);

    setTimeout(() => {
        const rect = icon.getBoundingClientRect();
        const computed = window.getComputedStyle(icon);
        const parentComputed = window.getComputedStyle(targetElement);
        const isInvisible = rect.width === 0 || rect.height === 0 || computed.display === 'none' || computed.visibility === 'hidden' || computed.opacity === '0';
        const isClipped = parentComputed.overflow === 'hidden' && (rect.right < 0 || rect.bottom < 0 || rect.left > window.innerWidth || rect.top > window.innerHeight);
        if (isInvisible || isClipped) {
            try {
                nativeRemove.call(wrapper);
            } catch(e) {
                wrapper.remove();
            }
            const idx = activeMediaWrappers.indexOf(wrapperItem);
            if (idx !== -1) activeMediaWrappers.splice(idx, 1);
            
            targetElement.dataset[RANDOM_PREFIX + 'Injected'] = "false";
            if (targetElement.parentElement) {
                injectMediaIcon(targetElement.parentElement, mediaUrl, isGrabberFallback, attempt + 1);
            }
        }
    }, 100);
}

function scanMedia() {
    if (isPaused) return;
    let mediaCount = 0;

    if (window.location.href.includes("youtube.com") || window.location.href.includes("youtu.be")) {
        if (window.self !== window.top) return;
        const ytPlayer = document.querySelector(".html5-video-player");
        if (ytPlayer) { injectMediaIcon(ytPlayer, window.location.href); mediaCount++; }
        const shortsPlayer = document.querySelector("ytd-reel-video-renderer[is-active]");
        if (shortsPlayer) { injectMediaIcon(shortsPlayer, window.location.href); mediaCount++; }
        updateGlobalFloatingButton(mediaCount);
        return; 
    }

    if (window.location.href.includes("x.com") || window.location.href.includes("twitter.com")) {
        document.querySelectorAll('[data-testid="videoComponent"], [data-testid="videoPlayer"], [data-testid="tweetPhoto"]').forEach(media => {
            injectMediaIcon(media, window.location.href, false);
            mediaCount++;
        });
    }

    document.querySelectorAll("video, audio").forEach(media => {
        const src = media.src || media.currentSrc;
        const isBlob = src && src.startsWith("blob:");
        const isGrabberFallback = isBlob || !src;
        let container = media.closest('.player, [class*="player"], [id*="player"], .video-container');
        if (!container) container = media.parentElement;
        if (container) {
            injectMediaIcon(container, isGrabberFallback ? null : src, isGrabberFallback);
            mediaCount++;
        }
    });

    if (window.self === window.top) {
        updateGlobalFloatingButton(mediaCount);
    }
}

setInterval(scanMedia, 3000);
scanMedia();
unblockRightClick();

nativeAddEventListener.call(window, "load", () => { scanMedia(); });
nativeAddEventListener.call(document, "DOMContentLoaded", () => { scanMedia(); });
nativeAddEventListener.call(window, "yt-navigate-finish", () => { scanMedia(); });
nativeAddEventListener.call(window, "popstate", () => { setTimeout(scanMedia, 500); });
