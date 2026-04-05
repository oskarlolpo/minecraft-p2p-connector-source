use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};
#[cfg(embedded_ygg)]
use std::{ffi::CStr, os::raw::c_char};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;
use tokio::task;

use crate::models::YggstackRuntimeInfo;

#[cfg(all(target_os = "windows", not(embedded_ygg)))]
const EMBEDDED_YGGSTACK_EXE: &[u8] =
    include_bytes!("../../resources/yggstack/yggstack.exe");

#[derive(Clone)]
pub struct YggstackManager {
    config: YggstackConfig,
    process: Arc<Mutex<Option<ManagedProcess>>>,
}

#[derive(Clone)]
struct YggstackConfig {
    source_dir: Option<PathBuf>,
    runtime_dir: PathBuf,
    binary_path: PathBuf,
    config_path: PathBuf,
    log_path: PathBuf,
}

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

struct ManagedProcess {
    child: Child,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct EmbeddedBridgeStatus {
    running: bool,
    public_key: Option<String>,
    address: Option<String>,
    subnet: Option<String>,
    peers_json: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct YggPublicPeerMeta {
    up: Option<bool>,
    response_ms: Option<u64>,
}

struct BootstrapPeers {
    peers: Vec<String>,
    source: String,
}

const YGG_PUBLIC_PEER_FEEDS: &[&str] = &[
    "https://publicpeers.neilalexander.dev/publicnodes.json",
    "https://peers.yggdrasil.link/publicnodes.json",
];

const YGG_FALLBACK_PEERS: &[&str] = &[
    "tls://103.109.234.106:443?key=000000035621c71b5610434589df051aed2688510f904ae79860668dc0fbf182",
    "tls://b.ygg.yt:443",
    "tls://g.ygg.yt:443",
    "tls://n.ygg.yt:443",
    "tls://45.147.200.202:443",
    "tls://45.95.202.21:443",
    "tls://193.93.119.42:443",
    "wss://assets.route172.de:443/api/request/media?key=00000000000da547036a01860a9e3a0476a525415801ec34f4e5b59fd6055b88",
    "wss://donotclickthis.link:443/api/v2/socket",
    "wss://ygg-evn-1.wgos.org:443",
];

impl YggstackManager {
    pub fn from_env() -> Self {
        let source_dir = env::var("MC_YGGSTACK_SOURCE_DIR")
            .map(PathBuf::from)
            .ok()
            .filter(|path| !path.as_os_str().is_empty());

        let runtime_root = env::var("MC_YGGSTACK_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_runtime_dir());

        let runtime_dir = runtime_root.join("yggstack");
        let binary_path = runtime_dir.join(yggstack_binary_name());
        let config_path = runtime_dir.join("yggstack.autogen.conf");
        let log_path = runtime_dir.join("yggstack.log");

        Self {
            config: YggstackConfig {
                source_dir,
                runtime_dir,
                binary_path,
                config_path,
                log_path,
            },
            process: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn runtime_info(&self) -> YggstackRuntimeInfo {
        if self.refresh_running_flag() {
            return self.sidecar_runtime_info();
        }

        if embedded_bridge_available() && !self.config.binary_path.exists() {
            return self.embedded_runtime_info();
        }

        self.sidecar_runtime_info()
    }

    pub async fn prepare_runtime(&self) -> Result<YggstackRuntimeInfo> {
        if embedded_bridge_available() && !self.config.binary_path.exists() {
            let (_, bootstrap_count, bootstrap_source) = self.build_embedded_config().await?;
            let mut info = self.embedded_runtime_info();
            info.ready = true;
            info.runtime_dir = Some(self.config.runtime_dir.display().to_string());
            info.config_path = Some(self.config.config_path.display().to_string());
            if info.note.is_empty() {
                info.note = format!(
                    "Ygg bootstrap подготовлен: {bootstrap_count} peer-узл(ов), источник {bootstrap_source}."
                );
            } else {
                info.note = format!(
                    "{} Bootstrap peers: {bootstrap_count}, source: {bootstrap_source}.",
                    info.note
                );
            }
            return Ok(info);
        }

        self.ensure_runtime_dir()?;
        self.ensure_binary_available().await?;
        self.ensure_sidecar_config_ready().await?;
        Ok(self.sidecar_runtime_info())
    }

    pub async fn start_sidecar(&self) -> Result<YggstackRuntimeInfo> {
        if embedded_bridge_available() && !self.config.binary_path.exists() {
            let (config_json, bootstrap_count, bootstrap_source) =
                self.build_embedded_config().await?;
            embedded_bridge_start_json(&config_json)?;
            let mut info = self.embedded_runtime_info();
            info.ready = true;
            info.runtime_dir = Some(self.config.runtime_dir.display().to_string());
            info.config_path = Some(self.config.config_path.display().to_string());
            info.note = format!(
                "{} Bootstrap peers: {bootstrap_count}, source: {bootstrap_source}.",
                info.note
            )
            .trim()
            .to_string();
            return Ok(info);
        }

        return self.start_sidecar_with_args(&[]).await;

    }

    pub async fn stop_sidecar(&self) -> Result<YggstackRuntimeInfo> {
        if embedded_bridge_available() {
            embedded_bridge_stop()?;
            return Ok(self.embedded_runtime_info());
        }

        if let Some(mut process) = self
            .process
            .lock()
            .map_err(|_| anyhow!("mutex yggstack process poisoned"))?
            .take()
        {
            let _ = process.child.kill();
            let _ = process.child.wait();
        }

        Ok(self.sidecar_runtime_info())
    }

    pub async fn retry_peers(&self) -> Result<YggstackRuntimeInfo> {
        if embedded_bridge_available() {
            embedded_bridge_retry_peers()?;
            return Ok(self.embedded_runtime_info());
        }

        Ok(self.sidecar_runtime_info())
    }

    pub async fn start_host_mapping(&self, local_game_port: u16) -> Result<YggstackRuntimeInfo> {
        if local_game_port == 0 {
            return Err(anyhow!("local game port must be greater than 0"));
        }

        let mapping = format!("-remote-tcp=25565:127.0.0.1:{local_game_port}");
        self.start_sidecar_with_args(&[mapping.as_str()]).await
    }

    pub async fn start_client_mapping(
        &self,
        remote_ygg_address: &str,
    ) -> Result<YggstackRuntimeInfo> {
        let remote_ygg_address = normalize_ygg_address(remote_ygg_address)
            .ok_or_else(|| anyhow!("invalid Ygg address: {remote_ygg_address}"))?;
        let mapping = format!("-local-tcp=127.0.0.1:25565:[{remote_ygg_address}]:25565");
        self.start_sidecar_with_args(&[mapping.as_str()]).await
    }

    fn embedded_runtime_info(&self) -> YggstackRuntimeInfo {
        match embedded_bridge_status() {
            Ok(status) => {
                let mut note_parts = Vec::new();
                if let Some(error) = status.error.as_deref() {
                    note_parts.push(format!("embedded bridge error: {error}"));
                }
                if status.running {
                    note_parts.push("Встроенный Yggstack bridge запущен.".into());
                } else {
                    note_parts.push("Встроенный Yggstack bridge готов.".into());
                }
                if let Some(address) = status.address.as_deref() {
                    note_parts.push(format!("Ygg address: {address}"));
                }
                if let Some(public_key) = status.public_key.as_deref() {
                    note_parts.push(format!("Public key: {public_key}"));
                }
                if let Some(subnet) = status.subnet.as_deref() {
                    note_parts.push(format!("Subnet: {subnet}"));
                }
                if let Some(peer_count) = embedded_peer_count(&status) {
                    note_parts.push(format!("Mesh peers: {peer_count}"));
                }

                YggstackRuntimeInfo {
                    ready: status.error.is_none(),
                    running: status.running,
                    source_dir: None,
                    runtime_dir: None,
                    binary_path: Some("embedded://yggstackbridge".into()),
                    config_path: None,
                    log_path: None,
                    ygg_public_key: status.public_key.clone(),
                    ygg_address: status.address.clone(),
                    ygg_subnet: status.subnet.clone(),
                    note: note_parts.join(" "),
                }
            }
            Err(error) => YggstackRuntimeInfo {
                ready: false,
                running: false,
                source_dir: None,
                runtime_dir: None,
                binary_path: Some("embedded://yggstackbridge".into()),
                config_path: None,
                log_path: None,
                ygg_public_key: None,
                ygg_address: None,
                ygg_subnet: None,
                note: format!("Встроенный Yggstack bridge недоступен: {error:#}"),
            },
        }
    }

    fn sidecar_runtime_info(&self) -> YggstackRuntimeInfo {
        let mut note_parts: Vec<String> = Vec::new();
        let binary_exists = self.config.binary_path.exists();
        let config_exists = self.config.config_path.exists();
        let running = self.refresh_running_flag();
        let ygg_address = if binary_exists && config_exists {
            query_sidecar_value(&self.config.binary_path, &self.config.config_path, "-address").ok()
        } else {
            None
        };
        let ygg_public_key = if binary_exists && config_exists {
            query_sidecar_value(&self.config.binary_path, &self.config.config_path, "-publickey").ok()
        } else {
            None
        };
        let ygg_subnet = if binary_exists && config_exists {
            query_sidecar_value(&self.config.binary_path, &self.config.config_path, "-subnet").ok()
        } else {
            None
        };

        if !binary_exists {
            note_parts.push(
                "Yggstack binary не найден. Приложение попробует встроенный bridge или встроенную bundled-копию.".into(),
            );
        }
        if binary_exists && !config_exists {
            note_parts.push("Конфиг yggstack ещё не сгенерирован.".into());
        }
        if running {
            note_parts.push("Yggstack runtime запущен.".into());
        }
        if note_parts.is_empty() {
            note_parts.push("Yggstack runtime готов.".into());
        }

        YggstackRuntimeInfo {
            ready: binary_exists && config_exists,
            running,
            source_dir: self.config.source_dir.as_ref().map(|path| path.display().to_string()),
            runtime_dir: Some(self.config.runtime_dir.display().to_string()),
            binary_path: Some(self.config.binary_path.display().to_string()),
            config_path: Some(self.config.config_path.display().to_string()),
            log_path: Some(self.config.log_path.display().to_string()),
            ygg_public_key,
            ygg_address,
            ygg_subnet,
            note: note_parts.join(" "),
        }
    }
    fn refresh_running_flag(&self) -> bool {
        let mut guard = match self.process.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };

        if let Some(process) = guard.as_mut() {
            match process.child.try_wait() {
                Ok(Some(_)) => {
                    *guard = None;
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    *guard = None;
                    false
                }
            }
        } else {
            false
        }
    }

    fn ensure_runtime_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.config.runtime_dir).with_context(|| {
            format!(
                "Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ РЎРѓР С•Р В·Р Т‘Р В°РЎвЂљРЎРЉ runtime Р С”Р В°РЎвЂљР В°Р В»Р С•Р С– {}",
                self.config.runtime_dir.display()
            )
        })
    }

    async fn ensure_binary_available(&self) -> Result<()> {
        if self.config.binary_path.exists() {
            return Ok(());
        }

        #[cfg(all(target_os = "windows", not(embedded_ygg)))]
        {
            write_embedded_binary(&self.config.binary_path)?;
            return Ok(());
        }

        if let Some(bundled_path) = resolve_bundled_binary_path() {
            copy_bundled_binary(&bundled_path, &self.config.binary_path)?;
            return Ok(());
        }

        if let Some(source_dir) = self.config.source_dir.clone().filter(|path| path.exists()) {
            let binary_path = self.config.binary_path.clone();
            let runtime_dir = self.config.runtime_dir.clone();

            task::spawn_blocking(move || build_binary(&source_dir, &runtime_dir, &binary_path))
                .await
                .context("РЎРѓР В±Р С•РЎР‚Р С”Р В° yggstack task panicked")??;

            return Ok(());
        }

        Err(anyhow!(
            "bundled yggstack binary Р Р…Р Вµ Р Р…Р В°Р в„–Р Т‘Р ВµР Р…. Р вЂќР С•Р В±Р В°Р Р†РЎРЉРЎвЂљР Вµ yggstack.exe Р Р† РЎР‚Р ВµР В»Р С‘Р В· Р С‘Р В»Р С‘ РЎС“Р С”Р В°Р В¶Р С‘РЎвЂљР Вµ MC_YGGSTACK_SOURCE_DIR Р Т‘Р В»РЎРЏ dev-РЎРѓР В±Р С•РЎР‚Р С”Р С‘"
        ))
    }

    #[allow(dead_code)]
    async fn generate_config_if_missing(&self) -> Result<()> {
        if self.config.config_path.exists() {
            return Ok(());
        }

        let binary_path = self.config.binary_path.clone();
        let config_path = self.config.config_path.clone();

        task::spawn_blocking(move || generate_config(&binary_path, &config_path))
            .await
            .context("Р С–Р ВµР Р…Р ВµРЎР‚Р В°РЎвЂ Р С‘РЎРЏ Р С”Р С•Р Р…РЎвЂћР С‘Р С–Р В° yggstack task panicked")??;

        Ok(())
    }
}

fn build_binary(source_dir: &Path, runtime_dir: &Path, binary_path: &Path) -> Result<()> {
    fs::create_dir_all(runtime_dir).with_context(|| {
        format!(
            "Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ РЎРѓР С•Р В·Р Т‘Р В°РЎвЂљРЎРЉ runtime Р С”Р В°РЎвЂљР В°Р В»Р С•Р С– Р Т‘Р В»РЎРЏ yggstack {}",
            runtime_dir.display()
        )
    })?;

    let mut command = Command::new("go");
    command
        .arg("build")
        .arg("-o")
        .arg(binary_path)
        .arg("./cmd/yggstack")
        .current_dir(source_dir);

    let status = command
        .configure_for_background()
        .status()
        .with_context(|| format!("Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ Р В·Р В°Р С—РЎС“РЎРѓРЎвЂљР С‘РЎвЂљРЎРЉ go build Р Р† {}", source_dir.display()))?;

    if !status.success() {
        return Err(anyhow!(
            "go build Р В·Р В°Р Р†Р ВµРЎР‚РЎв‚¬Р С‘Р В»РЎРѓРЎРЏ РЎРѓ Р С•РЎв‚¬Р С‘Р В±Р С”Р С•Р в„– Р С—РЎР‚Р С‘ РЎРѓР В±Р С•РЎР‚Р С”Р Вµ yggstack Р Р† {}",
            source_dir.display()
        ));
    }

    Ok(())
}

fn generate_config(binary_path: &Path, config_path: &Path) -> Result<()> {
    let mut command = Command::new(binary_path);
    command.arg("-genconf").arg("-json");

    let output = command
        .configure_for_background()
        .output()
        .with_context(|| format!("Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ Р В·Р В°Р С—РЎС“РЎРѓРЎвЂљР С‘РЎвЂљРЎРЉ {}", binary_path.display()))?;

    if !output.status.success() {
        return Err(anyhow!(
            "yggstack -genconf Р В·Р В°Р Р†Р ВµРЎР‚РЎв‚¬Р С‘Р В»РЎРѓРЎРЏ РЎРѓ Р С•РЎв‚¬Р С‘Р В±Р С”Р С•Р в„–: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    fs::write(config_path, output.stdout).with_context(|| {
        format!(
            "Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ Р В·Р В°Р С—Р С‘РЎРѓР В°РЎвЂљРЎРЉ РЎРѓР С–Р ВµР Р…Р ВµРЎР‚Р С‘РЎР‚Р С•Р Р†Р В°Р Р…Р Р…РЎвЂ№Р в„– Р С”Р С•Р Р…РЎвЂћР С‘Р С– yggstack Р Р† {}",
            config_path.display()
        )
    })?;

    Ok(())
}

fn generate_or_update_config(binary_path: &Path, config_path: &Path, peers: &[String]) -> Result<()> {
    let mut config_value = if config_path.exists() {
        let raw = fs::read_to_string(config_path).with_context(|| {
            format!("Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ Р С—РЎР‚Р С•РЎвЂЎР С‘РЎвЂљР В°РЎвЂљРЎРЉ Р С”Р С•Р Р…РЎвЂћР С‘Р С– Yggstack {}", config_path.display())
        })?;
        serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_else(|_| json!({}))
    } else {
        generate_config(binary_path, config_path)?;
        let raw = fs::read_to_string(config_path).with_context(|| {
            format!("Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ Р С—РЎР‚Р С•РЎвЂЎР С‘РЎвЂљР В°РЎвЂљРЎРЉ РЎРѓР Р†Р ВµР В¶Р ВµРЎРѓР С•Р В·Р Т‘Р В°Р Р…Р Р…РЎвЂ№Р в„– Р С”Р С•Р Р…РЎвЂћР С‘Р С– Yggstack {}", config_path.display())
        })?;
        serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_else(|_| json!({}))
    };

    if !config_value.is_object() {
        config_value = json!({});
    }

    let object = config_value
        .as_object_mut()
        .ok_or_else(|| anyhow!("yggstack config root must be an object"))?;
    object.insert("Peers".into(), json!(peers));
    object.insert("AdminListen".into(), json!("none"));
    if !object.contains_key("MulticastInterfaces") {
        object.insert(
            "MulticastInterfaces".into(),
            json!([{
                "Regex": ".*",
                "Beacon": true,
                "Listen": true,
                "Password": ""
            }]),
        );
    }

    let normalized = serde_json::to_string_pretty(&config_value)
        .context("Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ РЎРѓР ВµРЎР‚Р С‘Р В°Р В»Р С‘Р В·Р С•Р Р†Р В°РЎвЂљРЎРЉ Yggstack config")?;
    fs::write(config_path, normalized).with_context(|| {
        format!(
            "Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ Р В·Р В°Р С—Р С‘РЎРѓР В°РЎвЂљРЎРЉ Р Р…Р С•РЎР‚Р СР В°Р В»Р С‘Р В·Р С•Р Р†Р В°Р Р…Р Р…РЎвЂ№Р в„– Р С”Р С•Р Р…РЎвЂћР С‘Р С– Yggstack Р Р† {}",
            config_path.display()
        )
    })?;

    Ok(())
}

fn query_sidecar_value(binary_path: &Path, config_path: &Path, flag: &str) -> Result<String> {
    let mut command = Command::new(binary_path);
    command.arg("-useconffile").arg(config_path).arg(flag);

    let output = command
        .configure_for_background()
        .output()
        .with_context(|| format!("Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ Р В·Р В°Р С—РЎС“РЎРѓРЎвЂљР С‘РЎвЂљРЎРЉ {} {}", binary_path.display(), flag))?;

    if !output.status.success() {
        return Err(anyhow!(
            "yggstack {flag} Р В·Р В°Р Р†Р ВµРЎР‚РЎв‚¬Р С‘Р В»РЎРѓРЎРЏ РЎРѓ Р С•РЎв‚¬Р С‘Р В±Р С”Р С•Р в„–: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn normalize_ygg_address(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches('[').trim_matches(']');
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.parse::<std::net::Ipv6Addr>().is_ok() {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn default_runtime_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"G:\minecraftjava\p2p\.runtime"))
        .join("MinecraftP2PConnector")
}

impl YggstackManager {
    async fn ensure_sidecar_config_ready(&self) -> Result<()> {
        let binary_path = self.config.binary_path.clone();
        let config_path = self.config.config_path.clone();
        let bootstrap = fetch_bootstrap_peers().await.unwrap_or_else(|error| BootstrapPeers {
            peers: YGG_FALLBACK_PEERS.iter().map(|peer| (*peer).to_string()).collect(),
            source: format!("built-in fallback ({error:#})"),
        });

        task::spawn_blocking(move || {
            generate_or_update_config(&binary_path, &config_path, &bootstrap.peers)
        })
        .await
        .context("Р В РЎвЂ“Р В Р’ВµР В Р вЂ¦Р В Р’ВµР РЋР вЂљР В Р’В°Р РЋРІР‚В Р В РЎвЂР РЋР РЏ Р В РЎвЂќР В РЎвЂўР В Р вЂ¦Р РЋРІР‚С›Р В РЎвЂР В РЎвЂ“Р В Р’В° yggstack task panicked")??;

        Ok(())
    }

    async fn start_sidecar_with_args(&self, extra_args: &[&str]) -> Result<YggstackRuntimeInfo> {
        self.prepare_runtime().await?;
        self.stop_running_sidecar()?;

        self.ensure_runtime_dir()?;
        let log_file = File::options()
            .create(true)
            .append(true)
            .open(&self.config.log_path)
            .with_context(|| format!("Р В Р вЂ¦Р В Р’Вµ Р РЋРЎвЂњР В РўвЂР В Р’В°Р В Р’В»Р В РЎвЂўР РЋР С“Р РЋР Р‰ Р В РЎвЂўР РЋРІР‚С™Р В РЎвЂќР РЋР вЂљР РЋРІР‚в„–Р РЋРІР‚С™Р РЋР Р‰ Р В Р’В»Р В РЎвЂўР В РЎвЂ“ {}", self.config.log_path.display()))?;
        let err_file = log_file
            .try_clone()
            .context("Р В Р вЂ¦Р В Р’Вµ Р РЋРЎвЂњР В РўвЂР В Р’В°Р В Р’В»Р В РЎвЂўР РЋР С“Р РЋР Р‰ Р В РЎвЂќР В Р’В»Р В РЎвЂўР В Р вЂ¦Р В РЎвЂР РЋР вЂљР В РЎвЂўР В Р вЂ Р В Р’В°Р РЋРІР‚С™Р РЋР Р‰ Р РЋРІР‚С›Р В Р’В°Р В РІвЂћвЂ“Р В Р’В»Р В РЎвЂўР В Р вЂ Р РЋРІР‚в„–Р В РІвЂћвЂ“ Р В РўвЂР В Р’ВµР РЋР С“Р В РЎвЂќР РЋР вЂљР В РЎвЂР В РЎвЂ”Р РЋРІР‚С™Р В РЎвЂўР РЋР вЂљ Р В Р’В»Р В РЎвЂўР В РЎвЂ“Р В Р’В° yggstack")?;

        let mut command = Command::new(&self.config.binary_path);
        command
            .arg("-useconffile")
            .arg(&self.config.config_path)
            .arg("-logto")
            .arg(&self.config.log_path)
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(err_file));
        for arg in extra_args {
            command.arg(arg);
        }
        let child = command
            .configure_for_background()
            .spawn()
            .with_context(|| {
                format!(
                    "Р В Р вЂ¦Р В Р’Вµ Р РЋРЎвЂњР В РўвЂР В Р’В°Р В Р’В»Р В РЎвЂўР РЋР С“Р РЋР Р‰ Р В Р’В·Р В Р’В°Р В РЎвЂ”Р РЋРЎвЂњР РЋР С“Р РЋРІР‚С™Р В РЎвЂР РЋРІР‚С™Р РЋР Р‰ yggstack Р В РЎвЂР В Р’В· {}",
                    self.config.binary_path.display()
                )
            })?;

        *self
            .process
            .lock()
            .map_err(|_| anyhow!("mutex yggstack process poisoned"))? = Some(ManagedProcess { child });

        Ok(self.sidecar_runtime_info())
    }

    fn stop_running_sidecar(&self) -> Result<()> {
        if let Some(mut process) = self
            .process
            .lock()
            .map_err(|_| anyhow!("mutex yggstack process poisoned"))?
            .take()
        {
            let _ = process.child.kill();
            let _ = process.child.wait();
        }
        Ok(())
    }

    async fn build_embedded_config(&self) -> Result<(String, usize, String)> {
        self.ensure_runtime_dir()?;

        let bootstrap = match fetch_bootstrap_peers().await {
            Ok(peers) => peers,
            Err(error) => BootstrapPeers {
                peers: YGG_FALLBACK_PEERS.iter().map(|peer| (*peer).to_string()).collect(),
                source: format!("built-in fallback ({error:#})"),
            },
        };

        let mut config_value = if self.config.config_path.exists() {
            let raw = fs::read_to_string(&self.config.config_path).with_context(|| {
                format!(
                    "РЅРµ СѓРґР°Р»РѕСЃСЊ РїСЂРѕС‡РёС‚Р°С‚СЊ РєРѕРЅС„РёРі Yggstack {}",
                    self.config.config_path.display()
                )
            })?;
            serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        if !config_value.is_object() {
            config_value = json!({});
        }

        let object = config_value
            .as_object_mut()
            .ok_or_else(|| anyhow!("embedded ygg config root must be an object"))?;
        object.insert("Peers".into(), json!(bootstrap.peers));
        object.insert("AdminListen".into(), json!("none"));
        if !object.contains_key("MulticastInterfaces") {
            object.insert(
                "MulticastInterfaces".into(),
                json!([{
                    "Regex": ".*",
                    "Beacon": true,
                    "Listen": true,
                    "Password": ""
                }]),
            );
        }

        let config_json = serde_json::to_string_pretty(&config_value)
            .context("РЅРµ СѓРґР°Р»РѕСЃСЊ СЃРµСЂРёР°Р»РёР·РѕРІР°С‚СЊ embedded Yggstack config")?;
        fs::write(&self.config.config_path, &config_json).with_context(|| {
            format!(
                "РЅРµ СѓРґР°Р»РѕСЃСЊ Р·Р°РїРёСЃР°С‚СЊ embedded-РєРѕРЅС„РёРі Yggstack РІ {}",
                self.config.config_path.display()
            )
        })?;

        Ok((config_json, bootstrap.peers.len(), bootstrap.source))
    }
}

fn embedded_peer_count(status: &EmbeddedBridgeStatus) -> Option<usize> {
    let raw = status.peers_json.as_deref()?;
    let value = serde_json::from_str::<serde_json::Value>(raw).ok()?;
    value.as_array().map(|items| items.len())
}

async fn fetch_bootstrap_peers() -> Result<BootstrapPeers> {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .build()
        .context("РЅРµ СѓРґР°Р»РѕСЃСЊ СЃРѕР·РґР°С‚СЊ HTTP-РєР»РёРµРЅС‚ РґР»СЏ Ygg bootstrap")?;

    let mut last_error = None;
    for feed in YGG_PUBLIC_PEER_FEEDS {
        match fetch_bootstrap_peers_from_feed(&client, feed).await {
            Ok(peers) if !peers.is_empty() => {
                return Ok(BootstrapPeers {
                    peers,
                    source: (*feed).to_string(),
                })
            }
            Ok(_) => {
                last_error = Some(anyhow!("feed returned zero usable peers"));
            }
            Err(error) => {
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("no Ygg public peer feeds responded")))
}

async fn fetch_bootstrap_peers_from_feed(
    client: &reqwest::Client,
    feed_url: &str,
) -> Result<Vec<String>> {
    let payload = client
        .get(feed_url)
        .header(reqwest::header::USER_AGENT, "minecraft-p2p-connector/0.3.10")
        .send()
        .await
        .with_context(|| format!("РЅРµ СѓРґР°Р»РѕСЃСЊ Р·Р°РїСЂРѕСЃРёС‚СЊ Ygg public peer feed {feed_url}"))?
        .error_for_status()
        .with_context(|| format!("Ygg public peer feed {feed_url} РІРµСЂРЅСѓР» РѕС€РёР±РєСѓ"))?
        .json::<HashMap<String, HashMap<String, YggPublicPeerMeta>>>()
        .await
        .with_context(|| format!("РЅРµ СѓРґР°Р»РѕСЃСЊ СЂР°Р·РѕР±СЂР°С‚СЊ JSON Ygg public peer feed {feed_url}"))?;

    let mut ranked = Vec::new();
    for peers in payload.into_values() {
        for (uri, meta) in peers {
            if !meta.up.unwrap_or(false) {
                continue;
            }
            ranked.push((peer_uri_priority(&uri), meta.response_ms.unwrap_or(u64::MAX), uri));
        }
    }

    ranked.sort_by(|left, right| left.cmp(right));
    let mut selected = Vec::new();
    for (_, _, uri) in ranked {
        if selected.iter().any(|existing| existing == &uri) {
            continue;
        }
        selected.push(uri);
        if selected.len() >= 8 {
            break;
        }
    }

    Ok(selected)
}

fn peer_uri_priority(uri: &str) -> (u8, u8) {
    if uri.starts_with("tls://") && uri.contains(":443") {
        return (0, 0);
    }
    if uri.starts_with("wss://") && uri.contains(":443") {
        return (0, 1);
    }
    if uri.starts_with("tls://") {
        return (1, 0);
    }
    if uri.starts_with("wss://") {
        return (1, 1);
    }
    if uri.starts_with("tcp://") {
        return (2, 0);
    }
    if uri.starts_with("quic://") && uri.contains(":443") {
        return (2, 1);
    }
    if uri.starts_with("quic://") {
        return (3, 0);
    }
    if uri.starts_with("ws://") {
        return (4, 0);
    }
    (9, 0)
}

fn yggstack_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "yggstack.exe"
    } else {
        "yggstack"
    }
}

fn resolve_bundled_binary_path() -> Option<PathBuf> {
    if let Ok(value) = env::var("MC_YGGSTACK_BINARY") {
        let path = PathBuf::from(value);
        if path.exists() {
            return Some(path);
        }
    }

    let mut candidates = Vec::new();

    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("resources").join("yggstack").join(yggstack_binary_name()));
            candidates.push(exe_dir.join("yggstack").join(yggstack_binary_name()));
            candidates.push(exe_dir.join(yggstack_binary_name()));
        }
    }

    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join("resources").join("yggstack").join(yggstack_binary_name()));
        candidates.push(current_dir.join("yggstack").join(yggstack_binary_name()));
    }

    candidates.into_iter().find(|path| path.exists())
}

