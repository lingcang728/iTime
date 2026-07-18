<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { hideWindow, quitApplication } from '../platform/desktop'
import { useAppStore } from '../stores/appStore'
import AppMark from './AppMark.vue'
const store = useAppStore()
const dialog = ref<HTMLElement | null>(null)
let returnFocus: HTMLElement | null = null

function closeDialog(): void {
  store.state.closeDialogOpen = false
  nextTick(() => returnFocus?.focus())
}

function focusableElements(): HTMLElement[] {
  return dialog.value
    ? [...dialog.value.querySelectorAll<HTMLElement>('button, input, [href], [tabindex]:not([tabindex="-1"])')]
        .filter((element) => !element.hasAttribute('disabled'))
    : []
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.preventDefault()
    closeDialog()
    return
  }
  if (event.key !== 'Tab') return
  const elements = focusableElements()
  if (!elements.length) return
  const first = elements[0]!
  const last = elements[elements.length - 1]!
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault()
    last.focus()
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault()
    first.focus()
  }
}

watch(() => store.state.closeDialogOpen, async (open) => {
  if (!open) return
  returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
  await nextTick()
  focusableElements()[0]?.focus()
})

onBeforeUnmount(() => returnFocus?.focus())

async function choose(choice: 'hide' | 'quit') {
  if (store.state.rememberCloseChoice) {
    store.state.closePreference = choice
  }
  store.state.closeDialogOpen = false
  if (choice === 'hide') await hideWindow()
  else await quitApplication()
}
</script>

<template>
  <Transition name="modal">
    <div v-if="store.state.closeDialogOpen" class="modal-backdrop" @click.self="closeDialog">
      <section ref="dialog" class="close-dialog" role="dialog" aria-modal="true" aria-labelledby="close-title" tabindex="-1" @keydown="handleKeydown">
        <div class="close-dialog__mark"><AppMark :size="32" /></div>
        <h2 id="close-title">继续在托盘中运行？</h2>
        <p>隐藏窗口后，iTime 会继续保持当前记录状态。你可以随时从系统托盘重新打开。</p>
        <label class="check-row"><input v-model="store.state.rememberCloseChoice" type="checkbox" />记住我的选择</label>
        <div class="dialog-actions">
          <button type="button" class="button secondary" @click="choose('quit')">退出 iTime</button>
          <button type="button" class="button primary" @click="choose('hide')">隐藏到托盘</button>
        </div>
      </section>
    </div>
  </Transition>
</template>
