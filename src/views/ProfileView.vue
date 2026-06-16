<script setup>
import { ref, computed } from 'vue'
import { globalProfile as profile } from '../store.js'

const props = defineProps({
  user: Object,
  isGuest: Boolean
})

const emit = defineEmits(['logout'])

const isEditingName = ref(false)
const isEditingId = ref(false)
const nameInput = ref('')
const idInput = ref('')

const displayName = computed(() =>
  props.user?.user_metadata?.full_name || profile.nickname || 'Player'
)
const displayLetter = computed(() => displayName.value.slice(0, 1).toUpperCase())
const displayId = computed(() => profile.customId || null)

// Edit name
const startEditName = () => {
  nameInput.value = profile.nickname
  isEditingName.value = true
}
const saveName = () => {
  const v = nameInput.value.trim()
  if (v) profile.nickname = v
  isEditingName.value = false
}
const cancelName = () => { isEditingName.value = false }

// Edit ID
const startEditId = () => {
  idInput.value = profile.customId || ''
  isEditingId.value = true
}
const saveId = () => {
  const v = idInput.value.trim().toLowerCase().replace(/[^a-z0-9_]/g, '')
  profile.customId = v || null
  isEditingId.value = false
}
const cancelId = () => { isEditingId.value = false }

// Avatar
const avatarFileInput = ref(null)
const pickAvatar = () => avatarFileInput.value?.click()
const onAvatarFile = (e) => {
  const file = e.target.files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    profile.avatarDataUrl = String(reader.result)
  }
  reader.readAsDataURL(file)
}

// Banner
const bannerFileInput = ref(null)
const pickBanner = () => bannerFileInput.value?.click()
const onBannerFile = (e) => {
  const file = e.target.files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    profile.customBanner = String(reader.result)
  }
  reader.readAsDataURL(file)
}
</script>

<template>
  <section class="page page-active">
    <div class="social-profile-container">
      <!-- Banner -->
      <div class="social-banner-wrap">
        <img v-if="profile.customBanner" :src="profile.customBanner" class="social-banner" alt="Banner" />
        <div v-else class="social-banner-placeholder"></div>
        <button class="social-edit-img-btn" type="button" @click="pickBanner" aria-label="Изменить фон">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>
        </button>
        <input ref="bannerFileInput" type="file" accept="image/*" class="visually-hidden" @change="onBannerFile" />
      </div>

      <!-- Profile content -->
      <div class="social-profile-content">
        <div class="social-avatar-row">
          <!-- Avatar -->
          <div class="social-avatar-wrap">
            <div class="profile-avatar-xl">
              <img v-if="profile.avatarDataUrl || user?.user_metadata?.avatar_url" :src="profile.avatarDataUrl || user?.user_metadata?.avatar_url" alt="" @error="(e) => e.target.style.display = 'none'" />
              <span v-else>{{ displayLetter }}</span>
            </div>
            <button class="social-edit-img-btn avatar-edit-btn" type="button" @click="pickAvatar" aria-label="Изменить аватарку">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>
            </button>
            <input ref="avatarFileInput" type="file" accept="image/*" class="visually-hidden" @change="onAvatarFile" />
          </div>

          <!-- Identity -->
          <div class="social-identity">
            <!-- Name row -->
            <div class="social-name-row">
              <template v-if="isEditingName">
                <input
                  v-model="nameInput"
                  class="social-name-input"
                  maxlength="32"
                  @keyup.enter="saveName"
                  @keyup.escape="cancelName"
                  autofocus
                />
                <div class="edit-confirm-capsule">
                  <button class="confirm-capsule-btn save" @click="saveName" title="Сохранить">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
                  </button>
                  <div class="capsule-divider"></div>
                  <button class="confirm-capsule-btn discard" @click="cancelName" title="Отмена">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                  </button>
                </div>
              </template>
              <template v-else>
                <h1 class="social-name">{{ displayName }}</h1>
                <button class="mini-icon-button" @click="startEditName" aria-label="Изменить имя">
                  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>
                </button>
              </template>
            </div>

            <!-- ID row -->
            <div class="social-id-row">
              <template v-if="isEditingId">
                <input
                  v-model="idInput"
                  class="social-id-input-inline"
                  maxlength="24"
                  placeholder="your_id"
                  @keyup.enter="saveId"
                  @keyup.escape="cancelId"
                />
                <div class="edit-confirm-capsule">
                  <button class="confirm-capsule-btn save" @click="saveId" title="Сохранить">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
                  </button>
                  <div class="capsule-divider"></div>
                  <button class="confirm-capsule-btn discard" @click="cancelId" title="Отмена">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                  </button>
                </div>
              </template>
              <template v-else>
                <span class="social-uid">{{ displayId ? `@${displayId}` : 'ID: —' }}</span>
                <button class="mini-icon-button" @click="startEditId" aria-label="Изменить ID">
                  <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>
                </button>
              </template>
            </div>
          </div>
        </div>

        <!-- Stats row -->
        <div class="social-stats-row">
          <div class="social-stat"><strong class="social-stat-num">0</strong><span class="social-stat-label">Подписок</span></div>
          <div class="social-stat"><strong class="social-stat-num">0</strong><span class="social-stat-label">Подписчиков</span></div>
          <div class="social-stat"><strong class="social-stat-num">0</strong><span class="social-stat-label">Друзей</span></div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* Profile specific layout constraints */
.social-profile-container {
  margin-top: 24px;
}
</style>
