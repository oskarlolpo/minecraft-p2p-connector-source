use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, MqttOptions, Outgoing, QoS, Transport};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot, Mutex},
    task::JoinHandle,
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::proxy;

const DEFAULT_ABLY_API_KEY: &str = "aGkPAA.1VHkjw:Bai-67g05FcqHdfVOMiSfjYlK3aLz8wOzj5WeTgz4cw";
const DEFAULT_MQTT_HOST: &str = "main.mqtt.ably.net";
const DEFAULT_MQTT_PORT: u16 = 8883;
const DEFAULT_TOPIC_PREFIX: &str = "minecraft-p2p-relay";
const MQTT_REQUEST_CAPACITY: usize = 256;
const TCP_WRITE_QUEUE_CAPACITY: usize = 64;
const TCP_CHUNK_SIZE: usize = 16 * 1024;
const RELAY_READY_TIMEOUT_MS: u64 = 8_000;
const RELAY_RECONNECT_DELAY_MS: u64 = 1_250;

const FRAME_OPEN: u8 = 1;
const FRAME_DATA: u8 = 2;
const FRAME_CLOSE: u8 = 3;
const FRAME_HELLO: u8 = 4;
const FRAME_READY: u8 = 5;
const FRAME_ERROR: u8 = 6;

type StreamMap = Arc<Mutex<HashMap<u64, mpsc::Sender<Vec<u8>>>>>;

#[derive(Clone)]
pub struct RelayConfig {
    mqtt_host: String,
    mqtt_port: u16,
    username: String,
    password: String,
    topic_prefix: String,
}

pub struct RelayRuntime {
    join_handle: JoinHandle<Result<()>>,
}

#[derive(Clone)]
struct RelayTopics {
    client_to_host: String,
    host_to_client: String,
}

struct RelayFrame<'a> {
    kind: u8,
    stream_id: u64,
    payload: &'a [u8],
}

impl RelayConfig {
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ABLY_API_KEY")
            .or_else(|_| std::env::var("MC_ABLY_API_KEY"))
            .unwrap_or_else(|_| DEFAULT_ABLY_API_KEY.into());
        let (username, password) = api_key
            .split_once(':')
            .ok_or_else(|| anyhow!("invalid Ably API key format"))?;

        Ok(Self {
            mqtt_host: std::env::var("MC_RELAY_MQTT_HOST")
                .unwrap_or_else(|_| DEFAULT_MQTT_HOST.into()),
            mqtt_port: std::env::var("MC_RELAY_MQTT_PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_MQTT_PORT),
            username: username.to_string(),
            password: password.to_string(),
            topic_prefix: std::env::var("MC_RELAY_TOPIC_PREFIX")
                .unwrap_or_else(|_| DEFAULT_TOPIC_PREFIX.into()),
        })
    }
}

impl RelayRuntime {
    pub async fn wait(self) -> Result<()> {
        self.join_handle
            .await
            .context("relay task panicked")?
    }
}

impl RelayTopics {
    fn new(prefix: &str, session_id: &str) -> Self {
        Self {
            client_to_host: format!("{prefix}/{session_id}/c2h"),
            host_to_client: format!("{prefix}/{session_id}/h2c"),
        }
    }
}

