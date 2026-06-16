<script setup>
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps({
  isOpen: Boolean,
  serverName: String,
  serverHost: String,
  hasPassword: Boolean,
})
const emit = defineEmits(['close', 'connect'])

const password = ref('')
const connecting = ref(false)

const handleConnect = () => {
  connecting.value = true
  emit('connect', { password: password.value })
}
</script>

<template>
  <div v-if="isOpen" class="modal-shell">
    <div class="modal-backdrop" @click="emit('close')"></div>
    <div class="modal-card" role="dialog" aria-modal="true">
      <div class="panel-head">
        <div>
          <span class="eyebrow">ПОДКЛЮЧЕНИЕ</span>
          <h2>{{ serverName || 'Сервер' }}</h2>
          <span style="color: var(--text-soft); font-size: 13px;">Хост: {{ serverHost || 'Unknown' }}</span>
        </div>
        <button @click="emit('close')" class="icon-button" type="button" aria-label="Закрыть">×</button>
      </div>

      <div class="modal-grid">
        <label v-if="hasPassword" class="field">
          <span>Пароль сервера</span>
          <input v-model="password" type="password" placeholder="Введите пароль" autocomplete="off" />
        </label>
        <p v-else style="color: var(--text-soft); font-size: 14px;">Подключиться к серверу без пароля?</p>
      </div>

      <div v-if="connecting" style="margin: 12px 24px 0; height: 4px; background: var(--line); border-radius: 2px; overflow: hidden;">
        <div style="height: 100%; width: 30%; background: var(--accent); border-radius: 2px; animation: indeterminate 1.5s infinite ease-in-out;"></div>
      </div>

      <div class="modal-actions">
        <button @click="emit('close')" class="ghost-button">Отмена</button>
        <button @click="handleConnect" class="primary-button" :disabled="connecting">
          {{ connecting ? 'Подключение...' : 'Подключиться' }}
        </button>
      </div>
    </div>
  </div>
</template>