fn copy_bundled_binary(source: &Path, target: &Path) -> Result<()> {
    if source == target {
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ РЎРѓР С•Р В·Р Т‘Р В°РЎвЂљРЎРЉ Р С”Р В°РЎвЂљР В°Р В»Р С•Р С– Р Т‘Р В»РЎРЏ yggstack {}", parent.display())
        })?;
    }

    fs::copy(source, target).with_context(|| {
        format!(
            "Р Р…Р Вµ РЎС“Р Т‘Р В°Р В»Р С•РЎРѓРЎРЉ РЎРѓР С”Р С•Р С—Р С‘РЎР‚Р С•Р Р†Р В°РЎвЂљРЎРЉ bundled yggstack Р С‘Р В· {} Р Р† {}",
            source.display(),
            target.display()
        )
    })?;

    Ok(())
}

#[cfg(all(target_os = "windows", not(embedded_ygg)))]
fn write_embedded_binary(target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("не удалось создать каталог для yggstack {}", parent.display())
        })?;
    }

    fs::write(target, EMBEDDED_YGGSTACK_EXE).with_context(|| {
        format!(
            "не удалось записать встроенный yggstack binary в {}",
            target.display()
        )
    })?;

    Ok(())
}

trait CommandBackgroundExt {
    fn configure_for_background(&mut self) -> &mut Self;
}

