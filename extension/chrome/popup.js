// Safe chrome.runtime.sendMessage wrapper to prevent "No SW" / "Context Invalidated" warnings
if (typeof chrome !== 'undefined' && chrome.runtime) {
    const originalSendMessage = chrome.runtime.sendMessage;
    chrome.runtime.sendMessage = function(message, callback) {
        if (!chrome.runtime?.id) return;
        try {
            if (typeof callback === 'function') {
                return originalSendMessage.call(chrome.runtime, message, (response) => {
                    const err = chrome.runtime.lastError;
                    if (!err) {
                        callback(response);
                    }
                });
            } else {
                const p = originalSendMessage.call(chrome.runtime, message);
                if (p && typeof p.catch === 'function') {
                    return p.catch(() => {});
                }
                return p;
            }
        } catch(e) {}
    };
}

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

const qualitySelect = document.getElementById("quality");
const statusDiv = document.getElementById("status");
const qualityLabel = document.getElementById("qualityLabel");

let globalCookieString = "";
let globalNetscapeCookies = "";
let globalUserAgent = navigator.userAgent;
let activeMode = "siphon";
let activeGhostEngine = "ytdlp";

// Fetch cookies for exact and bridging domains (especially for X/Twitter)
async function getCookiesForUrl(targetUrl) {
    try {
        let allCookies = [];
        let seen = new Set();

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

const liveSupport = document.getElementById("liveSupport");
const liveFromStart = document.getElementById("liveFromStart");
const liveFromStartWrap = document.getElementById("liveFromStartWrap");
const liveFromStartWarning = document.getElementById("liveFromStartWarning");

async function recalculateModeState() {
    activeMode = document.getElementById("mode").value;
    activeGhostEngine = document.getElementById("ghostEngine").value;

    const ghostEngineWrap = document.getElementById("ghostEngineWrap");
    if (ghostEngineWrap) {
        ghostEngineWrap.style.display = activeMode === "ghost" ? "block" : "none";
    }

    const liveSupportWrap = document.getElementById("liveSupportWrap");
    const useNative = activeMode === "ghost" && activeGhostEngine === "chrome_native";
    const useBrowserFetch = activeMode === "ghost" && activeGhostEngine === "browser_fetch";
    const useDebugger = activeMode === "ghost" && activeGhostEngine === "debugger_capture";
    const useWebGL = activeMode === "ghost" && activeGhostEngine === "webgl_capture";

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
        qualitySelect.innerHTML = '<option value="">Best Quality (Default)</option>';
        if (liveSupportWrap) liveSupportWrap.style.display = "none";
        if (liveFromStartWrap) liveFromStartWrap.style.display = "none";
        return;
    }

    if (liveSupportWrap) liveSupportWrap.style.display = "flex";
    if (liveSupport && liveSupport.checked && liveFromStartWrap) {
        liveFromStartWrap.style.display = "flex";
    }

    // Refresh cookies if siphon or ghost
    globalCookieString = "";
    globalNetscapeCookies = "";
    if (activeMode === "siphon" || activeMode === "ghost") {
        const rawCookies = await getCookiesForUrl(url);
        if (rawCookies.length > 0) {
            globalCookieString = rawCookies.map(c => `${c.name}=${c.value}`).join("; ");
            globalNetscapeCookies = buildNetscape(rawCookies);
        }
    }

    statusDiv.innerText = "Checking available qualities...";
    qualitySelect.disabled = true;
    qualitySelect.innerHTML = '<option value="">Best Quality (Default)</option>';

    chrome.runtime.sendMessage({
        cmd: "GetFormats",
        ...{ 
            url: url, 
            cookies: globalCookieString || null,
            netscape_cookies: globalNetscapeCookies || null,
            userAgent: globalUserAgent,
            mode: activeMode,
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
            qualitySelect.innerHTML = '<option value="">Best Quality (Default)</option>';
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

async function init() {
    // Restore settings from local storage
    const stored = await new Promise(resolve => {
        chrome.storage.local.get(['downloadMode', 'activeGhostEngine'], (res) => resolve(res));
    });
    
    const modeVal = stored.downloadMode || 'siphon';
    const engineVal = stored.activeGhostEngine || 'ytdlp';

    document.getElementById("mode").value = modeVal;
    document.getElementById("ghostEngine").value = engineVal;

    document.getElementById("mode").addEventListener("change", recalculateModeState);
    document.getElementById("ghostEngine").addEventListener("change", recalculateModeState);

    globalCookieString = "";
    globalNetscapeCookies = "";

    // Capture initial session cookies
    const rawCookies = await getCookiesForUrl(url);
    if (rawCookies.length > 0) {
        globalCookieString = rawCookies.map(c => `${c.name}=${c.value}`).join("; ");
        globalNetscapeCookies = buildNetscape(rawCookies);
    }

    await recalculateModeState();
}

init();

if (liveSupport) {
    liveSupport.addEventListener("change", (e) => {
        if (liveFromStartWrap) {
            liveFromStartWrap.style.display = e.target.checked ? "flex" : "none";
        }
        if (!e.target.checked && liveFromStart) {
            liveFromStart.checked = false;
            if (liveFromStartWarning) liveFromStartWarning.style.display = "none";
        }
    });
}
if (liveFromStart) {
    liveFromStart.addEventListener("change", (e) => {
        if (liveFromStartWarning) {
            liveFromStartWarning.style.display = e.target.checked ? "block" : "none";
        }
    });
}

document.getElementById("cancelBtn").addEventListener("click", () => {
    window.close();
});

document.getElementById("downloadBtn").addEventListener("click", async () => {
    const folder = document.getElementById("folder").value;
    const format = qualitySelect.value;
    const writeSubs = document.getElementById("writeSubs").checked;
    const embedThumbnail = document.getElementById("embedThumbnail").checked;
    const embedChapters = document.getElementById("embedChapters").checked;
    const compressVideo = document.getElementById("compressVideo").checked;
    const downloadPlaylist = document.getElementById("downloadPlaylist") ? document.getElementById("downloadPlaylist").checked : false;

    // Read current mode and engine values selected by user in the popup
    const popupMode = document.getElementById("mode").value;
    const popupEngine = document.getElementById("ghostEngine").value;

    const useDebugger = popupMode === "ghost" && popupEngine === "debugger_capture";
    const useWebGL = popupMode === "ghost" && popupEngine === "webgl_capture";
    const useNative = popupMode === "ghost" && popupEngine === "chrome_native";
    const useBrowserFetch = popupMode === "ghost" && popupEngine === "browser_fetch";

    const reqEngine = useNative ? "chrome_native" : (popupMode === "ghost" ? popupEngine : null);

    // Assemble the complete download options package so ALL engines receive all settings
    const fullParams = {
        url: url,
        folder: folder,
        format: format || null,
        write_subs: writeSubs,
        embed_thumbnail: embedThumbnail,
        embed_chapters: embedChapters,
        compress_video: compressVideo,
        download_playlist: downloadPlaylist,
        live_support: liveSupport ? liveSupport.checked : false,
        live_from_start: liveFromStart ? liveFromStart.checked : false,
        cookies: globalCookieString || null,
        netscape_cookies: globalNetscapeCookies || null,
        userAgent: globalUserAgent,
        mode: popupMode,
        engine: reqEngine,
        referer: params.get("referer") || null
    };

    if (useWebGL) {
        statusDiv.innerText = "🎨 Starting WebGL Canvas Capture...";

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
                if (targetTab) targetTabId = targetTab.id;
            } catch(e) {}
        }

        if (!targetTabId) {
            alert("WebGL capture requires an active web tab. Cannot proceed.");
            return;
        }

        try {
            const regResp = await chrome.runtime.sendMessage({
                cmd: "AddDownload",
                ...fullParams
            });
            
            const daemonId = regResp.id || null;
            if (daemonId) {
                chrome.tabs.sendMessage(targetTabId, {
                    action: "start_webgl_capture",
                    daemonId: daemonId,
                    filename: "webgl_capture.mp4",
                    url: url
                });
                statusDiv.innerText = "✅ WebGL Canvas capture started.";
                setTimeout(() => window.close(), 1500);
            }
        } catch(e) {
            alert("Failed to start WebGL capture task: " + e.message);
        }
        return;
    }

    if (useDebugger) {
        statusDiv.innerText = "🪲 Starting Chrome Debugger Socket Capture...";
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
                if (targetTab) targetTabId = targetTab.id;
            } catch(e) {}
        }

        if (!targetTabId) {
            alert("Debugger Capture requires an active web tab.");
            return;
        }

        try {
            const regResp = await chrome.runtime.sendMessage({
                cmd: "AddDownload",
                ...fullParams
            });
            const daemonId = regResp.id || null;
            if (daemonId) {
                chrome.runtime.sendMessage({
                    action: "start_debugger_capture",
                    url: url,
                    tabId: targetTabId,
                    daemonId: daemonId,
                    folder: folder
                });
                statusDiv.innerText = "✅ Debugger capture started.";
                setTimeout(() => window.close(), 1500);
            }
        } catch(e) {
            alert("Failed to start Debugger Capture task: " + e.message);
        }
        return;
    }

    if (useBrowserFetch) {
        statusDiv.innerText = "🌐 Starting Tab Browser Fetch Stream...";
        let targetTabId = null;
        const queryTabId = params.get("tabId");
        if (queryTabId) {
            targetTabId = parseInt(queryTabId, 10);
        } else {
            try {
                const tabs = await chrome.tabs.query({});
                let targetTab = tabs.find(t => t.active && t.url && !t.url.startsWith("chrome-extension://"));
                if (targetTab) targetTabId = targetTab.id;
            } catch(e) {}
        }

        if (!targetTabId) {
            alert("Browser Fetch requires an active tab.");
            return;
        }

        try {
            const regResp = await chrome.runtime.sendMessage({
                cmd: "AddDownload",
                ...fullParams
            });
            const daemonId = regResp.id || null;
            if (daemonId) {
                chrome.runtime.sendMessage({
                    action: "start_browser_fetch",
                    url: url,
                    daemonId: daemonId,
                    folder: folder
                });
                statusDiv.innerText = "✅ Browser Fetch stream started.";
                setTimeout(() => window.close(), 1500);
            }
        } catch(e) {
            alert("Failed to start Browser Fetch: " + e.message);
        }
        return;
    }

    statusDiv.innerText = "Sending download task to JADMan daemon...";

    chrome.runtime.sendMessage({
        cmd: "AddDownload",
        ...fullParams
    }).then(response => {
        if (response && !response.error) {
            statusDiv.innerText = "Download successfully added to queue!";
            setTimeout(() => window.close(), 1000);
        } else {
            statusDiv.innerText = `Error: ${response ? response.error : 'No response'}`;
        }
    }).catch(err => {
        alert("Failed to connect to JADMan daemon. Is it running?");
        console.error("JADMan Error:", err);
    });
});
