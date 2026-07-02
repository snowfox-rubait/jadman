// JADMan Universal Heuristic Engine (IDM-Grade Detection v3)

const IDM_EXTENSIONS = [
    "3gp", "7z", "aac", "ace", "aif", "apk", "arj", "asf", "avi", "bin", "bz2", "exe", "gz", "gzip", 
    "img", "iso", "lzh", "m4a", "m4v", "mkv", "mov", "mp3", "mp4", "mpa", "mpe", "mpeg", "mpg", 
    "msi", "msu", "ogg", "ogv", "pdf", "plj", "pps", "ppt", "qt", "ra", "rar", "rm", "rmvb", 
    "sea", "sit", "sitx", "tar", "tif", "tiff", "wav", "wma", "wmv", "z", "zip",
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "ico", "doc", "docx", "xls", "xlsx", "txt", "csv", "rtf"
];

const DOWNLOAD_MIME_REGISTRY = [
    "video/", "audio/", "application/x-mpegURL", "application/dash+xml",
    "application/zip", "application/x-rar-compressed", "application/x-7z-compressed", "application/x-tar",
    "application/x-msdos-program", "application/x-executable", "application/vnd.android.package-archive",
    "application/octet-stream", "application/force-download", "application/download"
];

const IGNORED_MIME_TYPES = ["text/html", "text/css", "application/javascript", "application/json", "image/png", "image/jpeg", "image/gif"];

const DOMAIN_BLACKLIST = [
    "update.microsoft.com",
    "download.windowsupdate.com",
    "siteseal.thawte.com",
    "ecom.cimetz.com",
    "voice2page.com"
];

let tabDiscoveryMap = {};
let siphonedData = {}; // tabId -> Array of {url, mime, size, priority}
let whitelistedUrls = new Set();
let isPaused = false;

// Initialize and listen to pause/resume states
chrome.storage.local.get(['isPaused'], (res) => {
    isPaused = !!(res && res.isPaused);
});
chrome.storage.onChanged.addListener((changes, area) => {
    if (area === 'local' && changes.isPaused) {
        isPaused = !!changes.isPaused.newValue;
        console.log(`[JADMan] Interceptor paused status changed to: ${isPaused}`);
        if (isPaused) {
            chrome.tabs.query({}, (tabs) => {
                if (tabs) {
                    tabs.forEach(t => {
                        chrome.action.setBadgeText({ text: "", tabId: t.id }).catch(() => {});
                    });
                }
            });
        }
    }
});

const STARTUP_TIME = Date.now();
let isStartupGuardActive = true;

// Disable guard after 5 seconds
setTimeout(() => { isStartupGuardActive = false; }, 5000);

function isBlacklisted(url) {
    try {
        const hostname = new URL(url).hostname.toLowerCase();
        return DOMAIN_BLACKLIST.some(domain => hostname === domain || hostname.endsWith("." + domain));
    } catch(e) { return false; }
}

let nativePort = null;
let pendingNativeRequests = [];
let reconnectDelay = 2000;

function connectNative() {
    nativePort = chrome.runtime.connectNative('com.jadm.jadm');
    nativePort.onMessage.addListener((msg) => {
        console.log("[JADMan Native] Received:", msg);
        reconnectDelay = 2000; // Reset delay on successful message exchange
        if (pendingNativeRequests.length > 0) {
            const resolver = pendingNativeRequests.shift();
            resolver(msg);
        }
    });
    nativePort.onDisconnect.addListener(() => {
        const err = chrome.runtime.lastError ? chrome.runtime.lastError.message : "No error message";
        console.log(`[JADMan Native] Disconnected. Error: ${err}. Reconnecting in ${reconnectDelay / 1000}s...`);
        nativePort = null;
        setTimeout(connectNative, reconnectDelay);
        reconnectDelay = Math.min(reconnectDelay * 2, 30000); // Exponential backoff capped at 30s
    });
}
connectNative();

function sendNativeMessage(msg) {
    if (!nativePort) {
        console.log("[JADMan Native] Port disconnected. Attempting immediate reconnection...");
        reconnectDelay = 2000;
        connectNative();
    }
    if (nativePort) {
        nativePort.postMessage(msg);
    } else {
        console.warn("[JADMan Native] Port disconnected. Dropping message.");
    }
}

