use serde::Deserialize;
use anyhow::Result;
use tokio::fs;
use directories::ProjectDirs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub hooks: HooksConfig,
    #[serde(default = "default_max_concurrent_downloads")]
    pub max_concurrent_downloads: usize,
    #[serde(default)]
    pub proxy: ProxyConfig,
}

fn default_max_concurrent_downloads() -> usize {
    5
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hooks: HooksConfig {
                on_float_toggle: default_on_float_toggle(),
            },
            max_concurrent_downloads: default_max_concurrent_downloads(),
            proxy: ProxyConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct HooksConfig {
    #[serde(default = "default_on_float_toggle")]
    pub on_float_toggle: String,
}

fn default_on_float_toggle() -> String {
    #[cfg(target_os = "linux")]
    { "hyprctl dispatch togglespecialworkspace jadm_float".to_string() }
    #[cfg(not(target_os = "linux"))]
    { "".to_string() }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProxyConfig {
    #[serde(default = "default_proxy_enabled")]
    pub enabled: bool,
    #[serde(default = "default_proxy_port")]
    pub port: u16,
    #[serde(default = "default_proxy_mark")]
    pub mark: u32,
    #[serde(default = "default_proxy_setup_network")]
    pub setup_network: bool,
    #[serde(default = "default_proxy_install_ca")]
    pub install_ca: bool,
}

fn default_proxy_enabled() -> bool { false }
fn default_proxy_port() -> u16 { 6248 }
fn default_proxy_mark() -> u32 { 0x6248 }
fn default_proxy_setup_network() -> bool { false }
fn default_proxy_install_ca() -> bool { false }

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            enabled: default_proxy_enabled(),
            port: default_proxy_port(),
            mark: default_proxy_mark(),
            setup_network: default_proxy_setup_network(),
            install_ca: default_proxy_install_ca(),
        }
    }
}

impl Config {
    pub async fn load() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "jadm", "jadm")
            .ok_or_else(|| anyhow::anyhow!("Could not determine project directories"))?;
        
        let config_dir = proj_dirs.config_dir();
        let config_path = config_dir.join("config.toml");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path).await?;
            match toml::from_str(&content) {
                Ok(config) => Ok(config),
                Err(e) => {
                    eprintln!("Warning: Failed to parse config.toml: {}. Using default config.", e);
                    Ok(Config::default())
                }
            }
        } else {
            Ok(Config::default())
        }
    }
}

pub fn get_socks_token() -> anyhow::Result<String> {
    let proj_dirs = ProjectDirs::from("com", "jadm", "jadm")
        .ok_or_else(|| anyhow::anyhow!("Could not determine project directories"))?;
    let token_path = proj_dirs.runtime_dir()
        .map(|d| d.join("jadm_socks_token"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp/jadm_socks_token"));
    
    std::fs::read_to_string(token_path)
        .map_err(|_| anyhow::anyhow!("SOCKS token not found — is jadm-daemon running?"))
}
