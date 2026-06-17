<script setup>
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getVersion } from '@tauri-apps/api/app'
import { globalProfile as profile } from '../store.js'

const props = defineProps({
  logs: { type: Array, default: () => [] },
  user: Object,
  isGuest: Boolean
})
const emit = defineEmits(['logout'])

// Profile Account logic
const email = computed(() => props.user?.email || (props.isGuest ? 'Гость (без аккаунта)' : '—'))
const provider = computed(() => {
  const id = props.user?.app_metadata?.provider
  if (!id) return props.isGuest ? 'Гость' : '—'
  return id === 'google' ? 'Google' : 'Email'
})

// Preferences
const theme = ref(localStorage.getItem('minecraft-p2p-theme') || 'oled')
const accent = ref(localStorage.getItem('minecraft-p2p-accent') || 'blue')
const language = ref(localStorage.getItem('minecraft-p2p-language') || 'ru')

// App info
const appVersion = ref('—')
const updateStatus = ref('—')
const updateAvailable = ref(false)

// Diagnostics
const testPort = ref(25566)
const ignoredPorts = ref([])

onMounted(async () => {
  try {
    appVersion.value = await getVersion()
  } catch {
    try {
      const info = await invoke('get_app_info')
      appVersion.value = info?.version ?? '—'
    } catch { appVersion.value = '—' }
  }
  loadIgnoredPorts()
})

const applyTheme = (t) => {
  theme.value = t
  document.body.dataset.theme = t
  localStorage.setItem('minecraft-p2p-theme', t)
}

const applyAccent = (a) => {
  accent.value = a
  document.documentElement.style.setProperty('--accent', `var(--accent-${a})`)
  document.documentElement.style.setProperty('--accent-strong', `color-mix(in srgb, var(--accent-${a}) 80%, white)`)
  localStorage.setItem('minecraft-p2p-accent', a)
}

const applyLanguage = (l) => {
  language.value = l
  localStorage.setItem('minecraft-p2p-language', l)
}

const checkUpdates = async () => {
  updateStatus.value = 'Проверяем...'
  try {
    const info = await invoke('check_for_updates')
    if (info?.available) {
      updateStatus.value = `Доступна версия ${info.latestVersion}`
      updateAvailable.value = true
    } else {
      updateStatus.value = 'У вас актуальная версия'
      updateAvailable.value = false
    }
  } catch (e) {
    updateStatus.value = 'Ошибка проверки: ' + e
  }
}

const installUpdate = async () => {
  try {
    await invoke('install_update')
    updateStatus.value = 'Обновление установлено. Перезапустите приложение.'
  } catch (e) {
    updateStatus.value = 'Ошибка установки: ' + e
  }
}

const runPreflight = async () => {
  try {
    const res = await invoke('run_preflight', { localPort: testPort.value })
    alert(JSON.stringify(res, null, 2))
  } catch (e) {
    alert('Ошибка: ' + e)
  }
}

const copyLogs = () => {
  navigator.clipboard.writeText(props.logs.join('\n')).catch(() => {})
}

const copyDiagnostics = async () => {
  try {
    const status = await invoke('get_status')
    navigator.clipboard.writeText(JSON.stringify(status, null, 2)).catch(() => {})
  } catch (e) {
    console.error(e)
  }
}

const loadIgnoredPorts = () => {
  try {
    const raw = localStorage.getItem('minecraft-p2p-ignored-ports-v1')
    ignoredPorts.value = raw ? JSON.parse(raw) : []
  } catch {
    ignoredPorts.value = []
  }
}

const removeIgnoredPort = (port) => {
  ignoredPorts.value = ignoredPorts.value.filter(p => p !== port)
  localStorage.setItem('minecraft-p2p-ignored-ports-v1', JSON.stringify(ignoredPorts.value))
}

const clearIgnoredPorts = () => {
  ignoredPorts.value = []
  localStorage.removeItem('minecraft-p2p-ignored-ports-v1')
}

const accentColors = [
  { key: 'blue', label: 'Blue' },
  { key: 'red', label: 'Ruby' },
  { key: 'yellow', label: 'Amber' },
  { key: 'green', label: 'Emerald' },
  { key: 'purple', label: 'Amethyst' },
  { key: 'orange', label: 'Flame' },
]

const activeSettingsTab = ref('account')
const status = ref({ mode: 'Idle', publicEndpoint: 'n/a' })

const refreshNetworkStatus = async () => {
  try {
    const res = await invoke('get_status')
    status.value = res
  } catch (e) {
    // Ignore error
  }
}

onMounted(() => {
  loadIgnoredPorts()
  checkUpdates()
  refreshNetworkStatus()
})
</script>