async function triggerFloat() {
    sendNativeMessage({ cmd: "Float" });
}

function spawnJadmPage(url, width = 800, height = 600) {
    chrome.storage.local.get(['windowSpawnMode'], (res) => {
        const mode = res.windowSpawnMode || 'popup';
        if (mode === 'tab') {
            chrome.tabs.create({ url: url, active: true });
        } else if (mode === 'normal') {
            chrome.windows.create({ url: url, type: "normal", focused: true });
        } else {
            chrome.windows.create({ url: url, type: "popup", width: width, height: height, focused: true });
        }
    });
}

async function openJadmPopup(url, filename = null, mime = null) {
    if (isBlacklisted(url)) return;
    await triggerFloat();
    
    let tabIdParam = "";
    try {
        const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
        if (tabs && tabs.length > 0 && tabs[0].id) {
            tabIdParam = `&tabId=${tabs[0].id}`;
        }
    } catch(e) {
        console.error("Error querying active tab for popup:", e);
    }

    const targetUrl = `popup.html?url=${encodeURIComponent(url)}${filename ? '&filename=' + encodeURIComponent(filename) : ''}${mime ? '&mime=' + encodeURIComponent(mime) : ''}${tabIdParam}`;
    spawnJadmPage(targetUrl, 450, 380);
}

// 1. UNIVERSAL NETWORK SNIFFER
chrome.webRequest.onHeadersReceived.addListener(
    async (details) => {
        if (isPaused) return;
        if (details.method !== "GET" || details.tabId < 0) return;
        if (isBlacklisted(details.url)) return;
        
        const lowerUrl = details.url.toLowerCase();
        if (lowerUrl.includes(".m4s") || lowerUrl.includes(".ts") || lowerUrl.includes(".dash")) return;

        const headers = details.responseHeaders.reduce((acc, h) => {
            acc[h.name.toLowerCase()] = h.value.toLowerCase();
            return acc;
        }, {});

        const contentType = headers['content-type'] || "";
        const contentDisp = headers['content-disposition'] || "";
        const contentLength = parseInt(headers['content-length'] || "0", 10);

        let isDownload = false;
        if (contentDisp.includes("attachment")) isDownload = true;
        if (DOWNLOAD_MIME_REGISTRY.some(type => contentType.startsWith(type)) && !IGNORED_MIME_TYPES.includes(contentType)) isDownload = true;
        if (contentLength > 5 * 1024 * 1024 && !IGNORED_MIME_TYPES.some(t => contentType.includes(t))) isDownload = true;

        if (isDownload) {
            if (!tabDiscoveryMap[details.tabId]) tabDiscoveryMap[details.tabId] = new Set();
            if (!tabDiscoveryMap[details.tabId].has(details.url)) {
                tabDiscoveryMap[details.tabId].add(details.url);
                
                const list = Array.from(tabDiscoveryMap[details.tabId]).map(u => ({
                    url: u,
                    text: u.split('/').pop().split('?')[0] || "Detected Stream"
                }));

                chrome.storage.local.set({ grabbedLinks: list });

                chrome.action.setBadgeBackgroundColor({ color: '#ff0000', tabId: details.tabId });
                chrome.action.setBadgeText({ text: (list.length + (siphonedData[details.tabId]?.filter(i => i.priority === 'MANIFEST').length || 0)).toString(), tabId: details.tabId });

                chrome.tabs.sendMessage(details.tabId, { action: "media_discovered", url: details.url }).catch(() => {});
            }
        }
    },
    { urls: ["<all_urls>"] },
    ["responseHeaders"]
);

