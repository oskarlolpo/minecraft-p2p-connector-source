<script setup>
import { ref } from 'vue'

const props = defineProps({
  isOpen: Boolean
})
const emit = defineEmits(['close', 'login', 'guest', 'google'])

const isLoginMode = ref(true)
const email = ref('')
const password = ref('')
const errorMsg = ref('')

const handleSubmit = () => {
  emit('login', {
    email: email.value,
    password: password.value,
    isRegister: !isLoginMode.value
  })
}

const toggleMode = () => {
  isLoginMode.value = !isLoginMode.value
  errorMsg.value = ''
}
</script>

<template>
  <div v-if="isOpen" id="auth-overlay" class="modal-shell" style="z-index: 100;">
    <div class="modal-backdrop"></div>
    <div class="modal-card" style="max-width: 400px; width: 100%;">
      <div class="panel-head" style="justify-content: center; flex-direction: column; align-items: center; text-align: center; margin-bottom: 24px;">
        <h2 style="margin: 0; font-size: 28px;">{{ isLoginMode ? 'Вход' : 'Регистрация' }}</h2>
        <span class="eyebrow" style="margin-top: 8px;">
          {{ isLoginMode ? 'Войдите в свой аккаунт' : 'Создайте аккаунт, чтобы продолжить' }}
        </span>
      </div>

      <div v-if="errorMsg" style="margin-bottom: 16px; padding: 12px; background: rgba(239, 86, 111, 0.15); border: 1px solid var(--danger); border-radius: var(--radius-sm); color: var(--danger); font-size: 14px; text-align: center;">
        {{ errorMsg }}
      </div>

      <form @submit.prevent="handleSubmit" style="display: flex; flex-direction: column; gap: 16px;">
        <label class="field">
          <span>Email</span>
          <input v-model="email" type="email" placeholder="player@example.com" autocomplete="username" required />
        </label>
        <label class="field">
          <span>Пароль</span>
          <input v-model="password" type="password" placeholder="••••••••" required minlength="6" :autocomplete="isLoginMode ? 'current-password' : 'new-password'" />
        </label>
        
        <button type="submit" class="primary-button" style="width: 100%; margin-top: 8px;">
          {{ isLoginMode ? 'Войти' : 'Зарегистрироваться' }}
        </button>

        <div style="display: flex; align-items: center; gap: 12px; margin: 4px 0;">
          <div style="flex: 1; height: 1px; background: var(--line);"></div>
          <span style="font-size: 12px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.1em;">или</span>
          <div style="flex: 1; height: 1px; background: var(--line);"></div>
        </div>

        <button @click="emit('google')" type="button" class="ghost-button" style="width: 100%; display: flex; align-items: center; justify-content: center; gap: 10px;">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4"/>
            <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/>
            <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05"/>
            <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335"/>
          </svg>
          Войти через Google
        </button>

        <button @click="emit('guest')" type="button" class="ghost-button" style="width: 100%; margin-top: 8px;">
          Войти как Гость (Локально)
        </button>

        <div style="text-align: center; margin-top: 8px;">
          <button @click="toggleMode" type="button" style="background: none; border: none; color: var(--text-soft); text-decoration: underline; font-size: 13px; cursor: pointer;">
            {{ isLoginMode ? 'Нет аккаунта? Зарегистрироваться' : 'Уже есть аккаунт? Войти' }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>
