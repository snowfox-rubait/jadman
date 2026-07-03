use std::path::Path;
use anyhow::Result;
use mlua::prelude::*;
use jadm_common::types::Download;
use tokio::sync::{mpsc, oneshot};

pub struct ScriptManager {
    sender: mpsc::UnboundedSender<LuaRequest>,
}

enum LuaRequest {
    TriggerHook {
        event_name: String,
        download: Download,
    },
    DebridResolve {
        url: String,
        responder: oneshot::Sender<Option<String>>,
    },
    #[allow(dead_code)]
    RecoveryResolve {
        url: String,
        error_code: u16,
        responder: oneshot::Sender<Option<String>>,
    },
}

impl ScriptManager {
    pub fn new(config_dir: &Path) -> Result<Self> {
        let (sender, mut receiver) = mpsc::unbounded_channel::<LuaRequest>();
        let plugins_dir = config_dir.join("plugins");
        std::fs::create_dir_all(&plugins_dir)?;

        let plugins_dir_clone = plugins_dir.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build local Tokio runtime for Lua worker");

            rt.block_on(async move {
                let lua = Lua::new();

                if let Err(e) = init_bindings(&lua) {
                    eprintln!("[JADMan Lua Worker] Failed to init bindings: {}", e);
                    return;
                }
                if let Err(e) = load_plugins(&lua, &plugins_dir_clone) {
                    eprintln!("[JADMan Lua Worker] Failed to load plugins: {}", e);
                    return;
                }

                println!("[JADMan Lua Worker] Worker thread started successfully.");

                while let Some(req) = receiver.recv().await {
                    match req {
                        LuaRequest::TriggerHook { event_name, download } => {
                            if let Ok(download_lua) = lua.to_value(&download) {
                                let fut = lua.load(r#"
                                    local event_name, download = ...
                                    local callbacks = jadm._hooks[event_name]
                                    if callbacks then
                                        for _, cb in ipairs(callbacks) do
                                            pcall(cb, download)
                                        end
                                    end
                                "#)
                                .call_async::<_, ()>((event_name, download_lua));
                                if let Err(e) = fut.await {
                                    eprintln!("[JADMan Lua Worker] trigger_hook failed: {}", e);
                                }
                            }
                        }
                        LuaRequest::DebridResolve { url, responder } => {
                            let fut = lua.load(r#"
                                local url = ...
                                for _, resolver in ipairs(jadm._debrid_resolvers) do
                                    local ok, can_handle = pcall(resolver.can_handle, url)
                                    if ok and can_handle then
                                        local ok2, res = pcall(resolver.resolve, url)
                                        if ok2 and res then
                                            return res
                                        end
                                    end
                                end
                                return nil
                            "#)
                            .call_async::<_, Option<String>>(url);
                            let res = fut.await.unwrap_or(None);
                            let _ = responder.send(res);
                        }
                        LuaRequest::RecoveryResolve { url, error_code, responder } => {
                            let fut = lua.load(r#"
                                local url, error_code = ...
                                for _, resolver in ipairs(jadm._recovery_resolvers) do
                                    local ok, can_handle = pcall(resolver.can_handle, url, error_code)
                                    if ok and can_handle then
                                        local ok2, res = pcall(resolver.resolve, url, error_code)
                                        if ok2 and res then
                                            return res
                                        end
                                    end
                                end
                                return nil
                            "#)
                            .call_async::<_, Option<String>>((url, error_code));
                            let res = fut.await.unwrap_or(None);
                            let _ = responder.send(res);
                        }
                    }
                }
            });
        });

        Ok(Self { sender })
    }

    /// Triggers all registered callbacks for a specific lifecycle event.
    pub async fn trigger_hook(&self, event_name: &str, download: Download) -> Result<()> {
        self.sender.send(LuaRequest::TriggerHook {
            event_name: event_name.to_string(),
            download,
        }).map_err(|_| anyhow::anyhow!("Lua worker thread channel closed"))?;
        Ok(())
    }

    /// Iterates over registered debrid resolvers. Returns the resolved download URL if handled.
    pub async fn debrid_resolve(&self, url: &str) -> Result<Option<String>> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(LuaRequest::DebridResolve {
            url: url.to_string(),
            responder: tx,
        }).map_err(|_| anyhow::anyhow!("Lua worker thread channel closed"))?;
        
        let res = rx.await.unwrap_or(None);
        Ok(res)
    }

