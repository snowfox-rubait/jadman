const params = new URLSearchParams(window.location.search);
const url = params.get("url") || "";
const mimeType = params.get("mime") || null;
document.getElementById("url").value = url;

// Detect and show playlist option
const hasPlaylist = url.includes("list=") || url.includes("playlist?list=");
if (hasPlaylist) {
    const wrap = document.getElementById("playlistOptionWrap");
    if (wrap) {
        wrap.style.display = "flex";
        const chk = document.getElementById("downloadPlaylist");
        if (chk) chk.checked = true;
    }
}

const DAEMON_URL = "http://127.0.0.1:6246";

const qualitySelect = document.getElementById("quality");
const statusDiv = document.getElementById("status");
const qualityLabel = document.getElementById("qualityLabel");
const ghostEngineWrap = document.getElementById("ghostEngineWrap");
const ghostEngineSelect = document.getElementById("ghostEngine");
const modeSelect = document.getElementById("mode");

let globalCookieString = "";
let globalNetscapeCookies = "";
let globalUserAgent = navigator.userAgent;

// Show/hide ghost engine selector based on mode
function updateGhostEngineVisibility() {
    const mode = modeSelect.value;
    ghostEngineWrap.style.display = (mode === "ghost") ? "block" : "none";
}

// Fetch cookies for exact and bridging domains (especially for X/Twitter)
async function getCookiesForUrl(targetUrl) {
    try {
        let allCookies = [];
        let seen = new Set();

        // 1. Query by exact URL + partitionKey wildcard to capture CHIPS cookies (cf_clearance, etc.)
        try {
            let cookiesByUrl = [];
            try {
                cookiesByUrl = await chrome.cookies.getAll({ url: targetUrl, partitionKey: {} });
            } catch(e) {
                cookiesByUrl = await chrome.cookies.getAll({ url: targetUrl });
            }
            for (let cookie of cookiesByUrl) {
                const key = cookie.name + cookie.domain + cookie.path;
                if (!seen.has(key)) {
                    seen.add(key);
                    allCookies.push(cookie);
                }
            }
        } catch(e) {
            console.error("Error querying by URL:", e);
        }

        // 2. Query by bridged domains (for X/Twitter)
        const urlObj = new URL(targetUrl);
        let domain = urlObj.hostname;
        let domains = [];
        if (domain.includes("x.com")) domains.push("twitter.com", "api.x.com");
        if (domain.includes("twitter.com")) domains.push("x.com", "api.twitter.com");

        for (let d of domains) {
            try {
                let c = [];
                try {
                    c = await chrome.cookies.getAll({ domain: d, partitionKey: {} });
                } catch(e) {
                    c = await chrome.cookies.getAll({ domain: d });
                }
                for (let cookie of c) {
                    const key = cookie.name + cookie.domain + cookie.path;
                    if (!seen.has(key)) {
                        seen.add(key);
                        allCookies.push(cookie);
                    }
                }
            } catch(e) {}
        }
        return allCookies;
    } catch(e) {
        return [];
    }
}

// Convert chrome cookies to Netscape format for yt-dlp
function buildNetscape(cookies) {
    let out = "# Netscape HTTP Cookie File\n";
    cookies.forEach(c => {
        const domain = c.domain;
        const flag = domain.startsWith('.') ? "TRUE" : "FALSE";
        const path = c.path;
        const secure = c.secure ? "TRUE" : "FALSE";
        const exp = c.expirationDate ? Math.floor(c.expirationDate) : 0;
        out += `${domain}\t${flag}\t${path}\t${secure}\t${exp}\t${c.name}\t${c.value}\n`;
    });
    return out;
}