impl CommandBackgroundExt for Command {
    fn configure_for_background(&mut self) -> &mut Self {
        #[cfg(target_os = "windows")]
        {
            self.creation_flags(CREATE_NO_WINDOW);
        }
        self
    }
}

#[cfg(embedded_ygg)]
fn embedded_bridge_available() -> bool {
    true
}

#[cfg(not(embedded_ygg))]
fn embedded_bridge_available() -> bool {
    false
}

#[cfg(embedded_ygg)]
fn embedded_bridge_status() -> Result<EmbeddedBridgeStatus> {
    unsafe { call_bridge(ffi::YggstackBridgeStatus) }
}

#[cfg(embedded_ygg)]
fn embedded_bridge_start_json(config_json: &str) -> Result<EmbeddedBridgeStatus> {
    let config_json = std::ffi::CString::new(config_json)
        .context("embedded Ygg config contains an unexpected NUL byte")?;
    unsafe { call_bridge_with_string(ffi::YggstackBridgeStartJSON, &config_json) }
}

#[cfg(not(embedded_ygg))]
fn embedded_bridge_start_json(_config_json: &str) -> Result<EmbeddedBridgeStatus> {
    Err(anyhow!("embedded Yggstack bridge is not compiled into this build"))
}

#[cfg(not(embedded_ygg))]
fn embedded_bridge_status() -> Result<EmbeddedBridgeStatus> {
    Err(anyhow!("embedded Yggstack bridge is not compiled into this build"))
}

