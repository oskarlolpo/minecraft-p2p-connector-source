import * as Ably from "ably";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const ABLY_API_KEY = "aGkPAA.1VHkjw:Bai-67g05FcqHdfVOMiSfjYlK3aLz8wOzj5WeTgz4cw";
const LOBBY_CHANNEL_NAME = "minecraft-lobby";
const DEFAULT_SLOTS = "1/30";
const POLL_INTERVAL_MS = 1500;
const SAFE_RELEASE_STATES = new Set(["initialized", "detached", "failed"]);
const SAFE_SKIP_STATES = new Set(["detached", "failed", "suspended"]);

const modalEl = document.querySelector("#host-modal");
const openHostModalEl = document.querySelector("#open-host-modal");
const closeModalEl = document.querySelector("#close-modal");
const closeModalSecondaryEl = document.querySelector("#close-modal-secondary");
const roomNameEl = document.querySelector("#room-name");
const roomPasswordEl = document.querySelector("#room-password");
const localGamePortEl = document.querySelector("#local-game-port");
const hostButtonEl = document.querySelector("#host-button");
const stopButtonEl = document.querySelector("#stop-button");
const refreshLobbyEl = document.querySelector("#refresh-lobby");
const copyLogsEl = document.querySelector("#copy-logs");
const copySelectedEndpointEl = document.querySelector("#copy-selected-endpoint");
const connectSelectedEl = document.querySelector("#connect-selected");
const serverListEl = document.querySelector("#server-list");
const logsEl = document.querySelector("#logs");
const peerListEl = document.querySelector("#peer-list");
const connectionStateEl = document.querySelector("#connection-state");
const ablyStateEl = document.querySelector("#ably-state");
const lobbyCountEl = document.querySelector("#lobby-count");
const publicEndpointEl = document.querySelector("#public-endpoint");
const selectedServerEl = document.querySelector("#selected-server");
const selectedEndpointEl = document.querySelector("#selected-endpoint");
const selectedMetaEl = document.querySelector("#selected-meta");
const statusNoteEl = document.querySelector("#status-note");
const peerCountEl = document.querySelector("#peer-count");
const minecraftTargetHintEl = document.querySelector("#minecraft-target-hint");
const activeHostCardEl = document.querySelector("#active-host-card");

const hostSession = {
  active: false,
  roomName: "",
  hasPassword: false,
  peerAddr: null,
  localPort: 25565,
  presencePayload: null,
};

const localClientId = ensureClientId();
const state = {
  servers: [],
  selectedServerId: null,
  status: null,
  realtime: null,
  lobbyChannel: null,
  privateChannel: null,
  logBuffer: [],
  syncingPresence: false,
  pendingConnects: new Set(),
  tunnelReady: false,
};

function ensureClientId() {
  const key = "blood-paradise-client-id";
  const existing = localStorage.getItem(key);
  if (existing) {
    return existing;
  }
  const created = `bp-${crypto.randomUUID().slice(0, 8)}`;
  localStorage.setItem(key, created);
  return created;
}