// 2. DOWNLOAD INTERCEPTOR
chrome.downloads.onCreated.addListener((downloadItem) => {
    if (isPaused) {
        console.log("[JADMan] Interceptor paused, letting browser handle download.");
        return;
    }
    if (isStartupGuardActive) return;
    const downloadStartTime = new Date(downloadItem.startTime).getTime();
    if (downloadStartTime < STARTUP_TIME) return;

    // Do not intercept downloads that were explicitly whitelisted by the extension
    if (whitelistedUrls.has(downloadItem.url)) {
        console.log(`[JADMan] Skipping intercept of whitelisted download: ${downloadItem.url}`);
        whitelistedUrls.delete(downloadItem.url);
        return;
    }

    // Do not intercept downloads initiated by our own extension
    if (downloadItem.byExtensionId === chrome.runtime.id) {
        console.log(`[JADMan] Skipping intercept of download initiated by JADMan: ${downloadItem.url}`);
        return;
    }

    if (downloadItem.url.startsWith("blob:")) return;
    if (isBlacklisted(downloadItem.url)) return;

    const mime = (downloadItem.mime || "").toLowerCase();
    const filename = (downloadItem.filename || "").toLowerCase();
    const isHtmlOrSystem = mime === "text/html" || filename.endsWith(".htm") || filename.endsWith(".html") || filename.endsWith(".crx");

    if (!isHtmlOrSystem) {
        chrome.downloads.cancel(downloadItem.id);
        openJadmPopup(downloadItem.url, downloadItem.filename, mime);
    }
});