#[cfg(embedded_ygg)]
fn embedded_bridge_retry_peers() -> Result<EmbeddedBridgeStatus> {
    unsafe { call_bridge(ffi::YggstackBridgeRetryPeers) }
}

#[cfg(not(embedded_ygg))]
fn embedded_bridge_retry_peers() -> Result<EmbeddedBridgeStatus> {
    Err(anyhow!("embedded Yggstack bridge is not compiled into this build"))
}

#[cfg(embedded_ygg)]
fn embedded_bridge_stop() -> Result<EmbeddedBridgeStatus> {
    unsafe { call_bridge(ffi::YggstackBridgeStop) }
}

#[cfg(not(embedded_ygg))]
fn embedded_bridge_stop() -> Result<EmbeddedBridgeStatus> {
    Err(anyhow!("embedded Yggstack bridge is not compiled into this build"))
}

#[cfg(embedded_ygg)]
unsafe fn call_bridge(function: unsafe extern "C" fn() -> *mut c_char) -> Result<EmbeddedBridgeStatus> {
    let ptr = function();
    if ptr.is_null() {
        return Err(anyhow!("embedded Yggstack bridge returned a null pointer"));
    }

    let raw = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    ffi::YggstackBridgeFreeString(ptr);

    let status: EmbeddedBridgeStatus =
        serde_json::from_str(&raw).with_context(|| format!("invalid embedded Yggstack JSON: {raw}"))?;
    Ok(status)
}

#[cfg(embedded_ygg)]
unsafe fn call_bridge_with_string(
    function: unsafe extern "C" fn(*const c_char) -> *mut c_char,
    input: &std::ffi::CString,
) -> Result<EmbeddedBridgeStatus> {
    let ptr = function(input.as_ptr());
    if ptr.is_null() {
        return Err(anyhow!("embedded Yggstack bridge returned a null pointer"));
    }

    let raw = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    ffi::YggstackBridgeFreeString(ptr);

    let status: EmbeddedBridgeStatus =
        serde_json::from_str(&raw).with_context(|| format!("invalid embedded Yggstack JSON: {raw}"))?;
    Ok(status)
}

#[cfg(embedded_ygg)]
mod ffi {
    use std::os::raw::c_char;

    unsafe extern "C" {
        pub fn YggstackBridgeStartJSON(config: *const c_char) -> *mut c_char;
        pub fn YggstackBridgeStatus() -> *mut c_char;
        pub fn YggstackBridgeRetryPeers() -> *mut c_char;
        pub fn YggstackBridgeStop() -> *mut c_char;
        pub fn YggstackBridgeFreeString(ptr: *mut c_char);
    }
}