pub async fn start_host_runtime(
    config: RelayConfig,
    session_id: String,
    local_game_port: u16,
    cancel: CancellationToken,
) -> Result<RelayRuntime> {
    let topics = RelayTopics::new(&config.topic_prefix, &session_id);
    let streams = Arc::new(Mutex::new(HashMap::<u64, mpsc::Sender<Vec<u8>>>::new()));
    let join_handle = tokio::spawn(async move {
        loop {
            let client_id = format!("mcp2p-host-{}", Uuid::new_v4().simple());
            let (client, mut eventloop) = build_client(&config, client_id);
            client
                .subscribe(topics.client_to_host.clone(), QoS::AtLeastOnce)
                .await
                .context("failed to subscribe host relay topic")?;

            let poll_result: Result<()> = loop {
                let event = tokio::select! {
                    _ = cancel.cancelled() => return Ok(()),
                    event = eventloop.poll() => event,
                };

                match event {
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        handle_host_publish(
                            publish.payload.as_ref(),
                            &client,
                            &topics,
                            streams.clone(),
                            local_game_port,
                            cancel.clone(),
                        )
                        .await?;
                    }
                    Ok(Event::Outgoing(Outgoing::PingReq)) => {}
                    Ok(_) => {}
                    Err(error) => break Err(error).context("host relay MQTT poll failed"),
                }
            };

            if cancel.is_cancelled() {
                break;
            }

            if let Err(error) = poll_result {
                tracing::warn!("host relay reconnect after error: {error:#}");
                tokio::time::sleep(Duration::from_millis(RELAY_RECONNECT_DELAY_MS)).await;
                continue;
            }
        }

        Ok(())
    });

    Ok(RelayRuntime { join_handle })
}

pub async fn start_client_runtime(
    config: RelayConfig,
    session_id: String,
    cancel: CancellationToken,
) -> Result<RelayRuntime> {
    let topics = RelayTopics::new(&config.topic_prefix, &session_id);
    let listener = TcpListener::bind(proxy::MINECRAFT_LOCAL_ADDR)
        .await
        .with_context(|| {
            format!(
                "failed to bind relay listener on {}. Port is already busy",
                proxy::MINECRAFT_LOCAL_ADDR
            )
        })?;

    let streams = Arc::new(Mutex::new(HashMap::<u64, mpsc::Sender<Vec<u8>>>::new()));
    let (ready_tx, ready_rx) = oneshot::channel::<()>();
    let ready = Arc::new(Mutex::new(Some(ready_tx)));

    let accept_topics = topics.clone();
    let accept_cancel = cancel.clone();
    let accept_streams = streams.clone();
    let stream_ids = Arc::new(AtomicU64::new(1));
    let accept_stream_ids = stream_ids.clone();
    let current_client = Arc::new(Mutex::new(None::<AsyncClient>));
    let accept_current_client = current_client.clone();

    let accept_task = tokio::spawn(async move {
        loop {
            let incoming = tokio::select! {
                _ = accept_cancel.cancelled() => break,
                incoming = listener.accept() => incoming,
            };

            let (tcp_stream, _) = incoming.context("relay listener accept failed")?;
            let stream_id = accept_stream_ids.fetch_add(1, Ordering::Relaxed);
            let client = accept_current_client
                .lock()
                .await
                .clone()
                .ok_or_else(|| anyhow!("relay MQTT client is not ready yet"))?;
            start_client_stream(
                tcp_stream,
                stream_id,
                accept_streams.clone(),
                client,
                accept_topics.clone(),
                accept_cancel.clone(),
            )
            .await?;
        }

        Ok(())
    });

    let event_ready = ready.clone();
    let event_streams = streams.clone();
    let event_cancel = cancel.clone();
    let event_config = config.clone();
    let event_topics = topics.clone();
    let event_current_client = current_client.clone();
    let event_task = tokio::spawn(async move {
        loop {
            let client_id = format!("mcp2p-client-{}", Uuid::new_v4().simple());
            let (client, mut eventloop) = build_client(&event_config, client_id);
            *event_current_client.lock().await = Some(client.clone());
            client
                .subscribe(event_topics.host_to_client.clone(), QoS::AtLeastOnce)
                .await
                .context("failed to subscribe client relay topic")?;

            let should_send_hello = event_ready.lock().await.is_some();
            if should_send_hello {
                publish_frame(&client, &event_topics.client_to_host, FRAME_HELLO, 0, &[])
                    .await
                    .context("failed to publish relay hello")?;
            }

            let poll_result: Result<()> = loop {
                let event = tokio::select! {
                    _ = event_cancel.cancelled() => return Ok(()),
                    event = eventloop.poll() => event,
                };

                match event {
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        handle_client_publish(
                            publish.payload.as_ref(),
                            event_streams.clone(),
                            event_ready.clone(),
                        )
                        .await?;
                    }
                    Ok(Event::Outgoing(Outgoing::PingReq)) => {}
                    Ok(_) => {}
                    Err(error) => break Err(error).context("client relay MQTT poll failed"),
                }
            };

            if event_cancel.is_cancelled() {
                break;
            }

            if let Err(error) = poll_result {
                tracing::warn!("client relay reconnect after error: {error:#}");
                tokio::time::sleep(Duration::from_millis(RELAY_RECONNECT_DELAY_MS)).await;
                continue;
            }
        }

        Ok(())
    });

    timeout(Duration::from_millis(RELAY_READY_TIMEOUT_MS), ready_rx)
        .await
        .context("relay ready timed out")?
        .context("relay ready signal dropped")?;

    let join_handle = tokio::spawn(async move {
        let mut event_task = event_task;
        let mut accept_task = accept_task;

        let result = tokio::select! {
            _ = cancel.cancelled() => Ok(()),
            result = &mut event_task => {
                accept_task.abort();
                result.context("client relay event task panicked")?
            }
            result = &mut accept_task => {
                event_task.abort();
                result.context("client relay accept task panicked")?
            }
        };

        event_task.abort();
        accept_task.abort();
        result
    });

    Ok(RelayRuntime { join_handle })
}