// 3. MESSAGE HUB
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.cmd) {
        pendingNativeRequests.push(sendResponse);
        sendNativeMessage(message);
        return true;
    }
    if (message.action === "start_browser_fetch") {
        const { url, daemonId, folder } = message;
        console.log(`[JADMan BrowserFetch] Forwarding request to active tab... URL: ${url}, ID: ${daemonId}`);
        
        chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
            if (tabs && tabs[0]) {
                console.log(`[JADMan BrowserFetch] Sending message to tab ${tabs[0].id}`);
                chrome.tabs.sendMessage(tabs[0].id, {
                    action: "start_browser_fetch",
                    url: url,
                    daemonId: daemonId,
                    folder: folder
                }).catch((e) => {
                    console.error("[JADMan BrowserFetch] Failed to send message to tab content script:", e);
                    sendNativeMessage({ cmd: "StopDownload", id: daemonId });
                });
            } else {
                console.error("[JADMan BrowserFetch] No active tab found to run fetch in.");
                sendNativeMessage({ cmd: "StopDownload", id: daemonId });
            }
        });

        sendResponse({ status: "started" });
        return true;
    } else if (message.action === "start_debugger_capture") {
        const { url, tabId, daemonId, folder } = message;
        console.log(`[JADMan Debugger] Attaching debugger to tab ${tabId} for URL: ${url}`);

        const targetUrl = url;
        const debTarget = { tabId: tabId };
        const protocolVersion = "1.3";

        chrome.debugger.attach(debTarget, protocolVersion, () => {
            if (chrome.runtime.lastError) {
                console.error(`[JADMan Debugger] Attach failed:`, chrome.runtime.lastError.message);
                return;
            }

            console.log(`[JADMan Debugger] Attached to tab ${tabId}`);
            chrome.debugger.sendCommand(debTarget, "Network.enable", {}, () => {
                // Request page reload to force request to trigger through socket
                chrome.tabs.reload(tabId, { bypassCache: true });
            });
        });

        // Store finishListener reference so it can be unhooked correctly
        let registeredFinishListener = null;

        const onEvent = async (source, method, params) => {
            if (source.tabId !== tabId) return;

            if (method === "Network.responseReceived") {
                const { response, requestId } = params;
                const matchesTarget = response.url === targetUrl || 
                                      response.url.split('?')[0] === targetUrl.split('?')[0] || 
                                      targetUrl.includes(response.url.split('?')[0]);
                if (matchesTarget) {
                    console.log(`[JADMan Debugger] Target response received: ${response.url} (Request ID: ${requestId})`);

                    registeredFinishListener = async (fSource, fMethod, fParams) => {
                        if (fSource.tabId !== tabId) return;
                        if (fMethod === "Network.loadingFinished" && fParams.requestId === requestId) {
                            if (registeredFinishListener) {
                                chrome.debugger.onEvent.removeListener(registeredFinishListener);
                            }
                            console.log(`[JADMan Debugger] Request finished loading. Grabbing body...`);
                            
                            chrome.debugger.sendCommand(debTarget, "Network.getResponseBody", { requestId }, async (bodyResult) => {
                                if (chrome.runtime.lastError) {
                                    console.error(`[JADMan Debugger] Failed to get response body:`, chrome.runtime.lastError.message);
                                    chrome.debugger.detach(debTarget).catch(() => {});
                                    chrome.debugger.onEvent.removeListener(onEvent);
                                    return;
                                }

                                try {
                                    const { body, base64Encoded } = bodyResult;
                                    let binaryData;
                                    if (base64Encoded) {
                                        const binaryString = atob(body);
                                        const bytes = new Uint8Array(binaryString.length);
                                        for (let i = 0; i < binaryString.length; i++) {
                                            bytes[i] = binaryString.charCodeAt(i);
                                        }
                                        binaryData = bytes;
                                    } else {
                                        binaryData = new TextEncoder().encode(body);
                                    }

                                    const filename = targetUrl.split('/').pop().split('?')[0] || 'debugger_download';
                                    console.log(`[JADMan Debugger] Streaming captured response body of size ${binaryData.length} to daemon...`);

                                    // Upload body as single chunk to daemon
                                    sendNativeMessage({
                                        cmd: "SiphonChunk",
                                        daemon_id: daemonId,
                                        chunk_index: 0,
                                        is_last: true,
                                        filename: filename,
                                        total_size: binaryData.length,
                                        data: Array.from(binaryData)
                                    });

                                    console.log(`[JADMan Debugger] Success streaming file ${filename}`);
                                } catch(e) {
                                    console.error(`[JADMan Debugger] Streaming error:`, e);
                                    sendNativeMessage({ cmd: "StopDownload", id: daemonId });
                                } finally {
                                    chrome.debugger.detach(debTarget).catch(() => {});
                                    chrome.debugger.onEvent.removeListener(onEvent);
                                }
                            });
                        }
                    };
                    chrome.debugger.onEvent.addListener(registeredFinishListener);
                }
            }
        };

        chrome.debugger.onEvent.addListener(onEvent);

        sendResponse({ status: "started" });
        return true;
    } else if (message.action === "whitelist_url") {
        whitelistedUrls.add(message.url);
        console.log(`[JADMan] Whitelisting URL: ${message.url}`);
        // Remove from whitelist after 15 seconds to avoid memory leak
        setTimeout(() => whitelistedUrls.delete(message.url), 15000);
        sendResponse({ status: "ok" });
        return true;
    } else if (message.action === "request_download") {
        openJadmPopup(message.url, null, message.mime);
    } else if (message.action === "monitor_native_download") {
        // === CHROME NATIVE DOWNLOAD MONITOR ===
        // Tracks a chrome.downloads download and reports progress/completion to daemon.
        const { downloadId, daemonId, targetFolder, originalUrl } = message;
        console.log(`[JADMan Native] Monitoring Chrome download ID: ${downloadId}`);

        const onChanged = (delta) => {
            if (delta.id !== downloadId) return;

            if (delta.state) {
                if (delta.state.current === "complete") {
                    chrome.downloads.search({ id: downloadId }, (items) => {
                        if (!items || items.length === 0) return;
                        const item = items[0];
                        const chromePath = item.filename; // Full path where Chrome saved it

                        console.log(`[JADMan Native] Download complete at: ${chromePath}`);

                        // Tell daemon to move the file to the user's target folder
                        if (targetFolder && chromePath) {
                            sendNativeMessage({
                                cmd: "MoveFile",
                                source: chromePath,
                                destination: targetFolder,
                                daemon_id: daemonId || null
                            });
                        }
                    });
                    chrome.downloads.onChanged.removeListener(onChanged);

                } else if (delta.state.current === "interrupted") {
                    console.warn("[JADMan Native] Chrome download interrupted:", delta.error?.current);
                    chrome.downloads.onChanged.removeListener(onChanged);
                }
            }
        };
        chrome.downloads.onChanged.addListener(onChanged);
        sendResponse({ status: "monitoring" });

    } else if (message.action === "siphoned_data") {
        const tid = sender.tab.id;
        if (!siphonedData[tid]) siphonedData[tid] = [];
        
        if (!siphonedData[tid].some(i => i.url === message.url)) {
            const item = {
                url: message.url,
                mime: message.mime,
                size: message.size,
                priority: message.priority || 'NORMAL'
            };
            
            if (item.priority === 'MANIFEST') siphonedData[tid].unshift(item);
            else siphonedData[tid].push(item);
            
            if (siphonedData[tid].length > 100) siphonedData[tid].pop();
        }

        const discCount = (tabDiscoveryMap[tid]?.size || 0);
        const manifestCount = siphonedData[tid].filter(i => i.priority === 'MANIFEST').length;
        const totalCount = discCount + manifestCount;

        if (totalCount > 0) {
            chrome.action.setBadgeText({ text: totalCount.toString(), tabId: tid });
            chrome.action.setBadgeBackgroundColor({ color: manifestCount > 0 ? '#00ff00' : '#ff0000', tabId: tid });
        }

    } else if (message.action === "get_netscape_cookies") {
        const url = message.url;
        try {
            const domain = new URL(url).hostname;
            const retrieveCookies = (cookies) => {
                let netscape = "# Netscape HTTP Cookie File\n";
                cookies.forEach(c => {
                    const domain = c.domain.startsWith('.') ? c.domain : '.' + c.domain;
                    const path = c.path;
                    const secure = c.secure ? "TRUE" : "FALSE";
                    const expires = c.expirationDate ? Math.floor(c.expirationDate) : 0;
                    netscape += `${domain}\tTRUE\t${path}\t${secure}\t${expires}\t${c.name}\t${c.value}\n`;
                });
                sendResponse(netscape);
            };

            try {
                chrome.cookies.getAll({ domain, partitionKey: {} }, retrieveCookies);
            } catch(e) {
                chrome.cookies.getAll({ domain }, retrieveCookies);
            }
        } catch(e) { sendResponse(null); }
        return true; 
    } else if (message.action === "open_grabber") {
        triggerFloat();
        spawnJadmPage(`grabber.html?tabId=${sender.tab.id}`, 800, 600);
    } else if (message.action === "open_grabber_for_tab") {
        triggerFloat();
        const tid = message.tabId;
        const list = Array.from(tabDiscoveryMap[tid] || []).map(u => ({
            url: u,
            text: u.split('/').pop().split('?')[0] || "Detected Stream"
        }));
        const siphoned = (siphonedData[tid] || []).map(s => ({
            url: s.url,
            text: `[Siphoned] ${s.url.split('/').pop().split('?')[0] || "Deep Capture"}`,
            isSiphoned: true,
            mime: s.mime,
            size: s.size,
            priority: s.priority
        }));
        const combined = [...list, ...siphoned];
        chrome.storage.local.set({ grabbedLinks: combined }, () => {
            spawnJadmPage(`grabber.html?tabId=${tid}`, 800, 600);
        });
    } else if (message.action === "get_tab_discovery") {
        const tid = message.tabId > 0 ? message.tabId : sender.tab?.id;
        const list = Array.from(tabDiscoveryMap[tid] || []).map(u => ({
            url: u,
            text: u.split('/').pop().split('?')[0] || "Detected Stream"
        }));
        const siphoned = (siphonedData[tid] || []).map(s => ({
            url: s.url,
            text: `[Siphoned] ${s.url.split('/').pop().split('?')[0] || "Deep Capture"}`,
            isSiphoned: true,
            mime: s.mime,
            size: s.size,
            priority: s.priority
        }));
        sendResponse({ urls: list, siphoned: siphoned });
    } else if (message.action === "launch_cdm_extractor") {
        sendNativeMessage({ CdmStart: { url: "", license_url: "", headers: null } });
        sendResponse({ success: true });
    } else if (message.action === "read_cdm_keys_log") {
        pendingNativeRequests.push((msg) => {
            sendResponse({ content: msg?.folder || "" });
        });
        sendNativeMessage({ CdmGetKeys: null });
    } else if (message.action === "grab_media_for_tab") {
        const tid = message.tabId;
        chrome.scripting.executeScript({
            target: { tabId: tid },
            func: () => {
                const links = [];
                document.querySelectorAll("video, audio, source").forEach(m => {
                    const src = m.src || m.currentSrc;
                    if (src && !src.startsWith("blob:")) links.push({url: src, text: "Media Stream"});
                });
                document.querySelectorAll("a").forEach(a => {
                    if (!a.href) return;
                    const lower = a.href.toLowerCase();
                    const isMedia = [".mp4", ".mkv", ".mp3", ".pdf", ".zip", ".rar", ".7z", ".exe"].some(ext => lower.includes(ext));
                    if (isMedia) links.push({ url: a.href, text: a.innerText.trim() || a.href.split('/').pop() });
                });
                return links;
            }
        }, (results) => {
            const dataList = results?.[0]?.result || [];
            chrome.storage.local.set({ grabbedLinks: dataList }, () => {
                triggerFloat();
                spawnJadmPage(`grabber.html?tabId=${tid}`, 800, 600);
            });
        });
    } else if (message.action === "grab_assets_for_tab") {
        const tid = message.tabId;
        chrome.scripting.executeScript({
            target: { tabId: tid },
            func: () => {
                const links = [];
                document.querySelectorAll("img").forEach(i => { if (i.src && !i.src.startsWith("data:")) links.push({url: i.src, text: "Image Asset"}); });
                document.querySelectorAll("link[rel='stylesheet']").forEach(l => { if (l.href) links.push({url: l.href, text: "CSS Stylesheet"}); });
                document.querySelectorAll("script").forEach(s => { if (s.src) links.push({url: s.src, text: "JS Script"}); });
                return links;
            }
        }, (results) => {
            const dataList = results?.[0]?.result || [];
            chrome.storage.local.set({ grabbedLinks: dataList }, () => {
                triggerFloat();
                spawnJadmPage(`grabber.html?tabId=${tid}`, 800, 600);
            });
        });
    }
    return true;
});

