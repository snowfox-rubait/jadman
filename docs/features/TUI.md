# JADMan TUI Features

## Status: Operational Terminal Interface

### Interface Structure
- [x] **Three-Panel Design:** Navigation (Categories), Main List (Queue), and Inspection (Detail/Preview).
- [x] **Vim-Inspired Navigation:** Support for `h`/`l` (panel focus), `j`/`k` (item selection), `g`/`G` (top/bottom).
- [x] **Dynamic Columns:** Real-time display of Name, Size, Rate, ETA, and Status.
- [x] **Visual Progress:** Pacman-style progress bars (`[#######c   ]`) with animated mouth indicator.

### Functionality & Controls
- [x] **Syntax Preview Engine:** High-performance syntax highlighting for code and text files using `syntect`.
- [x] **Stream URL Clipboard Copy:** Press `v` or `V` to copy the seekable streaming URL (`http://127.0.0.1:6246/stream/:id`) to clipboard for immediate VLC or mpv playback.
- [x] **Cookie Master Password Modal:** Press `c` or `C` to securely enter the master password to decrypt Netscape cookies.
- [x] **Add URL Dialog:** Press `a` to open modal input to add downloads directly from the terminal.
- [x] **Download Controls:** Single-key controls for Pause (`p`), Resume (`r`), Stop (`s`), Remove from list (`d`), Remove and delete file from disk (`D`).
- [x] **Crash Protection:** Robust `TerminalGuard` and panic hook restoring terminal raw mode and cursor on any error unwind.
- [x] **Real-Time Polling:** Synchronized with the daemon via Unix RPC socket (`Request::GetQueue`).

### Planned Features
- [ ] **Interactive Search:** Press `/` to live-filter the download list by name.
- [ ] **Multi-Select:** Non-contiguous and range selection for batch actions.
- [ ] **Sorting Submenu:** Press `m` to sort by name, size, date added, or transfer rate.
- [ ] **Inline Terminal Image/Video Previews:** Native rendering using the Kitty graphics protocol with Chafa ASCII fallback.
- [ ] **Config Editor:** Edit JADMan configuration files directly inside the TUI.
