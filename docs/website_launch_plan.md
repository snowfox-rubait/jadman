# 🌐 JADMan Serverless Website Launch Plan (Minimalist / Human-Made)

This updated plan targets a lightweight, fast, and serverless website hosted on Cloudflare Pages, prioritizing utility, clean system typography, and a hand-crafted open-source aesthetic.

---

## 🎨 1. Aesthetic Direction: Plain, Clean, & Utilitarian

To avoid the generic, over-designed "AI template" look, JADMan's site will adopt a minimalist style inspired by high-utility technical documentations (e.g., `mitmproxy`, `curl`, and `rust-lang` pages).

### Core Visuals:
*   **Colors**: Flat dark gray background (`#0b0d0f`), cool slate borders (`#20242c`), crisp white text (`#f8f9fa`), and deep green terminal accents (`#4ade80`).
*   **Typography**: Clean System UI Font Stack (San Francisco, Segoe UI, Roboto) for reading, and Monospace (SFMono, JetBrains Mono) for commands.
*   **No Clichés**: No vector illustrations, no gradients, no glowing shadow cards. Just clean borders, raw grids, and terminal mockups.

---

## 🏗️ 2. File Structure (`/website`)

We will keep the codebase simple and dependency-free so it can be deployed directly to Cloudflare Pages:
*   `website/index.html`: Single semantic HTML5 document.
*   `website/style.css`: Clean, hand-crafted vanilla CSS layout using Flexbox/CSS Grid.
*   `website/script.js`: Client-side browser logic to dynamically fetch the latest release asset URLs from Codeberg's API and auto-detect the user's OS.

---

## 📝 3. Page Structure

1.  **Header**: Simple logo text + Latest Version badge (resolves dynamically via Javascript).
2.  **Introduction**: Direct description of what JADMan is and what it solves.
3.  **Command Installation Box**: One-click install terminal command.
4.  **Terminal Mockup**: A CSS-styled rendering of the TUI interface inside a classic terminal shell window.
5.  **Features Grid**: 2x2 flat table showing capabilities: Segmented Downloads, Video Sniffing, Cookie Decryption, and Scheduler hooks.
6.  **Platforms & Downloads**: Clean tabs mapping instructions and links for Windows, Linux, macOS, and Android.
7.  **Footer**: GPLv3, link to Codeberg/GitHub repos.
