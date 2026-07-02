// Safe chrome.runtime.sendMessage wrapper to prevent "No SW" / "Context Invalidated" warnings
if (typeof chrome !== 'undefined' && chrome.runtime) {
    const originalSendMessage = chrome.runtime.sendMessage;
    chrome.runtime.sendMessage = function(message, callback) {
        if (!chrome.runtime?.id) return;
        try {
            if (typeof callback === 'function') {
                originalSendMessage.call(chrome.runtime, message, (response) => {
                    const err = chrome.runtime.lastError;
                    if (!err) {
                        callback(response);
                    }
                });
            } else {
                originalSendMessage.call(chrome.runtime, message, () => {
                    const _ = chrome.runtime.lastError;
                });
            }
        } catch(e) {}
    };
}

// JADMan Menu UI Controller

let activeTabId = null;
let activeTabUrl = null;

const modes = ['general', 'siphon', 'ghost'];

// Restore active mode and fetch tab info on load
document.addEventListener('DOMContentLoaded', async () => {
  // 1. Get active tab
  chrome.tabs.query({ active: true, currentWindow: true }, (tabs) => {
    const tab = tabs[0];
    if (tab) {
      activeTabId = tab.id;
      activeTabUrl = tab.url;
      
      // 2. Fetch detected streams count
      chrome.runtime.sendMessage({ action: "get_tab_discovery", tabId: activeTabId }, (response) => {
        if (response) {
          const totalCount = (response.urls?.length || 0) + (response.siphoned?.length || 0);
          document.getElementById('detected-count').innerText = totalCount;
        }
      });
    }
  });

  // 3. Load active mode from storage
  chrome.storage.local.get(['downloadMode'], (res) => {
    const currentMode = res.downloadMode || 'general';
    setActiveModeUI(currentMode);
  });

  // 4. Bind mode options clicks
  modes.forEach(m => {
    const el = document.getElementById(`opt-${m}`);
    if (el) {
      el.addEventListener('click', () => {
        chrome.storage.local.set({ downloadMode: m }, () => {
          setActiveModeUI(m);
        });
      });
    }
  });
});

function setActiveModeUI(mode) {
  modes.forEach(m => {
    const el = document.getElementById(`opt-${m}`);
    if (el) {
      if (m === mode) {
        el.classList.add('active');
      } else {
        el.classList.remove('active');
      }
    }
  });
}

// Bind Action Buttons
document.getElementById('open-toolbox-btn').addEventListener('click', () => {
  if (activeTabId !== null) {
    chrome.windows.create({
      url: `toolbox.html?tabId=${activeTabId}`,
      type: "popup", width: 800, height: 600, focused: true
    });
    window.close();
  }
});

document.getElementById('open-grabber-btn').addEventListener('click', () => {
  if (activeTabId !== null) {
    chrome.runtime.sendMessage({ action: "open_grabber_for_tab", tabId: activeTabId });
    window.close();
  }
});

document.getElementById('add-url-btn').addEventListener('click', () => {
  if (activeTabUrl) {
    chrome.windows.create({
      url: `popup.html?url=${encodeURIComponent(activeTabUrl)}`,
      type: "popup", width: 450, height: 380, focused: true
    });
    window.close();
  }
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
    toggleActiveBtn.style.background = "#00ff66";
    toggleActiveBtn.style.color = "#000";
  } else {
    statusLabel.innerText = "🟢 JADMan Active";
    statusLabel.style.color = "#00ff66";
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

