# 📖 JADMan Multi-Platform Setup & Configuration Manual

This guide describes how to install dependencies, compile the binaries, and configure JADMan on **Windows**, **macOS**, **Linux**, and **Android/Termux**.

---

## 🪟 1. Windows Installation Guide

If you are sharing JADMan with a friend on Windows, send them these instructions.

### Step 1: Install Core Dependencies
JADMan uses third-party CLI engines for multi-threading and media extraction. These must be installed and added to the system `PATH`.

The easiest way to install them is using **Winget** (the Windows Package Manager). Open PowerShell and run:
```powershell
# Install aria2c downloader
winget install aria2.aria2

# Install yt-dlp media downloader
winget install yt-dlp.yt-dlp

# Install FFmpeg converter/remuxer
winget install Gyan.FFmpeg
```
*Note: Close and reopen your terminal after installation to refresh the PATH.*

### Step 2: Download or Compile JADMan
*   **From Codeberg Releases**: Download `jadman-x86_64-pc-windows-msvc.zip` from the repository's Release page and extract `jadm-daemon.exe` and `jadm-tui.exe` to a folder of your choice (e.g. `C:\Program Files\JADMan\`).
*   **Compile from Source**: Install the Rust compiler from [rustup.rs](https://rustup.rs/), clone the repository, and compile:
    ```cmd
    git clone https://codeberg.org/snowfox-rubait-96/jadman.git
    cd jaddman
    cargo build --release
    ```
    Your binaries will be generated at `target\release\jadm-daemon.exe` and `target\release\jadm-tui.exe`.

### Step 3: Register the Browser Native Messaging Host
To allow Chrome or Firefox to communicate with the JADMan background daemon, register the executable in the Windows Registry:
1.  Open Command Prompt or PowerShell.
2.  Navigate to the folder containing `jadm-daemon.exe`.
3.  Run the registration installer:
    ```cmd
    jadm-daemon.exe install-native-manifest
    ```
This creates the Registry keys under `HKCU\Software\Google\Chrome\NativeMessagingHosts\com.jadm.jadm` pointing to the JSON manifest automatically.

### Step 4: Install the Browser Extension
1.  Open Google Chrome or Brave.
2.  Navigate to: `chrome://extensions`
3.  Toggle the **Developer mode** switch (top-right corner).
4.  Click **Load unpacked** (top-left corner) and select the `extension/chrome/` folder from the repository.

*(For Firefox, go to `about:debugging#/runtime/this-firefox`, click **Load Temporary Add-on**, and select any file inside `extension/firefox/`)*

### Step 5: Run JADMan
1.  **Start the background daemon**: Run `jadm-daemon.exe`. It runs silently in the background waiting for extension calls.
2.  **Open TUI Console**: Run `jadm-tui.exe` to see your active downloads, metrics, and manage your queue.

---

## 🍎 2. macOS Installation Guide

### Step 1: Install Dependencies
Install the required packages using **Homebrew**:
```bash
brew install aria2 yt-dlp ffmpeg
```

### Step 2: Compile & Install Native Messaging Host
1.  Build the release binaries:
    ```bash
    cargo build --release
    ```
2.  Install the native messaging manifest path linking JADMan to your local browsers:
    ```bash
    target/release/jadm-daemon install-native-manifest
    ```

### Step 3: Install Browser Extension
1.  Open Chrome and navigate to `chrome://extensions`.
2.  Enable **Developer Mode**.
3.  Click **Load unpacked** and select the `extension/chrome/` folder.

### Step 4: Run
1.  Start the daemon in the background:
    ```bash
    target/release/jadm-daemon &
    ```
2.  Start the TUI controller:
    ```bash
    target/release/jadm-tui
    ```

---

## 🐧 3. Linux Installation Guide

### Step 1: Install Dependencies
```bash
sudo pacman -S aria2 yt-dlp ffmpeg   # Arch Linux
sudo apt install aria2 yt-dlp ffmpeg # Debian/Ubuntu
```

### Step 2: Build & Register
```bash
cargo build --release
target/release/jadm-daemon install-native-manifest
```

### Step 3: Load Extension
Enable developer mode in `chrome://extensions` and click **Load unpacked** pointing to the `extension/chrome/` directory.

---

## 🤖 4. Android/Termux Installation Guide

For power users who want JADMan running natively on their Android phone:

### Step 1: Set up the Termux Environment
Download **Termux** (via F-Droid), open it, and install compiler tools and downloader packages:
```bash
pkg update
pkg install rust clang make python ffmpeg aria2 git
```

### Step 2: Clone and Compile
```bash
git clone https://codeberg.org/snowfox-rubait-96/jadman.git
cd jaddman
cargo build --release
```

### Step 3: Running inside Termux
1.  Start the daemon inside Termux:
    ```bash
    target/release/jadm-daemon &
    ```
2.  Open Kiwi Browser or Firefox Beta on Android (which support unpacked extensions), load the JADMan extension, and configure browser proxy settings to redirect downloads directly to localhost (`127.0.0.1:6246`).
3.  Open the TUI client on your phone:
    ```bash
    target/release/jadm-tui
    ```
