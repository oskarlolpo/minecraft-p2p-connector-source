use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, UdpSocket},
    sync::{mpsc, Mutex},
};

use crate::signaling::SignalingConfig;

#[derive(Clone)]
struct AppState {
    shared: Arc<Mutex<SharedState>>,
}

#[derive(Default)]
struct SharedState {
    rooms: HashMap<String, Room>,
    peers: HashMap<String, PeerSession>,
    tokens: HashMap<String, String>,
}

#[derive(Clone, Default)]
struct Room {
    host_id: String,
    clients: HashSet<String>,
    announced_clients: HashSet<String>,
}

struct PeerSession {
    peer_id: String,
    room_code: String,
    role: PeerRole,
    udp_token: String,
    server_cert: Option<String>,
    udp_addr: Option<SocketAddr>,
    sender: mpsc::UnboundedSender<Message>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PeerRole {
    Host,
    Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientSignal {
    CreateRoom {
        peer_id: String,
        udp_token: String,
        server_cert: String,
    },
    JoinRoom {
        peer_id: String,
        room_code: String,
        udp_token: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerSignal {
    RoomCreated {
        room_code: String,
    },
    PeerReady {
        room_code: String,
        peer_id: String,
        peer_addr: String,
        peer_cert: Option<String>,
        role: String,
    },
    PeerLeft {
        peer_id: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct UdpRegistration {
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UdpAck {
    ok: bool,
    token: String,
    observed_addr: String,
}

pub async fn run_local_signaling(config: SignalingConfig) -> Result<()> {
    let state = AppState {
        shared: Arc::new(Mutex::new(SharedState::default())),
    };

    let ws_addr = socket_addr_from_ws(&config.ws_url)?;
    let udp_addr = config.udp_addr;

    let udp_state = state.clone();
    tokio::spawn(async move {
        let _ = run_udp_server(udp_state, udp_addr).await;
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state.clone());

    let listener = match TcpListener::bind(ws_addr).await {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => return Ok(()),
        Err(error) => {
            return Err(error).context("failed to bind embedded signaling websocket");
        }
    };

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

async fn handle_socket(state: AppState, socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if sender.send(message).await.is_err() {
                break;
            }
        }
    });

    let mut current_peer_id = None::<String>;

    if let Some(Ok(first)) = receiver.next().await {
        let message = match first {
            Message::Text(text) => serde_json::from_str::<ClientSignal>(text.as_ref()).ok(),
            Message::Binary(bytes) => serde_json::from_slice::<ClientSignal>(bytes.as_ref()).ok(),
            _ => None,
        };

        if let Some(message) = message {
            match message {
                ClientSignal::CreateRoom {
                    peer_id,
                    udp_token,
                    server_cert,
                } => {
                    current_peer_id = Some(peer_id.clone());
                    let room_code =
                        register_host(&state, &peer_id, udp_token, server_cert, tx.clone()).await;
                    send_json(&tx, &ServerSignal::RoomCreated { room_code });
                }
                ClientSignal::JoinRoom {
                    peer_id,
                    room_code,
                    udp_token,
                } => {
                    current_peer_id = Some(peer_id.clone());
                    match register_client(&state, &peer_id, &room_code, udp_token, tx.clone()).await
                    {
                        Ok(()) => dispatch_ready_for_room(&state, &room_code).await,
                        Err(message) => send_json(&tx, &ServerSignal::Error { message }),
                    }
                }
            }
        }
    }

    while let Some(message) = receiver.next().await {
        match message {
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }

    if let Some(peer_id) = current_peer_id {
        remove_peer(&state, &peer_id).await;
    }

    send_task.abort();
}

async fn run_udp_server(state: AppState, bind_addr: SocketAddr) -> Result<()> {
    let socket = match UdpSocket::bind(bind_addr).await {
        Ok(socket) => socket,
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => return Ok(()),
        Err(error) => return Err(error).context("failed to bind embedded signaling UDP"),
    };
    let mut buffer = [0u8; 2048];

    loop {
        let (size, peer_addr) = socket.recv_from(&mut buffer).await?;
        let registration: UdpRegistration = match serde_json::from_slice(&buffer[..size]) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let room_to_dispatch = {
            let mut shared = state.shared.lock().await;
            let Some(peer_id) = shared.tokens.get(&registration.token).cloned() else {
                let ack = serde_json::to_vec(&UdpAck {
                    ok: false,
                    token: registration.token.clone(),
                    observed_addr: peer_addr.to_string(),
                })?;
                socket.send_to(&ack, peer_addr).await?;
                continue;
            };

            let Some(peer) = shared.peers.get_mut(&peer_id) else {
                continue;
            };

            peer.udp_addr = Some(peer_addr);
            peer.room_code.clone()
        };

        let ack = serde_json::to_vec(&UdpAck {
            ok: true,
            token: registration.token,
            observed_addr: peer_addr.to_string(),
        })?;
        socket.send_to(&ack, peer_addr).await?;
        dispatch_ready_for_room(&state, &room_to_dispatch).await;
    }
}

async fn register_host(
    state: &AppState,
    peer_id: &str,
    udp_token: String,
    server_cert: String,
    sender: mpsc::UnboundedSender<Message>,
) -> String {
    let mut shared = state.shared.lock().await;
    let room_code = generate_room_code(&shared.rooms);
    shared.tokens.insert(udp_token.clone(), peer_id.into());
    shared.peers.insert(
        peer_id.into(),
        PeerSession {
            peer_id: peer_id.into(),
            room_code: room_code.clone(),
            role: PeerRole::Host,
            udp_token,
            server_cert: Some(server_cert),
            udp_addr: None,
            sender,
        },
    );
    shared.rooms.insert(
        room_code.clone(),
        Room {
            host_id: peer_id.into(),
            clients: HashSet::new(),
            announced_clients: HashSet::new(),
        },
    );
    room_code
}

async fn register_client(
    state: &AppState,
    peer_id: &str,
    room_code: &str,
    udp_token: String,
    sender: mpsc::UnboundedSender<Message>,
) -> std::result::Result<(), String> {
    let mut shared = state.shared.lock().await;
    if !shared.rooms.contains_key(room_code) {
        return Err("room code not found".into());
    }

    shared.tokens.insert(udp_token.clone(), peer_id.into());
    shared.peers.insert(
        peer_id.into(),
        PeerSession {
            peer_id: peer_id.into(),
            room_code: room_code.into(),
            role: PeerRole::Client,
            udp_token,
            server_cert: None,
            udp_addr: None,
            sender,
        },
    );
    if let Some(room) = shared.rooms.get_mut(room_code) {
        room.clients.insert(peer_id.into());
    }
    Ok(())
}

async fn dispatch_ready_for_room(state: &AppState, room_code: &str) {
    let notifications = {
        let mut shared = state.shared.lock().await;
        let Some(room_snapshot) = shared.rooms.get(room_code).cloned() else {
            return;
        };
        let Some(host) = shared.peers.get(&room_snapshot.host_id) else {
            return;
        };
        let Some(host_addr) = host.udp_addr else {
            return;
        };

        let host_peer_id = host.peer_id.clone();
        let host_cert = host.server_cert.clone();
        let host_sender = host.sender.clone();

        let mut ready_client_ids = Vec::new();
        let mut notifications = Vec::new();

        for client_id in &room_snapshot.clients {
            if room_snapshot.announced_clients.contains(client_id) {
                continue;
            }
            let Some(client) = shared.peers.get(client_id) else {
                continue;
            };
            let Some(client_addr) = client.udp_addr else {
                continue;
            };

            ready_client_ids.push(client_id.clone());
            notifications.push((
                host_sender.clone(),
                ServerSignal::PeerReady {
                    room_code: room_code.into(),
                    peer_id: client.peer_id.clone(),
                    peer_addr: client_addr.to_string(),
                    peer_cert: None,
                    role: "client".into(),
                },
            ));
            notifications.push((
                client.sender.clone(),
                ServerSignal::PeerReady {
                    room_code: room_code.into(),
                    peer_id: host_peer_id.clone(),
                    peer_addr: host_addr.to_string(),
                    peer_cert: host_cert.clone(),
                    role: "host".into(),
                },
            ));
        }

        if let Some(room) = shared.rooms.get_mut(room_code) {
            for client_id in ready_client_ids {
                room.announced_clients.insert(client_id);
            }
        }

        notifications
    };

    for (sender, message) in notifications {
        send_json(&sender, &message);
    }
}

async fn remove_peer(state: &AppState, peer_id: &str) {
    let notifications = {
        let mut shared = state.shared.lock().await;
        let Some(peer) = shared.peers.remove(peer_id) else {
            return;
        };
        shared.tokens.remove(&peer.udp_token);

        match peer.role {
            PeerRole::Host => {
                let Some(room) = shared.rooms.remove(&peer.room_code) else {
                    return;
                };
                let mut notifications = Vec::new();
                for client_id in room.clients {
                    if let Some(client) = shared.peers.remove(&client_id) {
                        shared.tokens.remove(&client.udp_token);
                        notifications.push((
                            client.sender.clone(),
                            ServerSignal::PeerLeft {
                                peer_id: peer.peer_id.clone(),
                            },
                        ));
                    }
                }
                notifications
            }
            PeerRole::Client => {
                let mut notifications = Vec::new();
                let host_id = if let Some(room) = shared.rooms.get_mut(&peer.room_code) {
                    room.clients.remove(peer_id);
                    room.announced_clients.remove(peer_id);
                    Some(room.host_id.clone())
                } else {
                    None
                };
                if let Some(host_id) = host_id {
                    if let Some(host) = shared.peers.get(&host_id) {
                        notifications.push((
                            host.sender.clone(),
                            ServerSignal::PeerLeft {
                                peer_id: peer.peer_id.clone(),
                            },
                        ));
                    }
                }
                notifications
            }
        }
    };

    for (sender, message) in notifications {
        send_json(&sender, &message);
    }
}

fn send_json(sender: &mpsc::UnboundedSender<Message>, message: &ServerSignal) {
    if let Ok(payload) = serde_json::to_string(message) {
        let _ = sender.send(Message::Text(payload.into()));
    }
}

fn generate_room_code(rooms: &HashMap<String, Room>) -> String {
    loop {
        let code = uuid::Uuid::new_v4().simple().to_string()[..6].to_uppercase();
        if !rooms.contains_key(&code) {
            return code;
        }
    }
}

fn socket_addr_from_ws(ws_url: &str) -> Result<SocketAddr> {
    let trimmed = ws_url
        .strip_prefix("ws://")
        .or_else(|| ws_url.strip_prefix("wss://"))
        .unwrap_or(ws_url);
    let host_port = trimmed.split('/').next().unwrap_or(trimmed);
    Ok(host_port.parse()?)
}
