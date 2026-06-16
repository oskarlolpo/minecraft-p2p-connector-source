<script setup>
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps({
  isOpen: Boolean,
})
const emit = defineEmits(['close', 'start-host'])

const roomName = ref('')
const roomTheme = ref('survival')
const gameVersion = ref('java')
const localGamePort = ref(25565)
const requirePassword = ref(false)
const roomPassword = ref('')
const enableGeyser = ref(false)
const geyserPort = ref(19132)
const forceDirectMode = ref(false)
const externalHostMode = ref(false)
const externalHostAddress = ref('')
const isHosting = ref(false)
const portDetecting = ref(false)

// Auto-detect port when modal opens
watch(() => props.isOpen, async (open) => {
  if (!open) { isHosting.value = false; return }
  roomName.value = ''
  await autoDetectPort()
  await autofillName()
})

const autoDetectPort = async () => {
  portDetecting.value = true
  try {
    const ports = await invoke('get_available_lan_ports_command', { ignoredPorts: [] })
    if (ports && ports.length >= 1) {
      localGamePort.value = ports[0].port
    }
  } catch { /* silent - user can enter manually */ }
  finally { portDetecting.value = false }
}

const autofillName = async () => {
  if (!localGamePort.value) return
  try {
    const probe = await invoke('query_external_server', { host: '127.0.0.1', port: localGamePort.value })
    let name = String(probe?.roomName || probe?.description || '').trim()
    if (!name) return
    name = name.replace(/[&§][0-9a-fklmnor]/gi, '').trim()
    const dash = name.indexOf(' - ')
    if (dash !== -1 && dash < 20) name = name.slice(dash + 3).trim()
    if (name && !roomName.value) roomName.value = name
  } catch { /* silent */ }
}

const startHosting = () => {
  isHosting.value = true
  emit('start-host', {
    roomName: roomName.value,
    roomTheme: roomTheme.value,
    gameVersion: gameVersion.value,
    localGamePort: localGamePort.value,
    requirePassword: requirePassword.value,
    roomPassword: roomPassword.value,
    enableGeyser: enableGeyser.value,
    geyserPort: geyserPort.value,
    forceDirectMode: forceDirectMode.value,
    isExternalHost: externalHostMode.value,
    externalHostAddress: externalHostAddress.value,
  })
}
</script>