// 4. CONTEXT MENU (JADMan Toolbox)
function setupContextMenus() {
    chrome.contextMenus.removeAll(() => {
        chrome.contextMenus.create({ id: "jadman-toolbox", title: "📥 JADMan Toolbox", contexts: ["all"] });
        chrome.contextMenus.create({ id: "jadman-open-toolbox", title: "🛠️ Open Advanced Toolbox", parentId: "jadman-toolbox", contexts: ["all"] });
        chrome.contextMenus.create({ id: "jadman-download-target", title: "Download this Media/Link", parentId: "jadman-toolbox", contexts: ["link", "image", "video", "audio"] });
        chrome.contextMenus.create({ id: "jadman-download-selection", title: "Download Selected Links", parentId: "jadman-toolbox", contexts: ["selection"] });
        chrome.contextMenus.create({ id: "jadman-grab-media", title: "Grab all Page Media (Videos/Audio/Docs)", parentId: "jadman-toolbox", contexts: ["all"] });
        chrome.contextMenus.create({ id: "jadman-grab-assets", title: "Grab all Website Assets (Images/CSS/JS)", parentId: "jadman-toolbox", contexts: ["all"] });
        chrome.contextMenus.create({ id: "jadman-position-menu", title: "⚙️ Button Position", parentId: "jadman-toolbox", contexts: ["all"] });
        ["top-right", "top-left", "bottom-right", "bottom-left", "center"].forEach(pos => {
            chrome.contextMenus.create({ id: `jadman-pos-${pos}`, title: pos.replace('-', ' '), parentId: "jadman-position-menu", contexts: ["all"] });
        });
    });
}