async function init() {
    // Restore from storage if available
    const stored = await new Promise(resolve => {
        chrome.storage.local.get(['downloadMode'], (res) => resolve(res.downloadMode));
    });
    if (stored) {
        modeSelect.value = stored;
    }
    const mode = modeSelect.value;
    updateGhostEngineVisibility();

    globalCookieString = "";
    globalNetscapeCookies = "";

    // Capture session cookies for siphon or ghost modes
    if (mode === "siphon" || mode === "ghost") {
        const rawCookies = await getCookiesForUrl(url);
        if (rawCookies.length > 0) {
            globalCookieString = rawCookies.map(c => `${c.name}=${c.value}`).join("; ");
            globalNetscapeCookies = buildNetscape(rawCookies);
        }
    }

    const ghostEngine = ghostEngineSelect ? ghostEngineSelect.value : "ytdlp";
    const useNative = mode === "ghost" && ghostEngine === "chrome_native";
    const useBrowserFetch = mode === "ghost" && ghostEngine === "browser_fetch";
    const useDebugger = mode === "ghost" && ghostEngine === "debugger_capture";
    const useWebGL = mode === "ghost" && ghostEngine === "webgl_capture";
    
    if (useNative || useBrowserFetch || useDebugger || useWebGL) {
        statusDiv.innerText = useNative
            ? "⚡ Chrome Native mode — uses Chrome's real TLS. Quality selection not applicable."
            : (useBrowserFetch
               ? "🌐 Browser Fetch mode — streams through current tab's network. Quality selection not applicable."
               : (useDebugger
                  ? "🪲 Debugger Capture mode — attaches tab debugger to intercept socket. Quality selection not applicable."
                  : "🎨 WebGL Canvas Capture mode — records canvas frames to daemon. Quality selection not applicable."));
        if (qualityLabel) qualityLabel.innerText = "Quality (Not Applicable):";
        qualitySelect.disabled = true;
        return;
    }

    statusDiv.innerText = "Checking available qualities...";
    chrome.runtime.sendMessage({
        cmd: "GetFormats",
        ...{ 
            url: url, 
            cookies: globalCookieString || null,
            netscape_cookies: globalNetscapeCookies || null,
            userAgent: globalUserAgent,
            mode: mode,
            referer: params.get("referer") || null
        }
    }).then(response => {
        if (!response || response.error) {
            throw new Error(`Daemon error: ${response ? response.error : 'unknown'}`);
        }
        return response;
    })
    .then(data => {
        if (data.status === "ok" && data.formats && data.formats.length > 0) {
            statusDiv.innerText = "Qualities loaded.";
            if(qualityLabel) qualityLabel.innerText = "Quality:";
            qualitySelect.innerHTML = '<option value="">Best (Default)</option>';
            data.formats.reverse().forEach(fmt => {
                const opt = document.createElement("option");
                opt.value = fmt.id;
                opt.innerText = `${fmt.resolution} (${fmt.ext}) - ${fmt.note}`;
                qualitySelect.appendChild(opt);
            });
            qualitySelect.disabled = false;
        } else {
            statusDiv.innerText = "Single stream/file detected.";
            if(qualityLabel) qualityLabel.innerText = "Quality (Not Applicable):";
            qualitySelect.disabled = true;
        }
    })
    .catch(err => {
        statusDiv.innerText = "Direct file. Quality check skipped.";
        if(qualityLabel) qualityLabel.innerText = "Quality (Not Applicable):";
        qualitySelect.disabled = true;
    });
}

modeSelect.addEventListener("change", (e) => {
    chrome.storage.local.set({ downloadMode: e.target.value }, () => {
        init();
    });
});

ghostEngineSelect.addEventListener("change", () => {
    init();
});

init();

document.getElementById("cancelBtn").addEventListener("click", () => {
    window.close();
});

