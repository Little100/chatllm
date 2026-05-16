import { ref } from 'vue'

type Theme = 'light' | 'dark' | 'system'

function readStoredTheme(): Theme {
  const saved = localStorage.getItem('chatllm-theme')
  return saved === 'light' || saved === 'dark' || saved === 'system' ? saved : 'system'
}

const theme = ref<Theme>(readStoredTheme())
const isDark = ref(false)

function applyTheme(dark: boolean) {
  isDark.value = dark
  if (dark) {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
}

function getSystemDark(): boolean {
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

// 启动前立即同步应用(避免白屏闪烁)
;(() => {
  const dark = theme.value === 'system' ? getSystemDark() : theme.value === 'dark'
  if (dark) document.documentElement.classList.add('dark')
})()

// 系统主题监听只挂一次
const systemMediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
systemMediaQuery.addEventListener('change', (e) => {
  if (theme.value === 'system') {
    applyTheme(e.matches)
  }
})

export function useTheme() {
  function setTheme(t: Theme) {
    theme.value = t
    localStorage.setItem('chatllm-theme', t)
    if (t === 'system') {
      applyTheme(getSystemDark())
    } else {
      applyTheme(t === 'dark')
    }
  }

  function initTheme() {
    theme.value = readStoredTheme()
    if (theme.value === 'system') {
      applyTheme(getSystemDark())
    } else {
      applyTheme(theme.value === 'dark')
    }
  }

  return { theme, isDark, setTheme, initTheme }
}
