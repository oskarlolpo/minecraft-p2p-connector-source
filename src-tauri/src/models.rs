use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SessionMode {
    #[default]
    Idle,
    Host,
    Client,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    #[default]
    Idle,
    Starting,
    WaitingForPeer,
    Punching,
    Connecting,
    Hosting,
    Connected,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum TransportKind {
    #[default]
    Unknown,
    Direct,
    DirectQuic,
    CloudflareWebrtc,
    Relay,
    AblyRelay,
    ReverseTunnel,
    MeshFallback,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LocalTargetState {
    #[default]
    Unknown,
    Reachable,
    Unreachable,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PeerInfo {
    pub peer_id: String,
    pub addr: String,
    pub connected: bool,
    pub ping_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SwarmBootstrap {
    pub peer_id: String,
    pub listen_addrs: Vec<String>,
    pub relay_addrs: Vec<String>,
    pub nat_status: String,
    pub local_game_port: Option<u16>,
    pub transport_preference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStatus {
    pub mode: SessionMode,
    pub state: ConnectionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udp_bind_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_udp_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_game_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minecraft_version: Option<String>,
    pub transport_kind: TransportKind,
    pub local_target_state: LocalTargetState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_preference: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cloudflare_enabled: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cloudflare_turn_ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloudflare_turn_endpoint: Option<String>,
    pub password_protected: bool,
    pub peer_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub peers: Vec<PeerInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub signaling_server: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<String>,
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self {
            mode: SessionMode::Idle,
            state: ConnectionState::Idle,
            room_code: None,
            udp_bind_addr: None,
            public_udp_addr: None,
            local_game_port: None,
            minecraft_version: None,
            transport_kind: TransportKind::Unknown,
            local_target_state: LocalTargetState::Unknown,
            transport_path: None,
            transport_preference: None,
            cloudflare_enabled: false,
            cloudflare_turn_ready: false,
            cloudflare_turn_endpoint: None,
            password_protected: false,
            peer_count: 0,
            peers: Vec::new(),
            note: None,
            last_error: None,
            signaling_server: String::new(),
            logs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreflightReport {
    pub local_port: u16,
    pub reachable: bool,
    pub state: LocalTargetState,
    pub minecraft_version: Option<String>,
    pub recommended_host_action: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TestServerInfo {
    pub bind_addr: String,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSnapshot {
    pub exported_at: String,
    pub role: SessionMode,
    pub status: NetworkStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preflight: Option<PreflightReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_server: Option<TestServerInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_checks: Option<NetworkChecks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_attempt: Option<TransportAttempt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloudflare_attempt: Option<CloudflareAttempt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yggstack_runtime: Option<YggstackRuntimeInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_transport: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkChecks {
    pub ably_tcp: CheckResult,
    pub system_dns: CheckResult,
    pub fallback_dns: CheckResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloudflare_https: Option<CheckResult>,
    pub turn_udp: CheckResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransportAttempt {
    pub transport: String,
    pub success: bool,
    pub detail: String,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareAttempt {
    pub transport: String,
    pub success: bool,
    pub detail: String,
    pub credential_status: String,
    pub selected_candidate_pair: Option<String>,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareRuntimeInfo {
    pub ready: bool,
    pub credential_endpoint: Option<String>,
    pub turn_endpoint: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct YggstackRuntimeInfo {
    pub ready: bool,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ygg_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ygg_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ygg_subnet: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub nickname: String,
    pub avatar_data_url: Option<String>,
    pub theme: String,
    pub language: String,
    pub overlay_shortcut: String,
}