fn build_client(config: &RelayConfig, client_id: String) -> (AsyncClient, EventLoop) {
    let mut mqtt = MqttOptions::new(client_id, config.mqtt_host.clone(), config.mqtt_port);
    mqtt.set_keep_alive(Duration::from_secs(30));
    mqtt.set_credentials(config.username.clone(), config.password.clone());
    mqtt.set_transport(Transport::tls_with_default_config());
    AsyncClient::new(mqtt, MQTT_REQUEST_CAPACITY)
}

async fn handle_host_publish(
    raw_payload: &[u8],
    client: &AsyncClient,
    topics: &RelayTopics,
    streams: StreamMap,
    local_game_port: u16,
    cancel: CancellationToken,
) -> Result<()> {
    let frame = decode_frame(raw_payload)?;
    match frame.kind {
        FRAME_HELLO => {
            publish_frame(client, &topics.host_to_client, FRAME_READY, 0, &[]).await?;
        }
        FRAME_OPEN => {
            if streams.lock().await.contains_key(&frame.stream_id) {
                return Ok(());
            }

            let target_addr = proxy::minecraft_local_addr(local_game_port);
            match TcpStream::connect(&target_addr).await {
                Ok(tcp_stream) => {
                    let (reader, writer) = tcp_stream.into_split();
                    let (tx, rx) = mpsc::channel::<Vec<u8>>(TCP_WRITE_QUEUE_CAPACITY);
                    streams.lock().await.insert(frame.stream_id, tx);

                    tokio::spawn(pump_channel_to_writer(
                        writer,
                        rx,
                        streams.clone(),
                        frame.stream_id,
                        cancel.clone(),
                    ));
                    tokio::spawn(pump_reader_to_topic(
                        reader,
                        client.clone(),
                        topics.host_to_client.clone(),
                        streams.clone(),
                        frame.stream_id,
                        cancel,
                    ));
                }
                Err(error) => {
                    publish_frame(
                        client,
                        &topics.host_to_client,
                        FRAME_ERROR,
                        frame.stream_id,
                        error.to_string().as_bytes(),
                    )
                    .await?;
                    publish_frame(client, &topics.host_to_client, FRAME_CLOSE, frame.stream_id, &[])
                        .await?;
                }
            }
        }
        FRAME_DATA => {
            if let Some(writer) = streams.lock().await.get(&frame.stream_id).cloned() {
                let _ = writer.send(frame.payload.to_vec()).await;
            }
        }
        FRAME_CLOSE | FRAME_ERROR => {
            streams.lock().await.remove(&frame.stream_id);
        }
        FRAME_READY => {}
        _ => return Err(anyhow!("unknown relay frame kind: {}", frame.kind)),
    }

    Ok(())
}

