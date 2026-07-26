import { onBeforeUnmount, onMounted, ref, type Ref } from 'vue'

/**
 * Returns a reactive timestamp (ms) updated every minute.
 * Used to drive "now" indicators on timeline views.
 */
export function useNow(intervalMs = 60_000): { nowMs: Ref<number> } {
  const nowMs = ref<number>(Date.now())
  let timer: number | undefined

  onMounted(() => {
    // align to next minute boundary for smoother ticking
    const remaining = intervalMs - (Date.now() % intervalMs)
    timer = window.setTimeout(() => {
      nowMs.value = Date.now()
      timer = window.setInterval(() => { nowMs.value = Date.now() }, intervalMs) as unknown as number
    }, remaining)
  })

  onBeforeUnmount(() => {
    window.clearTimeout(timer)
    window.clearInterval(timer)
  })

  return { nowMs }
}