document.getElementById("downloadBtn").addEventListener("click", async () => {
    const folder = document.getElementById("folder").value;
    const format = qualitySelect.value;
    const writeSubs = document.getElementById("writeSubs").checked;
    const embedThumbnail = document.getElementById("embedThumbnail").checked;
    const embedChapters = document.getElementById("embedChapters").checked;
    const liveSupport = document.getElementById("liveSupport").checked;
    const liveFromStart = document.getElementById("liveFromStart").checked;
    const compressVideo = document.getElementById("compressVideo").checked;
    const recordSegments = document.getElementById("recordSegments") ? document.getElementById("recordSegments").checked : false;
    const downloadPlaylist = document.getElementById("downloadPlaylist") ? document.getElementById("downloadPlaylist").checked : false;
    const mode = modeSelect.value;
    const ghostEngine = ghostEngineSelect ? ghostEngineSelect.value : "ytdlp";
    const useDebugger = mode === "ghost" && ghostEngine === "debugger_capture";
    const useWebGL = mode === "ghost" && ghostEngine === "webgl_capture";
    const useNative = mode === "ghost" && ghostEngine === "chrome_native";
    const useBrowserFetch = mode === "ghost" && ghostEngine === "browser_fetch";

    if (recordSegments) {
        statusDiv.innerText = "📥 Starting Media Stream Segment Siphoning...";

        // 1. Get active tab to communicate with
        let targetTabId = null;
        const queryTabId = params.get("tabId");
        if (queryTabId) {
            targetTabId = parseInt(queryTabId, 10);
        } else {
            try {
                const tabs = await chrome.tabs.query({});
                let targetTab = tabs.find(t => t.active && t.url && !t.url.startsWith("chrome-extension://"));
                if (!targetTab) {
                    targetTab = tabs.find(t => t.url && (t.url === url || url.includes(t.url.split('?')[0])));
                }
                if (!targetTab) {
                    targetTab = tabs.find(t => t.url && !t.url.startsWith("chrome-extension://"));
                }
                if (targetTab) targetTabId = targetTab.id;
            } catch(e) {
                console.error("Error querying tab for segment siphoning:", e);
            }
        }

        if (!targetTabId) {
            alert("Cannot determine active tab to start stream recording.");
            return;
        }

        // 2. Register download task with daemon
        let daemonId = null;
        let targetFolder = folder;
        try {
            const regResp = await chrome.runtime.sendMessage({
                cmd: "AddDownload",
                ...{
                    url: url,
                    folder: folder,
                    mode: "siphon",
                    engine: "siphon_record",
                    userAgent: globalUserAgent
                }
            });
            const regData = regResp;
            daemonId = regData.id || null;
            if (regData.folder) {
                targetFolder = regData.folder;
            }
        } catch(e) {
            alert("Failed to connect to JADMan daemon. Is it running?");
            console.error("JADMan Error:", e);
            return;
        }

        if (!daemonId) {
            alert("Failed to register download task with JADMan daemon.");
            return;
        }

        // 3. Send message to content script of the tab to start segment siphoning
        chrome.tabs.sendMessage(targetTabId, {
            action: "start_segment_recording",
            daemonId: daemonId,
            filename: "captured_stream.mp4",
            url: url
        });

        statusDiv.innerText = `✅ Segment Recording started in tab.`;
        setTimeout(() => window.close(), 1500);
        return;
    }

    if (useWebGL) {
        statusDiv.innerText = "🎨 Starting WebGL Canvas Capture...";

        // 1. Get active tab to communicate with
        let targetTabId = null;
        const queryTabId = params.get("tabId");
        if (queryTabId) {
            targetTabId = parseInt(queryTabId, 10);
        } else {
            try {
                const tabs = await chrome.tabs.query({});
                let targetTab = tabs.find(t => t.active && t.url && !t.url.startsWith("chrome-extension://"));
                if (!targetTab) {
                    targetTab = tabs.find(t => t.url && (t.url === url || url.includes(t.url.split('?')[0])));
                }
                if (!targetTab) {
                    targetTab = tabs.find(t => t.url && !t.url.startsWith("chrome-extension://"));
                }
                if (targetTab) targetTabId = targetTab.id;
            } catch(e) {
                console.error("Error querying tab for WebGL capture:", e);
            }
        }

        if (!targetTabId) {
            alert("Cannot determine active tab to capture WebGL canvas.");
            return;
        }

        // 2. Register with daemon
        let daemonId = null;
        let targetFolder = folder;
        try {
            const regResp = await chrome.runtime.sendMessage({
                cmd: "AddDownload",
                ...{
                    url: url,
                    folder: folder,
                    mode: "ghost",
                    engine: "webgl_capture",
                    userAgent: globalUserAgent
                }
            });
            const regData = regResp;
            daemonId = regData.id || null;
            if (regData.folder) {
                targetFolder = regData.folder;
            }
        } catch(e) {
            alert("Failed to connect to JADMan daemon. Is it running?");
            console.error("JADMan Error:", e);
            return;
        }

        if (!daemonId) {
            alert("Failed to register download task with JADMan daemon.");
            return;
        }

        // 3. Send message to content script of the tab to start canvas recording
        chrome.tabs.sendMessage(targetTabId, {
            action: "start_webgl_capture",
            daemonId: daemonId,
            folder: targetFolder,
            url: url
        });

        statusDiv.innerText = `✅ WebGL Canvas Capture started in tab.`;
        setTimeout(() => window.close(), 1500);
        return;
    }

    if (useDebugger) {
        statusDiv.innerText = "🪲 Starting Debugger Socket Capture...";

        // 1. Get active tab to attach debugger to
        let targetTabId = null;
        const queryTabId = params.get("tabId");
        if (queryTabId) {
            targetTabId = parseInt(queryTabId, 10);
        } else {
            try {
                const tabs = await chrome.tabs.query({});
                // Find a tab that is active and not an extension page
                let targetTab = tabs.find(t => t.active && t.url && !t.url.startsWith("chrome-extension://"));
                if (!targetTab) {
                    // Fallback to any tab matching our URL
                    targetTab = tabs.find(t => t.url && (t.url === url || url.includes(t.url.split('?')[0])));
                }
                if (!targetTab) {
                    // Fallback to any non-extension tab
                    targetTab = tabs.find(t => t.url && !t.url.startsWith("chrome-extension://"));
                }
                if (targetTab) targetTabId = targetTab.id;
            } catch(e) {
                console.error("Error querying tab for debugger:", e);
            }
        }

        if (!targetTabId) {
            alert("Cannot determine active tab to attach debugger.");
            return;
        }

        // 2. Register with daemon
        let daemonId = null;
        let targetFolder = folder;
        try {
            const regResp = await chrome.runtime.sendMessage({
                cmd: "AddDownload",
                ...{
                    url: url,
                    folder: folder,
                    mode: "ghost",
                    engine: "debugger_capture",
                    userAgent: globalUserAgent
                }
            });
            const regData = regResp;
            daemonId = regData.id || null;
            if (regData.folder) {
                targetFolder = regData.folder;
            }
        } catch(e) {
            alert("Failed to connect to JADMan daemon. Is it running?");
            console.error("JADMan Error:", e);
            return;
        }

        if (!daemonId) {
            alert("Failed to register download task with JADMan daemon.");
            return;
        }

        // 3. Request background to attach debugger and stream target URL response
        chrome.runtime.sendMessage({
            action: "start_debugger_capture",
            url: url,
            tabId: targetTabId,
            daemonId: daemonId,
            folder: targetFolder
        });

        statusDiv.innerText = `✅ Debugger Capture started.`;
        setTimeout(() => window.close(), 1500);
        return;
    }

    if (useBrowserFetch) {
        statusDiv.innerText = "🌐 Starting Browser Fetch download...";

        // 1. Register with daemon
        let daemonId = null;
        let targetFolder = folder;
        try {
            const regResp = await chrome.runtime.sendMessage({
                cmd: "AddDownload",
                ...{
                    url: url,
                    folder: folder,
                    mode: "ghost",
                    engine: "browser_fetch",
                    userAgent: globalUserAgent
                }
            });
            const regData = regResp;
            daemonId = regData.id || null;
            if (regData.folder) {
                targetFolder = regData.folder;
            }
        } catch(e) {
            alert("Failed to connect to JADMan daemon. Is it running?");
            console.error("JADMan Error:", e);
            return;
        }

        if (!daemonId) {
            alert("Failed to register download task with JADMan daemon.");
            return;
        }

        // 2. Request background to fetch in browser context and stream chunks
        chrome.runtime.sendMessage({
            action: "start_browser_fetch",
            url: url,
            daemonId: daemonId,
            folder: targetFolder,
            userAgent: globalUserAgent
        });

        statusDiv.innerText = `✅ Browser Fetch download started.`;
        setTimeout(() => window.close(), 1500);
        return;
    }

    if (useNative) {
        // === CHROME NATIVE DOWNLOAD PATH ===
        // Let Chrome itself download using its real TLS stack + real session cookies.
        // We register the job with the daemon first so it can track progress.
        statusDiv.innerText = "⚡ Starting Chrome Native download...";

        // 1. Register with daemon so it tracks this in the queue
        const fname = url.split('/').pop().split('?')[0] || 'download';
        let daemonId = null;
        let targetFolder = folder;
        try {
            const regResp = await chrome.runtime.sendMessage({
                cmd: "AddDownload",
                ...{
                    url: url,
                    folder: folder,
                    mode: "ghost",
                    engine: "chrome_native",
                    userAgent: globalUserAgent
                }
            });
            const regData = regResp;
            daemonId = regData.id || null;
            if (regData.folder) {
                targetFolder = regData.folder;
            }
        } catch(e) {
            console.warn("Could not register with daemon, continuing native download anyway:", e);
        }

        // 2. Send message to background to whitelist this URL so it doesn't get cancelled by the interceptor
        chrome.runtime.sendMessage({
            action: "whitelist_url",
            url: url
        }, () => {
            const downloadFilename = `JADMan/${fname}`;
            chrome.downloads.download({
                url: url,
                filename: downloadFilename,
                conflictAction: "overwrite",
                saveAs: false
            }, (downloadId) => {
                if (chrome.runtime.lastError) {
                    statusDiv.innerText = "❌ Chrome download failed: " + chrome.runtime.lastError.message;
                    return;
                }
                // 3. Tell background to monitor this download and report to daemon
                chrome.runtime.sendMessage({
                    action: "monitor_native_download",
                    downloadId: downloadId,
                    daemonId: daemonId,
                    targetFolder: targetFolder,
                    originalUrl: url
                });
                statusDiv.innerText = `✅ Chrome Native download started (ID: ${downloadId})`;
                setTimeout(() => window.close(), 1500);
            });
        });
        return;
    }

    // === STANDARD DAEMON PATH (aria2c / yt-dlp) ===
    const sendCookies = mode === "siphon" || mode === "ghost";
    
    chrome.runtime.sendMessage({
        cmd: "AddDownload",
        ...{ 
            url: url, 
            folder: folder,
            format: format || null,
            mime_type: mimeType,
            cookies: sendCookies ? (globalCookieString || null) : null,
            netscape_cookies: sendCookies ? (globalNetscapeCookies || null) : null,
            userAgent: globalUserAgent,
            mode: mode,
            engine: mode === "ghost" ? ghostEngine : null,
            write_subs: writeSubs,
            embed_thumbnail: embedThumbnail,
            embed_chapters: embedChapters,
            live_support: liveSupport,
            live_from_start: liveFromStart,
            compress_video: compressVideo,
            download_playlist: downloadPlaylist,
            referer: params.get("referer") || null
        }
    }).then(data => {
        window.close();
    })
    .catch(error => {
        alert("Failed to connect to JADMan daemon. Is it running?");
        console.error("JADMan Error:", error);
    });
});