async fn handle_client_publish(
    raw_payload: &[u8],
    streams: StreamMap,
    ready: Arc<Mutex<Option<oneshot::Sender<()>>>>,
) -> Result<()> {
    let frame = decode_frame(raw_payload)?;
    match frame.kind {
        FRAME_READY => {
            if let Some(sender) = ready.lock().await.take() {
                let _ = sender.send(());
            }
        }
        FRAME_DATA => {
            if let Some(writer) = streams.lock().await.get(&frame.stream_id).cloned() {
                let _ = writer.send(frame.payload.to_vec()).await;
            }
        }
        FRAME_CLOSE | FRAME_ERROR => {
            streams.lock().await.remove(&frame.stream_id);
        }
        FRAME_OPEN | FRAME_HELLO => {}
        _ => return Err(anyhow!("unknown relay frame kind: {}", frame.kind)),
    }

    Ok(())
}

async fn start_client_stream(
    tcp_stream: TcpStream,
    stream_id: u64,
    streams: StreamMap,
    client: AsyncClient,
    topics: RelayTopics,
    cancel: CancellationToken,
) -> Result<()> {
    let (reader, writer) = tcp_stream.into_split();
    let (tx, rx) = mpsc::channel::<Vec<u8>>(TCP_WRITE_QUEUE_CAPACITY);
    streams.lock().await.insert(stream_id, tx);

    publish_frame(&client, &topics.client_to_host, FRAME_OPEN, stream_id, &[])
        .await
        .context("failed to open relay stream")?;

    tokio::spawn(pump_channel_to_writer(
        writer,
        rx,
        streams.clone(),
        stream_id,
        cancel.clone(),
    ));
    tokio::spawn(pump_reader_to_topic(
        reader,
        client,
        topics.client_to_host,
        streams,
        stream_id,
        cancel,
    ));

    Ok(())
}

async fn pump_reader_to_topic<R>(
    mut reader: R,
    client: AsyncClient,
    topic: String,
    streams: StreamMap,
    stream_id: u64,
    cancel: CancellationToken,
) where
    R: AsyncRead + Unpin + Send + 'static,
{
    let mut buffer = vec![0u8; TCP_CHUNK_SIZE];

    loop {
        let read = tokio::select! {
            _ = cancel.cancelled() => break,
            read = reader.read(&mut buffer) => read,
        };

        match read {
            Ok(0) => break,
            Ok(size) => {
                if publish_frame(&client, &topic, FRAME_DATA, stream_id, &buffer[..size])
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let _ = publish_frame(&client, &topic, FRAME_CLOSE, stream_id, &[]).await;
    streams.lock().await.remove(&stream_id);
}

async fn pump_channel_to_writer<W>(
    mut writer: W,
    mut rx: mpsc::Receiver<Vec<u8>>,
    streams: StreamMap,
    stream_id: u64,
    cancel: CancellationToken,
) where
    W: AsyncWrite + Unpin + Send + 'static,
{
    loop {
        let next = tokio::select! {
            _ = cancel.cancelled() => None,
            next = rx.recv() => next,
        };

        let Some(payload) = next else {
            break;
        };

        if writer.write_all(&payload).await.is_err() {
            break;
        }
    }

    let _ = writer.shutdown().await;
    streams.lock().await.remove(&stream_id);
}

async fn publish_frame(
    client: &AsyncClient,
    topic: &str,
    kind: u8,
    stream_id: u64,
    payload: &[u8],
) -> Result<()> {
    client
        .publish(topic, QoS::AtLeastOnce, false, encode_frame(kind, stream_id, payload))
        .await
        .with_context(|| format!("failed to publish relay frame to {topic}"))
}

fn encode_frame(kind: u8, stream_id: u64, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(1 + 8 + payload.len());
    frame.push(kind);
    frame.extend_from_slice(&stream_id.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

fn decode_frame(payload: &[u8]) -> Result<RelayFrame<'_>> {
    if payload.len() < 9 {
        return Err(anyhow!("relay frame is too short"));
    }

    Ok(RelayFrame {
        kind: payload[0],
        stream_id: u64::from_be_bytes(payload[1..9].try_into().unwrap()),
        payload: &payload[9..],
    })
}