    /// Iterates over registered link recovery resolvers. Returns the recovered URL if handled.
    #[allow(dead_code)]
    pub async fn recovery_resolve(&self, url: &str, error_code: u16) -> Result<Option<String>> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(LuaRequest::RecoveryResolve {
            url: url.to_string(),
            error_code,
            responder: tx,
        }).map_err(|_| anyhow::anyhow!("Lua worker thread channel closed"))?;
        
        let res = rx.await.unwrap_or(None);
        Ok(res)
    }
}

fn init_bindings(lua: &Lua) -> Result<()> {
    let globals = lua.globals();
    
    let jadm_rust = lua.create_table()?;
    
    // HTTP Subtable
    let http = lua.create_table()?;
    
    http.set("get", lua.create_async_function(|_, (url, headers): (String, Option<std::collections::HashMap<String, String>>)| async move {
        let mut builder = reqwest::Client::new().get(&url);
        if let Some(hdrs) = headers {
            for (k, v) in hdrs {
                builder = builder.header(k, v);
            }
        }
        let res = builder.send().await
            .map_err(|e| mlua::Error::RuntimeError(format!("HTTP GET failed: {}", e)))?;
        let status = res.status().as_u16();
        let text = res.text().await
            .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read body: {}", e)))?;
        Ok((status, text))
    })?)?;

    http.set("post", lua.create_async_function(|_, (url, body, headers): (String, String, Option<std::collections::HashMap<String, String>>)| async move {
        let mut builder = reqwest::Client::new().post(&url);
        if let Some(hdrs) = headers {
            for (k, v) in hdrs {
                builder = builder.header(k, v);
            }
        }
        let res = builder.body(body).send().await
            .map_err(|e| mlua::Error::RuntimeError(format!("HTTP POST failed: {}", e)))?;
        let status = res.status().as_u16();
        let text = res.text().await
            .map_err(|e| mlua::Error::RuntimeError(format!("Failed to read body: {}", e)))?;
        Ok((status, text))
    })?)?;
    jadm_rust.set("http", http)?;

    // System notification
    jadm_rust.set("notify", lua.create_function(|_, (title, message): (String, String)| {
        let _ = notify_rust::Notification::new()
            .summary(&title)
            .body(&message)
            .show();
        Ok(())
    })?)?;

    // Initialize helper tables and registration functions
    lua.load(r#"
        jadm = jadm or {}
        jadm._hooks = {
            on_download_start = {},
            on_download_complete = {},
            on_download_error = {},
        }
        jadm.register_hook = function(event_name, callback)
            if jadm._hooks[event_name] then
                table.insert(jadm._hooks[event_name], callback)
            else
                error("Unknown hook event: " .. tostring(event_name))
           end
        end

        jadm._debrid_resolvers = {}
        jadm.register_debrid = function(resolver)
            table.insert(jadm._debrid_resolvers, resolver)
        end

        jadm._recovery_resolvers = {}
        jadm.register_recovery = function(resolver)
            table.insert(jadm._recovery_resolvers, resolver)
        end
    "#).exec()?;

    // Merge Rust functions into pre-initialized global `jadm` table
    let jadm_global: LuaTable = globals.get("jadm")?;
    for pair in jadm_rust.pairs::<LuaValue, LuaValue>() {
        let (k, v) = pair?;
        jadm_global.set(k, v)?;
    }

    Ok(())
}

fn load_plugins(lua: &Lua, plugins_dir: &Path) -> Result<()> {
    let subdirs = ["debrid", "hooks", "recovery"];
    for subdir in &subdirs {
        let dir = plugins_dir.join(subdir);
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "lua") {
                let content = std::fs::read_to_string(&path)?;
                let name = path.file_name().unwrap().to_string_lossy();
                lua.load(&content).set_name(name.as_ref()).exec()
                    .map_err(|e| anyhow::anyhow!("Failed to load plugin: {}/{}: {}", subdir, name, e))?;
                println!("[JADMan Lua] Loaded plugin: {}/{}", subdir, name);
            }
        }
    }
    Ok(())
}
