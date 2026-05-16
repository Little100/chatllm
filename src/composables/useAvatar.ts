import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const userAvatar = ref<string | null>(null)
const llmAvatar = ref<string | null>(null)
const loaded = ref(false)

async function loadAvatars() {
  if (loaded.value) return
  try {
    const [user, llm] = await Promise.all([
      invoke<string | null>('get_avatar', { avatarType: 'user' }),
      invoke<string | null>('get_avatar', { avatarType: 'llm' }),
    ])
    userAvatar.value = user
    llmAvatar.value = llm
    loaded.value = true
  } catch (_) {
    loaded.value = true
  }
}

async function saveAvatar(type: 'user' | 'llm', base64Data: string) {
  await invoke('save_avatar', { avatarType: type, data: base64Data })
  if (type === 'user') {
    userAvatar.value = base64Data
  } else {
    llmAvatar.value = base64Data
  }
}

async function removeAvatar(type: 'user' | 'llm') {
  try {
    await invoke('delete_avatar', { avatarType: type })
  } catch (_) {}
  if (type === 'user') {
    userAvatar.value = null
  } else {
    llmAvatar.value = null
  }
}

export function useAvatar() {
  loadAvatars()
  return { userAvatar, llmAvatar, saveAvatar, removeAvatar, loadAvatars }
}