// INTERCEPTOR PAUSE/RESUME TOGGLE SYSTEM
const statusLabel = document.getElementById("statusLabel");
const toggleActiveBtn = document.getElementById("toggleActiveBtn");
let isPaused = false;

chrome.storage.local.get(['isPaused'], (res) => {
    isPaused = !!(res && res.isPaused);
    updateToggleUI();
});

function updateToggleUI() {
    if (isPaused) {
        statusLabel.innerText = "🔴 JADMan Paused";
        statusLabel.style.color = "#ff3333";
        toggleActiveBtn.innerText = "Resume";
        toggleActiveBtn.style.background = "#00ff88";
        toggleActiveBtn.style.color = "#000";
    } else {
        statusLabel.innerText = "🟢 JADMan Active";
        statusLabel.style.color = "#00ff88";
        toggleActiveBtn.innerText = "Pause";
        toggleActiveBtn.style.background = "#ff3333";
        toggleActiveBtn.style.color = "#fff";
    }
}

toggleActiveBtn.addEventListener("click", () => {
    isPaused = !isPaused;
    chrome.storage.local.set({ isPaused }, () => {
        updateToggleUI();
    });
});

document.getElementById("liveFromStart").addEventListener("change", (e) => {
    document.getElementById("liveFromStartWarning").style.display = e.target.checked ? "block" : "none";
});
