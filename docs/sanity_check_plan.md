# 🛡️ JADMan Code Sanity & Security Check Plan

This plan details the vulnerability detection strategies and compilation sanity audits we will perform to verify the integrity and safety of the JADMan project.

---

## 🔍 Assessment Areas

### 1. Command & Process Injection
* **Risk**: Obfuscated URLs, filenames, or user-supplied passwords could trigger arbitrary code execution when running helper processes (`yt-dlp`, `aria2c`, `ffmpeg`, `cookie-jar`).
* **Check**: Inspect all `tokio::process::Command` calls to ensure arguments are passed as discrete elements (never concatenated into raw shells) and sensitive variables are mapped via environment inputs.

### 2. Path Traversal & File System Safety
* **Risk**: Website-provided filenames (e.g. `../../etc/passwd`) or custom download folder paths could lead to writing files in unauthorized system locations.
* **Check**: Audit how filenames are sanitized, verified, and joined onto target destination folders.

### 3. SQL Injection
* **Risk**: Dynamic queries built using string interpolation could allow executing malicious statements.
* **Check**: Inspect all query calls in `crates/jadm-daemon/src/db/queries.rs` and local SQLite connections to confirm parameterized bindings are universally enforced.

### 4. Code Health & Compiler Safety
* **Risk**: Implicit panics, unhandled errors, or legacy security holes in cargo dependencies.
* **Check**: Run compiler diagnostics (`cargo clippy`) and dependency safety audits.

---

## 📋 Execution Steps

- [ ] **Step 1**: Search and audit all shell command executions (`Command::new`).
- [ ] **Step 2**: Search and audit all filename sanitization paths.
- [ ] **Step 3**: Search and audit all database queries for string interpolation.
- [ ] **Step 4**: Run compiler audit (`cargo clippy` and compilation warnings sweep).
