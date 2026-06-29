// 🌐 JADMan Browser Logic

// 1. DYNAMIC RELEASE FETCHER (CODEBERG API)
const DEFAULT_VERSION = "v0.1.0";
const REPO_URL = "https://codeberg.org/snowfox-rubait-96/jadman";
const API_URL = "https://codeberg.org/api/v1/repos/snowfox-rubait-96/jadman/releases";

async function loadLatestRelease() {
    const badge = document.getElementById("version-badge");
    const winBtn = document.getElementById("btn-download-windows");
    const macBtn = document.getElementById("btn-download-macos");
    const linBtn = document.getElementById("btn-download-linux");

    try {
        const response = await fetch(API_URL);
        if (!response.ok) throw new Error(`API error: ${response.status}`);
        
        const releases = await response.json();
        if (!releases || releases.length === 0) {
            throw new Error("No releases found in Codeberg repo");
        }

        const latest = releases[0];
        const version = latest.tag_name || DEFAULT_VERSION;

        // Update version badge
        badge.innerText = `latest: ${version}`;
        badge.classList.add("active");

        // Update direct download links
        winBtn.href = `${REPO_URL}/releases/download/${version}/jadman-x86_64-pc-windows-msvc.zip`;
        macBtn.href = `${REPO_URL}/releases/download/${version}/jadman-x86_64-apple-darwin.tar.gz`;
        linBtn.href = `${REPO_URL}/releases/download/${version}/jadman-x86_64-unknown-linux-gnu.tar.gz`;

    } catch (err) {
        console.warn("Fallback to manual releases page:", err);
        badge.innerText = "latest releases";
        
        // Fallback links point to releases UI page directly
        const fallbackUrl = `${REPO_URL}/releases`;
        winBtn.href = fallbackUrl;
        macBtn.href = fallbackUrl;
        linBtn.href = fallbackUrl;
    }
}

// 2. OS DETECTION & AUTO-TAB SELECT
function autoDetectOS() {
    const userAgent = window.navigator.userAgent;
    let targetTabId = "tab-linux"; // Default

    if (userAgent.indexOf("Win") !== -1) {
        targetTabId = "tab-windows";
    } else if (userAgent.indexOf("Mac") !== -1) {
        targetTabId = "tab-macos";
    } else if (userAgent.indexOf("Android") !== -1) {
        targetTabId = "tab-android";
    } else if (userAgent.indexOf("Linux") !== -1) {
        targetTabId = "tab-linux";
    }

    // Programmatically trigger tab switch
    const links = document.querySelectorAll(".tab-link");
    links.forEach(link => {
        const onclickAttr = link.getAttribute("onclick") || "";
        if (onclickAttr.includes(targetTabId)) {
            // Find target element and switch
            const mockEvent = { currentTarget: link };
            switchTab(mockEvent, targetTabId);
        }
    });
}

// 3. TAB CONTROLLER
function switchTab(evt, tabId) {
    // Hide all tab contents
    const tabContents = document.querySelectorAll(".tab-content");
    tabContents.forEach(content => {
        content.classList.remove("active");
    });

    // Remove active class from all tab links
    const tabLinks = document.querySelectorAll(".tab-link");
    tabLinks.forEach(link => {
        link.classList.remove("active");
    });

    // Show selected tab content, add active class to triggering button
    document.getElementById(tabId).classList.add("active");
    if (evt && evt.currentTarget) {
        evt.currentTarget.classList.add("active");
    }
}

// 4. COPY TO CLIPBOARD HELPER
function copyInstallCommand() {
    const cmdText = document.getElementById("install-cmd").innerText;
    const btn = document.querySelector(".copy-btn");

    navigator.clipboard.writeText(cmdText).then(() => {
        const originalText = btn.innerText;
        btn.innerText = "Copied!";
        btn.style.borderColor = "var(--accent-green)";
        btn.style.color = "var(--accent-green)";

        setTimeout(() => {
            btn.innerText = originalText;
            btn.style.borderColor = "";
            btn.style.color = "";
        }, 2000);
    }).catch(err => {
        console.error("Clipboard copy failed:", err);
    });
}

// 5. RUN INITIALIZERS ON DOM LOAD
document.addEventListener("DOMContentLoaded", () => {
    loadLatestRelease();
    autoDetectOS();
});