chrome.runtime.onInstalled.addListener(() => {
    setupContextMenus();
});

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
    if (info.menuItemId === "jadman-open-toolbox") {
        spawnJadmPage(`toolbox.html?tabId=${tab.id}`, 850, 620);
        return;
    }

    if (info.menuItemId.startsWith("jadman-pos-")) {
        const position = info.menuItemId.replace("jadman-pos-", "");
        chrome.storage.local.set({ buttonPosition: position });
        chrome.tabs.sendMessage(tab.id, { action: "update_position", position: position }).catch(() => {});
        return;
    }

    const triggerGrabber = (dataList) => {
        if (dataList.length > 0) {
            chrome.storage.local.set({ grabbedLinks: dataList }, () => {
                triggerFloat();
                spawnJadmPage(`grabber.html?tabId=${tab.id}`, 800, 600);
            });
        }
    };

    if (info.menuItemId === "jadman-download-target") {
        const url = info.linkUrl || info.srcUrl || info.pageUrl;
        const mime = info.mediaType === "image" ? "image/jpeg" : (info.mediaType === "video" ? "video/mp4" : null);
        openJadmPopup(url, null, mime);
    } else if (info.menuItemId === "jadman-download-selection") {
        chrome.scripting.executeScript({
            target: { tabId: tab.id },
            func: () => {
                const selection = window.getSelection();
                const links = [];
                document.querySelectorAll("a").forEach(a => {
                    if (selection.containsNode(a, true) && a.href && !a.href.includes(window.location.href + "#")) {
                        links.push({ url: a.href, text: a.innerText.trim() || a.href.split('/').pop() });
                    }
                });
                return links;
            }
        }, (results) => triggerGrabber(results?.[0]?.result || []));
    } else if (info.menuItemId === "jadman-grab-media") {
        chrome.scripting.executeScript({
            target: { tabId: tab.id },
            func: () => {
                const links = [];
                document.querySelectorAll("video, audio, source").forEach(m => {
                    const src = m.src || m.currentSrc;
                    if (src && !src.startsWith("blob:")) links.push({url: src, text: "Media Stream"});
                });
                document.querySelectorAll("a").forEach(a => {
                    if (!a.href) return;
                    const lower = a.href.toLowerCase();
                    const isMedia = [".mp4", ".mkv", ".mp3", ".pdf", ".zip", ".rar", ".7z", ".exe"].some(ext => lower.includes(ext));
                    if (isMedia) links.push({ url: a.href, text: a.innerText.trim() || a.href.split('/').pop() });
                });
                return links;
            }
        }, (results) => triggerGrabber(results?.[0]?.result || []));
    } else if (info.menuItemId === "jadman-grab-assets") {
        chrome.scripting.executeScript({
            target: { tabId: tab.id },
            func: () => {
                const links = [];
                document.querySelectorAll("img").forEach(i => { if (i.src && !i.src.startsWith("data:")) links.push({url: i.src, text: "Image Asset"}); });
                document.querySelectorAll("link[rel='stylesheet']").forEach(l => { if (l.href) links.push({url: l.href, text: "CSS Stylesheet"}); });
                document.querySelectorAll("script").forEach(s => { if (s.src) links.push({url: s.src, text: "JS Script"}); });
                return links;
            }
        }, (results) => triggerGrabber(results?.[0]?.result || []));
    }
});

