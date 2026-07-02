// Safe chrome.runtime.sendMessage wrapper to prevent "No SW" / "Context Invalidated" warnings
if (typeof chrome !== 'undefined' && chrome.runtime) {
    const originalSendMessage = chrome.runtime.sendMessage;
    chrome.runtime.sendMessage = function(message, callback) {
        if (!chrome.runtime?.id) return;
        try {
            if (typeof callback === 'function') {
                return originalSendMessage.call(chrome.runtime, message, (response) => {
                    const err = chrome.runtime.lastError;
                    callback(response);
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

// JADMan Advanced Toolbox Controller

let activeTabId = null;
let activeTabUrl = null;
let selectedTabPane = "pane-investigator";
let inspectorActive = false;

// 1. Initialize view and populate active configuration
document.addEventListener('DOMContentLoaded', () => {
    // Sidebar Tabs navigation
    document.querySelectorAll('.nav-item').forEach(item => {
        item.addEventListener('click', (e) => {
            document.querySelectorAll('.nav-item').forEach(i => i.classList.remove('active'));
            document.querySelectorAll('.tab-pane').forEach(p => p.classList.remove('active'));
            
            const targetPane = e.target.dataset.tab;
            e.target.classList.add('active');
            const paneEl = document.getElementById(targetPane);
            if (paneEl) paneEl.classList.add('active');
            selectedTabPane = targetPane;
        });
    });

    // Load active settings from storage
    chrome.storage.local.get(['buttonPosition', 'downloadMode', 'activeGhostEngine', 'siphonRecordSegments', 'windowSpawnMode'], (res) => {
        if (res.buttonPosition) {
            document.getElementById('select-btn-pos').value = res.buttonPosition;
        }
        if (res.activeGhostEngine) {
            document.getElementById('select-ghost-engine').value = res.activeGhostEngine;
        }
        if (res.windowSpawnMode) {
            document.getElementById('select-window-mode').value = res.windowSpawnMode;
        }
        document.getElementById('toggle-segment-siphon').checked = !!res.siphonRecordSegments;
        
        // Mode mappings
        const mode = res.downloadMode || 'general';
        const isGhost = (mode === 'ghost');
        document.getElementById('toggle-ghost-mode').checked = isGhost;
        
        if (isGhost) {
            document.getElementById('ghost-engine-group').style.display = 'block';
        }
    });

    // Populate active tab information
    const params = new URLSearchParams(window.location.search);
    const queryTabId = params.get("tabId");
    if (queryTabId) {
        activeTabId = parseInt(queryTabId, 10);
        chrome.tabs.get(activeTabId, (tab) => {
            if (tab) {
                activeTabUrl = tab.url;
                refreshSniffedStreams();
                
                // Check if element inspector is currently active on this tab
                chrome.tabs.sendMessage(activeTabId, { action: "check_inspector_state" }, (response) => {
                    if (response && response.active) {
                        inspectorActive = true;
                        updateInspectorButtonUI();
                    }
                });

                // Poll for new sniffed streams periodically when Page Investigator is active
                setInterval(() => {
                    if (selectedTabPane === "pane-investigator") {
                        refreshSniffedStreams();
                    }
                }, 2000);
            }
        });
    }

    // Set listener for settings changes
    document.getElementById('select-btn-pos').addEventListener('change', (e) => {
        const position = e.target.value;
        chrome.storage.local.set({ buttonPosition: position });
        if (activeTabId) {
            chrome.tabs.sendMessage(activeTabId, { action: "update_position", position }).catch(() => {});
        }
    });

    document.getElementById('select-window-mode').addEventListener('change', (e) => {
        chrome.storage.local.set({ windowSpawnMode: e.target.value });
    });

    // Mode Toggle Switches
    document.getElementById('toggle-ghost-mode').addEventListener('change', (e) => {
        const checked = e.target.checked;
        document.getElementById('ghost-engine-group').style.display = checked ? 'block' : 'none';
        
        const mode = checked ? 'ghost' : 'general';
        chrome.storage.local.set({ downloadMode: mode });
    });

    document.getElementById('select-ghost-engine').addEventListener('change', (e) => {
        const engine = e.target.value;
        chrome.storage.local.set({ activeGhostEngine: engine });
    });

    document.getElementById('toggle-segment-siphon').addEventListener('change', (e) => {
        chrome.storage.local.set({ siphonRecordSegments: e.target.checked });
    });

    // Setup CDM Click Handlers
    document.getElementById('start-cdm-btn').addEventListener('click', () => {
        const consoleEl = document.getElementById('cdm-console');
        consoleEl.innerHTML = "[JADMan CDM] Triggering Widevine key dumper via daemon...";
        
        chrome.runtime.sendMessage({ action: "launch_cdm_extractor" }, (response) => {
            if (response && response.success) {
                consoleEl.innerHTML += "<br>[JADMan CDM] Hook deployed successfully. Spawning Frida listener...";
                document.getElementById('start-cdm-btn').style.display = 'none';
                document.getElementById('stop-cdm-btn').style.display = 'block';
                startCdmLogPoller();
            } else {
                consoleEl.innerHTML += `<br>[JADMan CDM ERROR] ${response?.error || 'Failed to communicate with daemon.'}`;
            }
        });
    });

    document.getElementById('stop-cdm-btn').addEventListener('click', () => {
        document.getElementById('start-cdm-btn').style.display = 'block';
        document.getElementById('stop-cdm-btn').style.display = 'none';
        document.getElementById('cdm-console').innerHTML += "<br>[JADMan CDM] Detached Frida session.";
        stopCdmLogPoller();
    });

    // Bulk buttons Action
    document.getElementById('grab-media-btn').addEventListener('click', () => {
        if (activeTabId) {
            chrome.runtime.sendMessage({ action: "grab_media_for_tab", tabId: activeTabId });
        }
    });

    document.getElementById('grab-assets-btn').addEventListener('click', () => {
        if (activeTabId) {
            chrome.runtime.sendMessage({ action: "grab_assets_for_tab", tabId: activeTabId });
        }
    });

    // Custom URL download
    document.getElementById('custom-dl-btn').addEventListener('click', () => {
        const url = document.getElementById('custom-dl-url').value.trim();
        if (url) {
            chrome.windows.create({
                url: `popup.html?url=${encodeURIComponent(url)}`,
                type: "popup", width: 450, height: 380, focused: true
            });
        }
    });

    // Preload host buffers
    document.getElementById('preload-stream-btn').addEventListener('click', () => {
        if (activeTabId) {
            chrome.tabs.sendMessage(activeTabId, { action: "force_preload_streams" }, (response) => {
                alert("Triggered aggressive stream preload loop. Check siphoned segments.");
            });
        }
    });

    // Element inspector toggle
    document.getElementById('toggle-inspector-btn').addEventListener('click', () => {
        if (!activeTabId) return;
        inspectorActive = !inspectorActive;
        updateInspectorButtonUI();
        chrome.tabs.sendMessage(activeTabId, { action: "toggle_element_inspector", active: inspectorActive });
    });

    // Handle investigator refresh button if any
    document.getElementById('refresh-streams-btn')?.addEventListener('click', refreshSniffedStreams);
});

function updateInspectorButtonUI() {
    const btn = document.getElementById('toggle-inspector-btn');
    if (inspectorActive) {
        btn.innerText = "🛑 Stop Inspector Mode";
        btn.style.background = "#ff3366";
        btn.style.color = "#fff";
    } else {
        btn.innerText = "🔍 Toggle Inspector Mode";
        btn.style.background = "rgba(255, 255, 255, 0.05)";
        btn.style.color = "var(--text-white)";
    }
}

// 2. Fetch and display discovered streams for active tab
function refreshSniffedStreams() {
    if (!activeTabId) return;
    
    chrome.runtime.sendMessage({ action: "get_tab_discovery", tabId: activeTabId }, (response) => {
        const countEl = document.getElementById('stream-count');
        const container = document.getElementById('streams-container');
        const emptyEl = document.getElementById('streams-empty');
        
        if (!response || (!response.urls?.length && !response.siphoned?.length)) {
            countEl.innerText = "0";
            container.style.display = "none";
            emptyEl.style.display = "flex";
            return;
        }
        
        const allItems = [];
        if (response.urls) {
            response.urls.forEach(obj => {
                if (obj && obj.url) {
                    allItems.push({ url: obj.url, type: 'Network URL', label: 'NET' });
                }
            });
        }
        if (response.siphoned) {
            response.siphoned.forEach(obj => {
                if (obj && obj.url) {
                    allItems.push({ url: obj.url, type: obj.mime || 'Segment Stream', label: 'SIPHON' });
                }
            });
        }
        
        countEl.innerText = allItems.length;
        container.innerHTML = "";
        
        allItems.forEach(item => {
            const el = document.createElement('div');
            el.className = "stream-item";
            
            const name = item.url.split('/').pop().split('?')[0] || item.url;
            el.innerHTML = `
                <div class="stream-info">
                    <div class="stream-title" title="${item.url}">${name}</div>
                    <div class="stream-meta">
                        <span style="color: ${item.label === 'SIPHON' ? '#00c8ff' : '#00ff66'}">[${item.label}]</span>
                        <span>Type: ${item.type}</span>
                    </div>
                </div>
                <button class="btn-primary" style="padding: 6px 12px; font-size: 11px;" data-url="${encodeURIComponent(item.url)}">Download</button>
            `;
            
            el.querySelector('button').addEventListener('click', (e) => {
                const targetUrl = decodeURIComponent(e.target.dataset.url);
                chrome.runtime.sendMessage({ action: "request_download", url: targetUrl });
            });
            
            container.appendChild(el);
        });
        
        container.style.display = "block";
        emptyEl.style.display = "none";
    });
}

// 3. Monitor CDM keys dump file
let cdmPollInterval = null;
function startCdmLogPoller() {
    stopCdmLogPoller();
    cdmPollInterval = setInterval(() => {
        // Query background service worker to fetch extracted keys from daemon
        chrome.runtime.sendMessage({ action: "read_cdm_keys_log" }, (response) => {
            const consoleEl = document.getElementById('cdm-console');
            if (response && response.content) {
                const lines = response.content.split('\n').filter(l => l.trim().length > 0);
                let html = "[JADMan CDM Console] Active Listening Logs:<br>";
                lines.forEach(l => {
                    html += `<div style="margin-top: 4px; border-bottom: 1px solid rgba(255,255,255,0.05); padding-bottom: 4px;">🔑 Key Found: <span style="color: #00c8ff; font-weight: bold;">${l}</span></div>`;
                });
                consoleEl.innerHTML = html;
                consoleEl.scrollTop = consoleEl.scrollHeight;
            }
        });
    }, 2000);
}

function stopCdmLogPoller() {
    if (cdmPollInterval) {
        clearInterval(cdmPollInterval);
        cdmPollInterval = null;
    }
}