<template>
  <div v-if="isOpen" class="modal-shell" aria-hidden="false">
    <div class="modal-backdrop" @click="emit('close')"></div>
    <div class="modal-card" role="dialog" aria-modal="true">
      <div class="panel-head">
        <div>
          <span class="eyebrow">ПАРАМЕТРЫ ХОСТА</span>
          <div style="display: flex; justify-content: space-between; align-items: center;">
            <h2>Создать хост</h2>
          </div>
        </div>
        <button @click="emit('close')" class="icon-button" type="button" aria-label="Закрыть">×</button>
      </div>

      <div class="modal-grid">
        <label class="field">
          <span>Название сервера</span>
          <input v-model="roomName" type="text" maxlength="32" placeholder="Northern Lights SMP" />
        </label>

        <div class="field">
          <span>Тематика сервера</span>
          <div class="theme-chip-grid">
            <button type="button" class="theme-chip" :class="{ active: roomTheme === 'survival' }" @click="roomTheme = 'survival'">
              <span>Выживание</span>
            </button>
            <button type="button" class="theme-chip" :class="{ active: roomTheme === 'vanilla' }" @click="roomTheme = 'vanilla'">
              <span>Ванилла</span>
            </button>
            <button type="button" class="theme-chip" :class="{ active: roomTheme === 'creative' }" @click="roomTheme = 'creative'">
              <span>Творческий</span>
            </button>
            <button type="button" class="theme-chip" :class="{ active: roomTheme === 'minigames' }" @click="roomTheme = 'minigames'">
              <span>Мини-игры</span>
            </button>
            <button type="button" class="theme-chip" :class="{ active: roomTheme === 'anarchy' }" @click="roomTheme = 'anarchy'">
              <span>Анархия</span>
            </button>
            <button type="button" class="theme-chip" :class="{ active: roomTheme === 'modded' }" @click="roomTheme = 'modded'">
              <span>С модами</span>
            </button>
            <button type="button" class="theme-chip" :class="{ active: roomTheme === 'pvp' }" @click="roomTheme = 'pvp'">
              <span>PvP</span>
            </button>
            <button type="button" class="theme-chip" :class="{ active: roomTheme === 'rpg' }" @click="roomTheme = 'rpg'">
              <span>РПГ</span>
            </button>
            <button type="button" class="theme-chip" :class="{ active: roomTheme === 'other' }" @click="roomTheme = 'other'">
              <span>Другое</span>
            </button>
          </div>
        </div>

        <div class="field">
          <span>Версия игры</span>
          <div class="theme-chip-grid">
            <button type="button" class="theme-chip" :class="{ active: gameVersion === 'java' }" @click="gameVersion = 'java'">
              <span>Java Edition</span>
            </button>
            <button type="button" class="theme-chip" :class="{ active: gameVersion === 'bedrock' }" @click="gameVersion = 'bedrock'">
              <span>Bedrock Edition</span>
            </button>
          </div>
        </div>

        <label class="field" v-if="!externalHostMode">
          <span class="field-inline">
            <span>Локальный {{ gameVersion === 'java' ? 'Java' : 'Bedrock' }}-порт</span>
            <div class="field-inline-actions">
              <button class="mini-button" type="button" @click="autoDetectPort" :disabled="portDetecting">
                {{ portDetecting ? '...' : 'Auto' }}
              </button>
            </div>
          </span>
          <input v-model.number="localGamePort" type="number" min="1" max="65535" />
        </label>

        <label class="checkbox-row">
          <input v-model="requirePassword" type="checkbox" />
          <span>Требовать пароль</span>
        </label>

        <form v-if="requirePassword" class="field" @submit.prevent>
          <span>Пароль комнаты</span>
          <input v-model="roomPassword" type="password" autocomplete="new-password" maxlength="64" placeholder="Введите пароль" />
        </form>

        <div v-if="gameVersion === 'java'">
          <label class="checkbox-row">
            <input v-model="enableGeyser" type="checkbox" />
            <span>Включить Bedrock bridge (Geyser)</span>
          </label>
        </div>

        <label v-if="enableGeyser && gameVersion === 'java'" class="field">
          <span>Bedrock UDP-порт</span>
          <input v-model.number="geyserPort" type="number" min="1" max="65535" />
        </label>

        <label class="checkbox-row">
          <input v-model="forceDirectMode" type="checkbox" />
          <span>Использовать только бесплатное подключение (NAT)</span>
        </label>

        <label class="checkbox-row">
          <input v-model="externalHostMode" type="checkbox" />
          <span>Добавить внешний сервер вместо локального хоста</span>
        </label>

        <label v-if="externalHostMode" class="field">
          <span>IP или домен (можно с портом)</span>
          <input v-model="externalHostAddress" type="text" maxlength="255" placeholder="play.example.com:25565" />
        </label>
      </div>

      <div v-if="isHosting" class="host-progress-container" style="margin: 16px 24px 0 24px;">
        <div style="display: flex; justify-content: space-between; font-size: 13px; color: var(--text-secondary); margin-bottom: 6px;">
          <span>Запуск хоста...</span>
        </div>
        <div style="height: 4px; background: var(--line); border-radius: 2px; overflow: hidden; position: relative;">
          <div style="position: absolute; top: 0; left: 0; height: 100%; width: 30%; background: var(--accent); border-radius: 2px; animation: indeterminate 1.5s infinite ease-in-out;"></div>
        </div>
      </div>

      <div class="modal-actions">
        <button @click="emit('close')" class="ghost-button" type="button">Отмена</button>
        <button @click="startHosting" class="primary-button" type="button" :disabled="isHosting">
          Запустить хост
        </button>
      </div>
    </div>
  </div>
</template>