<template>
  <section class="page page-active">
    <header class="hero-panel settings-hero">
      <div class="hero-left">
        <h1>Настройки</h1>
      </div>
      <div class="hero-right settings-top-right">
        <span class="status-label">Версия</span>
        <strong>{{ appVersion }}</strong>
      </div>
    </header>

    <nav class="settings-tabs">
      <button class="settings-tab" :class="{ active: activeSettingsTab === 'account' }" @click="activeSettingsTab = 'account'">Аккаунт</button>
      <button class="settings-tab" :class="{ active: activeSettingsTab === 'interface' }" @click="activeSettingsTab = 'interface'">Интерфейс</button>
      <button class="settings-tab" :class="{ active: activeSettingsTab === 'network' }" @click="activeSettingsTab = 'network'">Сеть</button>
      <button class="settings-tab" :class="{ active: activeSettingsTab === 'diagnostics' }" @click="activeSettingsTab = 'diagnostics'">Диагностика</button>
      <button class="settings-tab" :class="{ active: activeSettingsTab === 'info' }" @click="activeSettingsTab = 'info'">Инфо</button>
    </nav>

    <div class="settings-layout">

      <!-- Account tab -->
      <div v-if="activeSettingsTab === 'account'" class="settings-tab-content active">
        <section class="panel">
          <div class="panel-head"><div><h2>Управление аккаунтом</h2></div></div>

          <!-- Account info -->
          <div class="profile-account-grid" style="margin-top: 16px;">
            <div class="profile-info-row">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/><polyline points="22,6 12,13 2,6"/></svg>
              <div class="profile-info-col">
                <span class="profile-info-label">Email</span>
                <span class="profile-info-value">{{ email }}</span>
              </div>
            </div>
            <div class="profile-info-row">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
              <div class="profile-info-col">
                <span class="profile-info-label">Способ входа</span>
                <span class="profile-info-value">{{ provider }}</span>
              </div>
            </div>
          </div>

          <div style="margin-top: 24px;">
            <p v-if="isGuest" style="color: var(--text-soft); font-size: 14px; margin-bottom: 12px;">Вы вошли как гость. Войдите в аккаунт, чтобы сохранять настройки и синхронизировать профиль.</p>
            <p v-else style="color: var(--text-soft); font-size: 14px; margin-bottom: 12px;">Вы вошли как <strong>{{ user?.email || 'Пользователь' }}</strong>.</p>
            
            <button v-if="!isGuest" class="ghost-button danger-ghost-button profile-logout-button" @click="emit('logout')">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>
              Выйти из аккаунта
            </button>
            <button v-else class="primary-button" @click="emit('logout')">
              Войти в аккаунт
            </button>
          </div>
        </section>
      </div>

      <!-- Interface tab -->
      <div v-if="activeSettingsTab === 'interface'" class="settings-tab-content active">
        <section class="panel">
          <div class="panel-head"><div><h2>Тема</h2></div></div>
          <div class="settings-options">
            <button class="option-card" :class="{ active: theme === 'white' }" @click="applyTheme('white')">
              <span>Light</span><small>Clean bright surfaces.</small>
            </button>
            <button class="option-card" :class="{ active: theme === 'dark' }" @click="applyTheme('dark')">
              <span>Graphite</span><small>Balanced dark theme.</small>
            </button>
            <button class="option-card" :class="{ active: theme === 'oled' }" @click="applyTheme('oled')">
              <span>Ember</span><small>Warm premium dark theme.</small>
            </button>
          </div>
        </section>

        <section class="panel">
          <div class="panel-head"><div><h2>Акцентный цвет</h2></div></div>
          <div class="accent-picker">
            <button
              v-for="a in accentColors" :key="a.key"
              class="accent-btn"
              :class="{ active: accent === a.key }"
              :style="`--btn-color: var(--accent-${a.key})`"
              :title="a.label"
              @click="applyAccent(a.key)"
            ></button>
          </div>
        </section>

        <section class="panel">
          <div class="panel-head"><div><h2>Язык</h2></div></div>
          <div class="settings-options">
            <button class="option-card" :class="{ active: language === 'ru' }" @click="applyLanguage('ru')">
              <span>Русский</span><small>Русский интерфейс.</small>
            </button>
            <button class="option-card" :class="{ active: language === 'en' }" @click="applyLanguage('en')">
              <span>English</span><small>English interface.</small>
            </button>
          </div>
        </section>
      </div>

      <!-- Network tab -->
      <div v-if="activeSettingsTab === 'network'" class="settings-tab-content active">
        <section class="panel snapshot-panel">
          <div class="panel-head">
            <div>
              <h2>Сетевой снимок</h2>
            </div>
          </div>
          <div class="snapshot-grid">
            <div class="metric">
              <span>Режим</span>
              <strong>{{ status?.mode || 'Idle' }}</strong>
            </div>
            <div class="metric">
              <span>Публичный адрес</span>
              <strong>{{ status?.publicUdpAddr || status?.publicEndpoint || 'n/a' }}</strong>
            </div>
            <div class="metric">
              <span>Версия</span>
              <strong>{{ appVersion }}</strong>
            </div>
            <div class="metric">
              <span>Подключения</span>
              <strong>{{ status?.peerCount || 0 }}</strong>
            </div>
          </div>
          <div class="network-stats-graph-container" style="margin-top: 16px; height: 100px;">
            <canvas id="network-stats-graph"></canvas>
            <div style="display: flex; justify-content: space-between; font-size: 0.85rem; color: var(--text-secondary); margin-top: 6px; font-family: monospace;">
              <span>100ms</span>
              <span>150ms</span>
              <span>200ms+</span>
            </div>
          </div>
        </section>
      </div>

      <!-- Diagnostics tab -->
      <div v-if="activeSettingsTab === 'diagnostics'" class="settings-tab-content active">
        <section class="panel">
          <div class="panel-head">
            <div><h2>Обновления</h2></div>
            <div class="hero-actions compact">
              <button class="ghost-button" @click="checkUpdates">Проверить обновления</button>
              <button v-if="updateAvailable" class="primary-button" @click="installUpdate">Установить</button>
            </div>
          </div>
          <div class="panel-copy">{{ updateStatus }}</div>
        </section>

        <section class="panel log-panel">
          <div class="panel-head">
            <div><h2>Логи приложения</h2></div>
            <div class="hero-actions compact">
              <button class="ghost-button" @click="copyLogs">Скопировать лог</button>
              <button class="ghost-button" @click="copyDiagnostics">JSON лог</button>
            </div>
          </div>
          <div class="log-list">
            <div v-if="!logs || logs.length === 0" class="empty-state">Лог пуст</div>
            <div v-else v-for="(line, i) in logs" :key="i" class="log-entry">{{ line }}</div>
          </div>
        </section>

        <section class="panel">
          <div class="panel-head"><div><h2>Диагностика</h2></div></div>
          <label class="field">
            <span>Порт диагностики</span>
            <input v-model.number="testPort" type="number" min="1" max="65535" />
          </label>
          <div class="tool-grid">
            <button class="ghost-button" @click="runPreflight">Проверить Minecraft</button>
          </div>
        </section>

        <section class="panel">
          <div class="panel-head">
            <div><h2>Игнорируемые порты</h2></div>
            <div class="hero-actions compact">
              <button class="ghost-button danger-button" @click="clearIgnoredPorts">Очистить список</button>
            </div>
          </div>
          <p class="panel-copy">Эти порты будут пропускаться при автопоиске.</p>
          <div class="tag-list">
            <div v-if="ignoredPorts.length === 0" class="empty-state">Список пуст</div>
            <div v-else v-for="port in ignoredPorts" :key="port" class="tag-item">
              <span>{{ port }}</span>
              <button class="remove-tag" @click="removeIgnoredPort(port)">×</button>
            </div>
          </div>
        </section>
      </div>

      <!-- Info tab -->
      <div v-if="activeSettingsTab === 'info'" class="settings-tab-content active">
        <section class="panel">
          <div class="panel-head"><div><h2>Информация</h2></div></div>
          <div class="panel-copy" style="margin-bottom: 16px;">
            Исходный код доступен на GitHub:<br/>
            <a href="https://github.com/oskarlolpo/minecraft-p2p-connector" target="_blank" style="color: var(--accent); text-decoration: none; display: inline-flex; align-items: center; gap: 6px; margin-top: 8px;">
              github.com/oskarlolpo/minecraft-p2p-connector
            </a>
          </div>
          <div class="panel-head" style="margin-top: 24px; border-top: 1px solid var(--line); padding-top: 16px;">
            <div><h2>Как создать хост</h2></div>
          </div>
          <div class="panel-copy">
            <ol style="margin-left: 20px; line-height: 1.6;">
              <li>Откройте Minecraft и зайдите в ваш одиночный мир.</li>
              <li>Нажмите ESC и выберите <strong>"Открыть для сети"</strong>.</li>
              <li>В приложении нажмите <strong>"Создать хост"</strong>. Приложение автоматически найдет открытый порт.</li>
              <li>Укажите название и тематику вашего сервера, затем нажмите <strong>"Запустить хост"</strong>.</li>
              <li>Поздравляем! Ваш сервер теперь виден другим игрокам.</li>
            </ol>
          </div>
        </section>
      </div>

    </div>
  </section>
</template>
