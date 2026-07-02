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

let allLinks = [];
const DAEMON_URL = "http://127.0.0.1:6246";

// Parse tabId from URL
const urlParams = new URLSearchParams(window.location.search);
const targetTabId = parseInt(urlParams.get('tabId') || "-1");

// 1. Get detected and siphoned links from background
async function loadData() {
    chrome.runtime.sendMessage({ action: "get_tab_discovery", tabId: targetTabId }, (response) => {
        const detected = response.urls || [];
        const siphoned = response.siphoned || [];
        
        // Merge them, prioritizing siphoned if URLs match
        const seen = new Set();
        allLinks = [];

        siphoned.forEach(s => {
            seen.add(s.url);
            allLinks.push(s);
        });

        detected.forEach(d => {
            if (!seen.has(d.url)) {
                seen.add(d.url);
                allLinks.push(d);
            }
        });

        // Also add anything from grabbedLinks storage (manual selection)
        chrome.storage.local.get(['grabbedLinks'], (res) => {
            const manual = res.grabbedLinks || [];
            manual.forEach(m => {
                if (!seen.has(m.url)) {
                    seen.add(m.url);
                    allLinks.push(m);
                }
            });
            renderLinks();
        });
    });
}

loadData();

const liveSupport = document.getElementById("liveSupport");
const liveFromStart = document.getElementById("liveFromStart");
const liveFromStartWrap = document.getElementById("liveFromStartWrap");
const liveFromStartWarning = document.getElementById("liveFromStartWarning");
const modeSelect = document.getElementById("mode");
const ghostEngineSelect = document.getElementById("ghostEngine");
const ghostEngineWrap = document.getElementById("ghostEngineWrap");

function updateUI() {
    const activeMode = modeSelect.value;
    ghostEngineWrap.style.display = activeMode === "ghost" ? "flex" : "none";
}

modeSelect.addEventListener("change", updateUI);
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
            liveFromStartWarning.style.display = e.target.checked ? "inline" : "none";
        }
    });
}

chrome.storage.local.get(['downloadMode', 'activeGhostEngine'], (res) => {
    if (res.downloadMode) modeSelect.value = res.downloadMode;
    if (res.activeGhostEngine) ghostEngineSelect.value = res.activeGhostEngine;
    updateUI();
});

function renderLinks() {
  const filter = document.getElementById('extensionFilter').value.toLowerCase();
  const list = document.getElementById('linkList');
  const stats = document.getElementById('stats');
  
  const manifests = allLinks.filter(l => l.priority === 'MANIFEST');
  const normal = allLinks.filter(l => !l.priority || l.priority === 'NORMAL');
  const segments = allLinks.filter(l => l.priority === 'SEGMENT');

  const showSegments = filter.includes('segment') || filter.includes('.ts') || filter.includes('.m4s');
  
  let displayList = [...manifests, ...normal];
  if (showSegments) displayList = [...displayList, ...segments];

  const filtered = displayList.filter(l => l.url.toLowerCase().includes(filter));
  
  list.innerHTML = filtered.map((link, idx) => {
    let tag = '';
    let style = '';
    if (link.priority === 'MANIFEST') {
        tag = '<span style="color:#00ff00; border:1px solid #00ff00; padding:1px 4px; border-radius:3px; font-size:10px; margin-right:5px;">STITCHABLE</span>';
        style = 'background: rgba(0,255,0,0.05); border-left: 4px solid #00ff00;';
    } else if (link.isSiphoned) {
        tag = '<span style="color:#aaa; border:1px solid #444; padding:1px 4px; border-radius:3px; font-size:10px; margin-right:5px;">SIPHONED</span>';
        style = 'border-left: 4px solid #444;';
    }

    return `
    <div class="link-item" style="${style}">
      <input type="checkbox" ${link.priority === 'MANIFEST' ? 'checked' : ''} data-url="${link.url}" data-priority="${link.priority || 'NORMAL'}">
      <div class="link-name">
        ${tag}
        ${link.text || 'Untitled'}
      </div>
      <div class="link-url" title="${link.url}">${link.url}</div>
      ${link.size ? `<div style="font-size:11px; color:#888;">(${(link.size/1024/1024).toFixed(2)} MB)</div>` : ''}
    </div>
  `;}).join('');
  
  stats.innerText = `Found ${allLinks.length} items (showing ${filtered.length}). Manifests prioritized.`;
}

