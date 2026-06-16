const fs = require('fs');
let code = fs.readFileSync('main.js', 'utf8');

const setupAblyRegex = /async function setupAbly\(\) \{[\s\S]*?\n\}/;
const ensureChannelsRegex = /async function ensureChannels\(\) \{[\s\S]*?\n\}/;
const bindChannelHandlersRegex = /async function bindChannelHandlers\(\) \{[\s\S]*?state\.privateChannel\.__mcp2pHandshakeBound = true;\s*\n\}/;
const refreshLobbyRegex = /async function refreshLobby\(\) \{[\s\S]*?\n\}/;
const syncPresenceRegex = /async function syncPresence\(status, \{ force = false, enter = false \} = \{\}\) \{[\s\S]*?\n\}/;

const newEnsureChannels = `async function ensureChannels() { /* deprecated */ }`;

const newSetupAbly = `async function setupAbly() {
  ablyStateEl.textContent = "connected";
  await bindChannelHandlers();
  setInterval(refreshLobby, 10000); // Poll lobby every 10 seconds
  await refreshLobby();
}`;

const newRefreshLobby = `async function refreshLobby() {
  try {
    const members = await invoke("refresh_lobby");
    await refreshExternalServers();
    hydrateServers(members);
  } catch (error) {
    addLog(t("lobbyRefreshFailed", { error: String(error) }));
  }
}`;

const newSyncPresence = `async function syncPresence(status, { force = false, enter = false } = {}) {
  if (!canAdvertiseHost() || state.syncingPresence) return;

  const payload = buildPresencePayload(status);
  const serialized = JSON.stringify(payload);
  if (!force && !enter && serialized === hostSession.presencePayload) return;

  state.syncingPresence = true;
  try {
    const shouldEnter = enter || !hostSession.presenceEntered;
    const advertisedAddress = payload.public_join_address ?? payload.endpoint ?? "n/a";
    
    await invoke("update_lobby_presence", { clientId: localClientId, payload });
    
    if (shouldEnter) {
      addLog(t("hostStartedPresence", { room: hostSession.roomName, addr: advertisedAddress }));
      hostSession.presenceEntered = true;
    } else {
      addLog(\`Presence updated for \${hostSession.roomName} (\${advertisedAddress}).\`);
    }
    hostSession.presencePayload = serialized;
  } catch (error) {
    addLog(t("presenceSyncFailed", { error: String(error) }));
  } finally {
    state.syncingPresence = false;
  }
}`;

const newBindChannelHandlers = `async function bindChannelHandlers() {
  if (state.__mcp2pHandshakeBound) return;
  const { listen } = window.__TAURI__.event;
  
  await invoke("subscribe_lobby_events", { channel: \`lobby:\${localClientId}\` });
  
  await listen("lobby-event", async (event) => {
    const channel = event.payload.channel;
    const msg = event.payload.data;
    const name = msg.name;
    const data = typeof msg.data === "string" ? JSON.parse(msg.data) : msg.data;
    
    if (name === "connect-request") {
      const peerAddr = data?.peer_addr;
      const requester = data?.client_id ?? "unknown";
      const nickname = data?.nickname ?? requester;
      const minecraftNickname = data?.minecraft_nickname ?? null;
      const launcher = data?.launcher ?? null;
      const minecraftVersion = data?.client_minecraft_version ?? null;
      const modLoader = data?.mod_loader ?? null;
      const relaySessionId = data?.relay_session_id ?? null;
      state.peerProfiles.set(requester, { nickname, minecraftNickname, launcher, minecraftVersion, modLoader });
      addLog(t("incomingHandshake", { peer: requester, addr: peerAddr ?? "n/a" }));

      const isRealAddr = peerAddr && !peerAddr.startsWith("0.0.0.0") && peerAddr !== "";

      try {
        if (isRealAddr) {
          await invoke("connect_to_peer", { peerId: requester, peerAddrs: [peerAddr], relaySessionId });
          addLog(t("hostPunchSent", { addr: peerAddr }));
        } else if (relaySessionId) {
          addLog(\`Relay-only connect from \${requester} (addr=\${peerAddr ?? "none"}), session=\${relaySessionId}\`);
          await invoke("connect_to_peer", { peerId: requester, peerAddrs: ["0.0.0.0:0"], relaySessionId });
        } else {
          addLog(\`Rejected connect-request from \${requester}: no addr and no relay session\`);
          return;
        }

        await invoke("publish_lobby_event", { 
          channel: \`lobby:\${requester}\`, 
          event: "connect-ack", 
          payload: { relay_session_id: relaySessionId, host_id: localClientId }
        });
      } catch (error) {
        addLog(\`Punch/relay error for \${requester}: \${String(error)}\`);
        await invoke("publish_lobby_event", { 
          channel: \`lobby:\${requester}\`, 
          event: "connect-reject", 
          payload: { relay_session_id: relaySessionId, host_id: localClientId, error: String(error) }
        });
      }
    } else if (name === "connect-ack") {
      const relaySessionId = data?.relay_session_id ?? null;
      if (relaySessionId) state.pendingRelayAcks.get(relaySessionId)?.resolve(data ?? null);
    } else if (name === "connect-reject") {
      const relaySessionId = data?.relay_session_id ?? null;
      if (relaySessionId) {
        const error = data?.error ? new Error(String(data.error)) : new Error("relay connect rejected by host");
        state.pendingRelayAcks.get(relaySessionId)?.reject(error);
      }
    }
  });
  state.__mcp2pHandshakeBound = true;
}`;

code = code.replace(ensureChannelsRegex, newEnsureChannels);
code = code.replace(setupAblyRegex, newSetupAbly);
code = code.replace(refreshLobbyRegex, newRefreshLobby);
code = code.replace(syncPresenceRegex, newSyncPresence);
code = code.replace(bindChannelHandlersRegex, newBindChannelHandlers);

fs.writeFileSync('main.js', code);
console.log('Successfully patched main.js');