function addLog(message) {
  const stamp = new Date().toLocaleTimeString("ru-RU", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
  state.logBuffer.unshift(`[${stamp}] ${message}`);
  state.logBuffer = state.logBuffer.slice(0, 120);
  renderLogs();
}

function currentLogLines() {
  const combined = [...state.logBuffer];
  if (state.status?.logs?.length) {
    combined.push(...state.status.logs);
  }
  return [...new Set(combined)].slice(0, 100);
}

function renderLogs() {
  const lines = currentLogLines();
  logsEl.innerHTML = lines.length
    ? lines.map((entry) => `<div class="log-entry">${escapeHtml(entry)}</div>`).join("")
    : '<div class="empty-state">Лог пока пуст.</div>';
}

function openModal() {
  modalEl.classList.remove("hidden");
  modalEl.setAttribute("aria-hidden", "false");
  window.setTimeout(() => roomNameEl.focus(), 40);
}

function closeModal() {
  modalEl.classList.add("hidden");
  modalEl.setAttribute("aria-hidden", "true");
}

function setMinecraftHint(text, active = false) {
  minecraftTargetHintEl.textContent = text;
  minecraftTargetHintEl.classList.toggle("active", active);
}

function getSelectedServer() {
  return state.servers.find((server) => server.clientId === state.selectedServerId) ?? null;
}

function renderSelectedServer() {
  const selected = getSelectedServer();
  selectedServerEl.textContent = selected ? selected.roomName : "No selection";
  selectedEndpointEl.textContent = selected?.peerAddr ?? "n/a";
  selectedMetaEl.textContent = selected
    ? `Host: ${selected.hostName}${selected.clientId === localClientId ? " (you)" : ""} · LAN ${selected.localPort} · ${selected.slots}${selected.hasPassword ? " · пароль" : ""}`
    : "Выберите сервер из списка ниже.";
  syncButtons();
}

function syncButtons() {
  const mode = state.status?.mode ?? "idle";
  const busy = ["starting", "connecting", "punching", "waitingForPeer"].includes(
    state.status?.state ?? "idle",
  );
  const isHostMode = mode === "host";
  const selected = getSelectedServer();

  openHostModalEl.disabled = isHostMode || busy;
  hostButtonEl.disabled = isHostMode || busy;
  hostButtonEl.textContent = busy && !isHostMode ? "Запуск..." : "Хостить";
  stopButtonEl.disabled = mode === "idle";
  refreshLobbyEl.disabled = state.realtime?.connection.state === "connecting";
  connectSelectedEl.disabled =
    !selected ||
    selected.clientId === localClientId ||
    busy ||
    isHostMode ||
    state.pendingConnects.has(selected.clientId);
  connectSelectedEl.textContent =
    selected && state.pendingConnects.has(selected.clientId) ? "Connecting..." : "Connect";
  copySelectedEndpointEl.disabled = !selected?.peerAddr;
}

function renderServers() {
  lobbyCountEl.textContent = `${state.servers.length} servers`;
  if (!state.servers.length) {
    serverListEl.innerHTML = '<div class="empty-state">В lobby пока нет активных хостов.</div>';
    renderSelectedServer();
    return;
  }

  serverListEl.innerHTML = state.servers
    .map((server) => {
      const isSelected = state.selectedServerId === server.clientId;
      const isLocal = server.clientId === localClientId;
      const isConnecting = state.pendingConnects.has(server.clientId);
      const icon = server.hasPassword ? "🔒" : "⚔";

      return `
        <article class="server-row ${isSelected ? "active" : ""}">
          <div class="server-main">
            <div class="server-main-top">
              <span class="server-name">${escapeHtml(server.roomName)}</span>
              <span class="badge-lock">${icon}</span>
            </div>
            <div class="server-sub">Host: ${escapeHtml(server.hostName)}${isLocal ? " (you)" : ""}</div>
            <div class="server-sub">${escapeHtml(server.peerAddr ?? "n/a")}</div>
            <div class="server-sub">LAN ${escapeHtml(server.localPort)} · ${escapeHtml(server.slots)}</div>
          </div>
          <div class="server-players">${escapeHtml(server.slots)}</div>
          <div class="server-actions">
            <button class="ghost-button server-mini" data-select-server="${escapeHtml(server.clientId)}">Select</button>
            <button
              class="${isLocal ? "ghost-button" : "primary-button"} server-mini"
              data-connect-server="${escapeHtml(server.clientId)}"
              ${isLocal || isConnecting ? "disabled" : ""}
            >
              ${isLocal ? "Hosting" : isConnecting ? "Connecting..." : "Join"}
            </button>
          </div>
        </article>
      `;
    })
    .join("");

  renderSelectedServer();
}

function renderPeers(peers) {
  peerCountEl.textContent = `${peers?.length ?? 0} peers`;
  if (!peers?.length) {
    peerListEl.innerHTML = '<div class="empty-state">Пока никого нет.</div>';
    return;
  }

  peerListEl.innerHTML = peers
    .map((peer) => {
      const ping = peer.pingMs == null ? "n/a" : `${peer.pingMs} ms`;
      return `
        <div class="peer-row">
          <div class="peer-head">
            <span class="peer-name">${escapeHtml(peer.peerId)}</span>
            <span class="peer-sub">${escapeHtml(peer.addr)}</span>
          </div>
          <div class="peer-side">
            <span class="peer-sub">${peer.connected ? "online" : "pending"}</span>
            <span class="peer-sub">${ping}</span>
          </div>
        </div>
      `;
    })
    .join("");
}

function renderActiveHost() {
  if (!hostSession.active) {
    activeHostCardEl.className = "active-host-card empty";
    activeHostCardEl.innerHTML = `
      <div class="host-card-body">
        <div class="host-thumb">◼</div>
        <div class="host-meta">
          <h2>Хост не запущен</h2>
          <p>Откройте модалку и создайте комнату, чтобы опубликовать сервер в лобби.</p>
        </div>
      </div>
    `;
    return;
  }

  const status = state.status ?? {};
  activeHostCardEl.className = "active-host-card";
  activeHostCardEl.innerHTML = `
    <div class="host-card-body">
      <div class="host-thumb">MC</div>
      <div class="host-meta">
        <h2>${escapeHtml(hostSession.roomName)}</h2>
        <p>${escapeHtml(status.note ?? "Хост активен")}.</p>
        <div class="host-tags">
          <span class="host-tag">Endpoint: ${escapeHtml(hostSession.peerAddr ?? "n/a")}</span>
          <span class="host-tag">LAN: 127.0.0.1:${escapeHtml(hostSession.localPort)}</span>
          <span class="host-tag">${hostSession.hasPassword ? "Пароль включён" : "Без пароля"}</span>
        </div>
      </div>
    </div>
  `;
}

function renderStatus(status) {
  state.status = status;
  connectionStateEl.textContent = formatState(status.state);
  connectionStateEl.dataset.state = status.state ?? "idle";
  ablyStateEl.textContent = state.realtime?.connection.state ?? "offline";
  publicEndpointEl.textContent = status.publicUdpAddr ?? status.udpBindAddr ?? "n/a";
  statusNoteEl.textContent = status.note ?? "Idle";
  renderPeers(status.peers ?? []);
  renderActiveHost();
  renderLogs();
  syncButtons();
}

function formatState(value) {
  const labels = {
    idle: "Idle",
    starting: "Booting",
    waitingForPeer: "Waiting",
    punching: "Punching",
    connecting: "Connecting",
    hosting: "Hosting",
    connected: "Connected",
    error: "Error",
  };
  return labels[value] ?? value ?? "Idle";
}

function hydrateServers(members) {
  state.servers = members
    .map((member) => {
      const data = member.data ?? {};
      return {
        clientId: member.clientId,
        roomName: data.room_name ?? "Unnamed room",
        hostName: data.host_name ?? member.clientId,
        slots: data.slots ?? DEFAULT_SLOTS,
        hasPassword: Boolean(data.has_password),
        peerAddr: data.peer_addr ?? null,
        localPort: data.local_port ?? 25565,
      };
    })
    .filter((server) => Boolean(server.peerAddr));

  if (state.selectedServerId && !state.servers.find((server) => server.clientId === state.selectedServerId)) {
    state.selectedServerId = null;
  }

  if (!state.selectedServerId && state.servers.length === 1) {
    state.selectedServerId = state.servers[0].clientId;
  }

  renderServers();
}

function buildPresencePayload(status) {
  return {
    room_name: hostSession.roomName,
    host_name: localClientId,
    slots: `${Math.max(1, (status?.peerCount ?? 0) + 1)}/30`,
    has_password: hostSession.hasPassword,
    peer_addr: hostSession.peerAddr,
    local_port: hostSession.localPort,
  };
}

function safeShouldSkip(channel) {
  return !channel || SAFE_SKIP_STATES.has(channel.state);
}

async function safePresenceLeave(channel) {
  if (!channel || channel.state !== "attached") {
    return;
  }
  try {
    await channel.presence.leave();
    addLog("Presence left.");
  } catch (error) {
    addLog(`Presence leave skipped: ${String(error)}`);
  }
}

async function safeDetachChannel(channel) {
  if (!channel || SAFE_SKIP_STATES.has(channel.state) || channel.state === "initialized") {
    return;
  }
  try {
    if (channel.state !== "detached") {
      await channel.detach();
    }
  } catch (error) {
    addLog(`Channel detach skipped: ${String(error)}`);
  }
}

function safeReleaseChannel(name) {
  const channel = state.realtime?.channels.get(name);
  if (!channel || !SAFE_RELEASE_STATES.has(channel.state)) {
    return;
  }
  try {
    state.realtime.channels.release(name);
  } catch (error) {
    addLog(`Channel release skipped: ${String(error)}`);
  }
}

async function ensureChannels() {
  if (!state.realtime) {
    return;
  }

  state.lobbyChannel ??= state.realtime.channels.get(LOBBY_CHANNEL_NAME);
  state.privateChannel ??= state.realtime.channels.get(`lobby:${localClientId}`);
}

async function bindChannelHandlers() {
  await ensureChannels();
  if (!state.lobbyChannel || !state.privateChannel) {
    return;
  }

  if (!state.lobbyChannel.__bpPresenceBound) {
    await state.lobbyChannel.presence.subscribe("enter", () => void refreshLobby());
    await state.lobbyChannel.presence.subscribe("update", () => void refreshLobby());
    await state.lobbyChannel.presence.subscribe("leave", () => void refreshLobby());
    state.lobbyChannel.__bpPresenceBound = true;
  }

  if (!state.privateChannel.__bpMessageBound) {
    await state.privateChannel.subscribe("connect-request", async (message) => {
      const peerAddr = message.data?.peer_addr;
      const requester = message.data?.client_id ?? "unknown";
      addLog(`Incoming handshake from ${requester}: ${peerAddr ?? "n/a"}`);
      if (!peerAddr) {
        return;
      }

      try {
        await invoke("connect_to_peer", { peerAddr });
        addLog(`Host sent punch packets to ${peerAddr}.`);
      } catch (error) {
        addLog(`Punch error: ${String(error)}`);
      }
    });
    state.privateChannel.__bpMessageBound = true;
  }
}

async function refreshLobby() {
  await ensureChannels();
  if (!state.lobbyChannel || !state.realtime) {
    return;
  }

  try {
    if (state.realtime.connection.state !== "connected") {
      addLog("Lobby refresh postponed: Ably not connected.");
      return;
    }

    if (!safeShouldSkip(state.lobbyChannel) && state.lobbyChannel.state !== "attached") {
      await state.lobbyChannel.attach();
    }

    const members = await state.lobbyChannel.presence.get();
    hydrateServers(members);
    addLog(`Lobby refresh: ${members.length} presence members.`);
  } catch (error) {
    addLog(`Lobby refresh failed: ${String(error)}`);
  }
}

async function syncPresence(status, { force = false, enter = false } = {}) {
  await ensureChannels();
  if (
    !hostSession.active ||
    !state.lobbyChannel ||
    !hostSession.peerAddr ||
    state.syncingPresence ||
    state.realtime?.connection.state !== "connected"
  ) {
    return;
  }

  const payload = buildPresencePayload(status);
  const serialized = JSON.stringify(payload);
  if (!force && !enter && serialized === hostSession.presencePayload) {
    return;
  }

  state.syncingPresence = true;
  try {
    if (!safeShouldSkip(state.lobbyChannel) && state.lobbyChannel.state !== "attached") {
      await state.lobbyChannel.attach();
    }

    if (enter || !hostSession.presencePayload) {
      await state.lobbyChannel.presence.enter(payload);
      addLog(`Presence entered for room "${hostSession.roomName}" (${hostSession.peerAddr}).`);
    } else {
      await state.lobbyChannel.presence.update(payload);
    }

    hostSession.presencePayload = serialized;
  } catch (error) {
    addLog(`Presence sync failed: ${String(error)}`);
  } finally {
    state.syncingPresence = false;
  }
}

async function setupAbly() {
  state.realtime = new Ably.Realtime({
    key: ABLY_API_KEY,
    clientId: localClientId,
  });

  state.realtime.connection.on(async (change) => {
    ablyStateEl.textContent = change.current;
    addLog(`Ably connection: ${change.previous ?? "none"} -> ${change.current}`);

    if (change.current === "connected") {
      await bindChannelHandlers();
      await syncPresence(state.status, { force: true, enter: !hostSession.presencePayload });
      await refreshLobby();
    }
  });

  await new Promise((resolve) => state.realtime.connection.once("connected", resolve));
  await bindChannelHandlers();
  await refreshLobby();
}

async function startHosting() {
  const roomName = roomNameEl.value.trim();
  if (!roomName) {
    roomNameEl.focus();
    return;
  }

  const localPort = Number(localGamePortEl.value || 25565);
  const password = roomPasswordEl.value.trim() || null;
  hostButtonEl.disabled = true;
  state.tunnelReady = false;
  setMinecraftHint("Ожидание туннеля...", false);

  try {
    await invoke("start_hosting", { roomName, password, localPort });
    const status = await waitForStatus((snapshot) => Boolean(snapshot.publicUdpAddr));
    renderStatus(status);

    hostSession.active = true;
    hostSession.roomName = roomName;
    hostSession.hasPassword = Boolean(password);
    hostSession.peerAddr = status.publicUdpAddr ?? status.udpBindAddr;
    hostSession.localPort = localPort;
    hostSession.presencePayload = null;

    await syncPresence(status, { force: true, enter: true });
    await refreshLobby();
    renderActiveHost();
    closeModal();
  } catch (error) {
    addLog(`Host start failed: ${String(error)}`);
  } finally {
    syncButtons();
  }
}

async function stopHosting() {
  stopButtonEl.disabled = true;
  openHostModalEl.disabled = true;
  state.pendingConnects.clear();
  state.tunnelReady = false;
  setMinecraftHint("Ожидание туннеля...", false);

  try {
    await safePresenceLeave(state.lobbyChannel);
    await invoke("stop_hosting");
  } catch (error) {
    addLog(`Stop failed: ${String(error)}`);
  } finally {
    hostSession.active = false;
    hostSession.roomName = "";
    hostSession.hasPassword = false;
    hostSession.peerAddr = null;
    hostSession.localPort = 25565;
    hostSession.presencePayload = null;
    state.selectedServerId = null;

    try {
      const status = await invoke("get_status");
      renderStatus(status);
    } catch {
      renderStatus({
        mode: "idle",
        state: "idle",
        roomCode: null,
        udpBindAddr: null,
        publicUdpAddr: null,
        peerCount: 0,
        peers: [],
        note: "Idle",
        lastError: null,
        signalingServer: "Ably Presence + Channels",
        logs: ["Session cleared."],
      });
    }

    renderActiveHost();
    await refreshLobby();
    addLog("Host session stopped.");
    syncButtons();
  }
}

async function connectToServer(server) {
  if (server.clientId === localClientId) {
    addLog("Собственный host выбран. Для подключения нужен второй клиент.");
    return;
  }

  if (state.pendingConnects.has(server.clientId)) {
    addLog(`Повторный connect к ${server.roomName} заблокирован.`);
    return;
  }

  state.selectedServerId = server.clientId;
  state.pendingConnects.add(server.clientId);
  state.tunnelReady = false;
  setMinecraftHint("Connecting...", false);
  renderServers();

  if (server.hasPassword) {
    const provided = window.prompt(`Введите пароль для "${server.roomName}"`);
    if (provided == null) {
      state.pendingConnects.delete(server.clientId);
      renderServers();
      return;
    }
  }

  try {
    addLog(`Connecting to ${server.roomName} via ${server.peerAddr}`);
    await invoke("connect_to_peer", { peerAddr: server.peerAddr });
    const status = await waitForStatus((snapshot) => Boolean(snapshot.publicUdpAddr), 8000);

    await state.realtime.channels
      .get(`lobby:${server.clientId}`)
      .publish("connect-request", {
        client_id: localClientId,
        room_name: server.roomName,
        peer_addr: status.publicUdpAddr ?? status.udpBindAddr,
      });

    addLog(`Handshake request sent to host ${server.clientId}.`);
  } catch (error) {
    state.pendingConnects.delete(server.clientId);
    setMinecraftHint("Failed to punch through NAT.", false);
    addLog(`Connect failed: ${String(error)}`);
    renderServers();
  }
}

async function pollStatus() {
  try {
    const status = await invoke("get_status");
    renderStatus(status);
    await syncPresence(status);
  } catch (error) {
    addLog(`Status poll failed: ${String(error)}`);
  }
}

async function waitForStatus(predicate, timeoutMs = 6000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const status = await invoke("get_status");
    renderStatus(status);
    if (predicate(status)) {
      return status;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error("Timed out while waiting for backend status.");
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

await listen("tunnel_established", async (event) => {
  state.pendingConnects.clear();
  state.tunnelReady = true;
  setMinecraftHint("ПОДКЛЮЧАЙТЕСЬ В MINECRAFT К LOCALHOST:25565", true);
  addLog(`Tunnel established: ${event.payload?.minecraftAddr ?? "localhost:25565"}`);
  renderServers();
});

await listen("tunnel_failed", async (event) => {
  state.pendingConnects.clear();
  state.tunnelReady = false;
  setMinecraftHint("Failed to punch through NAT.", false);
  addLog(event.payload?.reason ?? "Failed to punch through NAT.");
  renderServers();
});

openHostModalEl.addEventListener("click", openModal);
closeModalEl.addEventListener("click", closeModal);
closeModalSecondaryEl.addEventListener("click", closeModal);
modalEl.addEventListener("click", (event) => {
  if (event.target instanceof HTMLElement && event.target.dataset.closeModal === "true") {
    closeModal();
  }
});

hostButtonEl.addEventListener("click", startHosting);
stopButtonEl.addEventListener("click", stopHosting);
refreshLobbyEl.addEventListener("click", async () => {
  await safeDetachChannel(state.lobbyChannel);
  safeReleaseChannel(LOBBY_CHANNEL_NAME);
  safeReleaseChannel(`lobby:${localClientId}`);
  state.lobbyChannel = null;
  state.privateChannel = null;
  await bindChannelHandlers();
  await refreshLobby();
});
copyLogsEl.addEventListener("click", async () => {
  const text = currentLogLines().join("\n");
  await navigator.clipboard.writeText(text);
  addLog("Debug log copied to clipboard.");
});
copySelectedEndpointEl.addEventListener("click", async () => {
  const selected = getSelectedServer();
  if (!selected?.peerAddr) {
    return;
  }
  await navigator.clipboard.writeText(selected.peerAddr);
  addLog(`Copied endpoint: ${selected.peerAddr}`);
});
connectSelectedEl.addEventListener("click", async () => {
  const selected = getSelectedServer();
  if (selected) {
    await connectToServer(selected);
  }
});

serverListEl.addEventListener("click", async (event) => {
  const target = event.target;
  if (!(target instanceof HTMLElement)) {
    return;
  }

  const selectId = target.closest("[data-select-server]")?.dataset.selectServer;
  if (selectId) {
    state.selectedServerId = selectId;
    renderSelectedServer();
    renderServers();
    return;
  }

  const connectId = target.closest("[data-connect-server]")?.dataset.connectServer;
  if (!connectId) {
    return;
  }

  const server = state.servers.find((item) => item.clientId === connectId);
  if (server) {
    await connectToServer(server);
  }
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    closeModal();
  }
});

setInterval(() => {
  void pollStatus();
}, POLL_INTERVAL_MS);

renderLogs();
renderActiveHost();
renderSelectedServer();
syncButtons();
await setupAbly();
await pollStatus();
