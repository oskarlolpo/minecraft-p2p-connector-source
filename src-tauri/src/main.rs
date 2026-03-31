#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod models;
mod network;

use models::{NetworkStatus, SwarmBootstrap};
use network::network_swarm::NetworkSwarmManager;
use tauri::{AppHandle, State};

#[derive(Clone)]
struct AppState {
    manager: NetworkSwarmManager,
}

#[tauri::command]
async fn start_hosting(
    app: AppHandle,
    state: State<'_, AppState>,
    room_name: String,
    password: Option<String>,
    local_port: u16,
) -> Result<SwarmBootstrap, String> {
    state
        .manager
        .start_hosting(app, room_name, password, local_port)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn stop_hosting(state: State<'_, AppState>) -> Result<(), String> {
    state
        .manager
        .stop_hosting()
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn connect_to_peer(
    app: AppHandle,
    state: State<'_, AppState>,
    peer_id: String,
    peer_addrs: Vec<String>,
) -> Result<(), String> {
    state
        .manager
        .connect_to_peer(app, peer_id, peer_addrs)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn kick_peer(state: State<'_, AppState>, peer_id: String) -> Result<(), String> {
    state
        .manager
        .kick_peer(peer_id)
        .await
        .map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn get_status(state: State<'_, AppState>) -> Result<NetworkStatus, String> {
    Ok(state.manager.get_status().await)
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "minecraft_p2p_connector=info,libp2p=warn".into()),
        )
        .init();

    tauri::Builder::default()
        .manage(AppState {
            manager: NetworkSwarmManager::new(),
        })
        .invoke_handler(tauri::generate_handler![
            start_hosting,
            stop_hosting,
            connect_to_peer,
            kick_peer,
            get_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
