import { ref, nextTick, onUnmounted } from 'vue'
import type { Ref } from 'vue'

export function useAutoScroll(containerRef: Ref<HTMLElement | null>) {
  const enabled = ref(true)
  let observer: MutationObserver | null = null

  function scrollToBottom() {
    const el = containerRef.value
    if (el && enabled.value) {
      el.scrollTop = el.scrollHeight
    }
  }

  function onScroll() {
    const el = containerRef.value
    if (!el) return
    // 距底部 50px 以内视为在底部
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 50
    enabled.value = atBottom
  }

  function startObserving() {
    const el = containerRef.value
    if (!el) return
    el.addEventListener('scroll', onScroll)
    observer = new MutationObserver(() => {
      if (enabled.value) {
        nextTick(scrollToBottom)
      }
    })
    observer.observe(el, { childList: true, subtree: true, characterData: true })
  }

  function stopObserving() {
    const el = containerRef.value
    if (el) {
      el.removeEventListener('scroll', onScroll)
    }
    observer?.disconnect()
    observer = null
  }

  onUnmounted(stopObserving)

  return { enabled, scrollToBottom, startObserving, stopObserving }
}