document.getElementById('extensionFilter').addEventListener('input', renderLinks);

document.getElementById('selectAll').onclick = () => {
  document.querySelectorAll('input[type="checkbox"]').forEach(cb => cb.checked = true);
};

document.getElementById('selectNone').onclick = () => {
  document.querySelectorAll('input[type="checkbox"]').forEach(cb => cb.checked = false);
};

document.getElementById('cancelBtn').onclick = () => window.close();

document.getElementById('downloadBtn').onclick = async () => {
  const selectedCheckboxes = Array.from(document.querySelectorAll('input[type="checkbox"]:checked'));
    
  if (selectedCheckboxes.length === 0) {
    alert("Please select at least one file.");
    return;
  }

  const btn = document.getElementById('downloadBtn');
  btn.disabled = true;
  btn.innerText = "Sending...";

  const folder = document.getElementById("folder").value;
  const activeMode = modeSelect.value;
  const activeGhostEngine = ghostEngineSelect.value;

  const writeSubs = document.getElementById("writeSubs").checked;
  const embedThumbnail = document.getElementById("embedThumbnail").checked;
  const embedChapters = document.getElementById("embedChapters").checked;
  const compressVideo = document.getElementById("compressVideo").checked;
  const liveSuppVal = liveSupport ? liveSupport.checked : false;
  const liveFromStartVal = liveFromStart ? liveFromStart.checked : false;

  const useNative = activeMode === "ghost" && activeGhostEngine === "chrome_native";
  const reqEngine = useNative ? "chrome_native" : (activeMode === "ghost" ? activeGhostEngine : null);

  const BATCH_SIZE = 5;
  const total = selectedCheckboxes.length;
  let count = 0;

  async function processBatch(batch) {
    return Promise.all(batch.map(async (cb) => {
        const url = cb.dataset.url;
        const priority = cb.dataset.priority;
        
        try {
            const nc = await new Promise(r => chrome.runtime.sendMessage({ action: "get_netscape_cookies", url }, r));
            
            let cookieStr = "";
            try {
                const rawCookies = await new Promise(r => {
                    chrome.cookies.getAll({ url }, r);
                });
                if (rawCookies && rawCookies.length > 0) {
                    cookieStr = rawCookies.map(c => `${c.name}=${c.value}`).join("; ");
                }
            } catch(e) {}

            await chrome.runtime.sendMessage({
                cmd: "AddDownload",
                ...{ 
                    url: url, 
                    folder: folder,
                    userAgent: navigator.userAgent,
                    netscape_cookies: nc || null,
                    cookies: cookieStr || null,
                    format: priority === 'MANIFEST' ? 'best' : null,
                    mode: activeMode,
                    engine: reqEngine,
                    write_subs: writeSubs,
                    embed_thumbnail: embedThumbnail,
                    embed_chapters: embedChapters,
                    compress_video: compressVideo,
                    live_support: liveSuppVal,
                    live_from_start: liveFromStartVal
                }
            });
            count++;
            btn.innerText = `Sending... (${count}/${total})`;
        } catch (e) {
            console.error("JADMan: Failed to send", url, e);
        }
    }));
  }

  for (let i = 0; i < selectedCheckboxes.length; i += BATCH_SIZE) {
    const batch = selectedCheckboxes.slice(i, i + BATCH_SIZE);
    await processBatch(batch);
  }
  
  window.close();
};