// 5. UNBLOCKABLE TOOLBAR ACTION
chrome.action.onClicked.addListener((tab) => {
    const list = Array.from(tabDiscoveryMap[tab.id] || []).map(u => ({
        url: u,
        text: u.split('/').pop().split('?')[0] || "Detected Stream"
    }));
    
    if (list.length > 0) {
        chrome.storage.local.set({ grabbedLinks: list }, () => {
            triggerFloat();
            spawnJadmPage(`grabber.html?tabId=${tab.id}`, 800, 600);
        });
    } else {
        if (!tab.url || tab.url.startsWith("chrome://") || tab.url.startsWith("edge://") || tab.url.startsWith("about:")) {
            chrome.storage.local.set({ grabbedLinks: [] }, () => {
                triggerFloat();
                spawnJadmPage(`grabber.html?tabId=${tab.id}`, 800, 600);
            });
            return;
        }

        chrome.scripting.executeScript({
            target: { tabId: tab.id },
            func: () => {
                const links = [];
                document.querySelectorAll("a").forEach(a => {
                    if (a.href && !a.href.includes(window.location.href + "#")) {
                        links.push({ url: a.href, text: a.innerText.trim() || a.href.split('/').pop() });
                    }
                });
                return links;
            }
        }, (results) => {
            if (chrome.runtime.lastError) return;
            if (results?.[0]?.result?.length > 0) {
                chrome.storage.local.set({ grabbedLinks: results[0].result }, () => {
                    triggerFloat();
                    spawnJadmPage(`grabber.html?tabId=${tab.id}`, 800, 600);
                });
            } else {
                chrome.storage.local.set({ grabbedLinks: [] }, () => {
                    spawnJadmPage(`grabber.html?tabId=${tab.id}`, 800, 600);
                });
            }
        });
    }
});

// Reset logic
chrome.tabs.onRemoved.addListener(tid => {
    delete tabDiscoveryMap[tid];
    delete siphonedData[tid];
});
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
    if (changeInfo.status === 'loading') {
        delete tabDiscoveryMap[tabId];
        delete siphonedData[tabId];
        chrome.action.setBadgeText({ text: "", tabId: tabId });
    }
});
