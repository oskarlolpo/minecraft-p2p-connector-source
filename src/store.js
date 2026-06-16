import { reactive, watch } from 'vue'

function loadProfile() {
  try {
    const raw = localStorage.getItem('minecraft-p2p-profile-v1')
    if (!raw) return { nickname: 'Player', avatarDataUrl: null, customId: null, customBanner: null }
    const p = JSON.parse(raw)
    return {
      nickname: p.nickname || 'Player',
      avatarDataUrl: p.avatarDataUrl || null,
      customId: p.customId || null,
      customBanner: p.customBanner || null
    }
  } catch {
    return { nickname: 'Player', avatarDataUrl: null, customId: null, customBanner: null }
  }
}

export const globalProfile = reactive(loadProfile())

watch(globalProfile, (val) => {
  localStorage.setItem('minecraft-p2p-profile-v1', JSON.stringify(val))
}, { deep: true })
