<script setup lang="ts">
import { hideWindow, quitApplication } from '../platform/desktop'
import { useAppStore } from '../stores/appStore'
const store = useAppStore()

async function choose(choice: 'hide' | 'quit') {
  if (store.state.rememberCloseChoice) {
    store.state.closePreference = choice
    store.state.hideToTray = choice === 'hide'
  }
  store.state.closeDialogOpen = false
  if (choice === 'hide') await hideWindow()
  else await quitApplication()
}
</script>

<template>
  <Transition name="modal">
    <div v-if="store.state.closeDialogOpen" class="modal-backdrop">
      <section class="close-dialog" role="dialog" aria-modal="true" aria-labelledby="close-title">
        <div class="brand-mark brand-mark--small"><span></span></div>
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

