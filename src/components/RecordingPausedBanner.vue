<script setup lang="ts">
import { computed } from 'vue'
import { PhPause } from '@phosphor-icons/vue'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()
const paused = computed(() => !store.state.recording && store.state.recordingStatus !== 'loading')
const busy = computed(() => store.state.recordingStatus === 'loading')
const failed = computed(() => store.state.recordingStatus === 'error')
</script>

<template>
  <div v-if="paused || failed || busy" class="paused-banner" :data-state="failed ? 'error' : busy ? 'busy' : 'paused'" role="status">
    <PhPause :size="17" weight="fill" aria-hidden="true" />
    <p v-if="failed"><strong>记录状态异常</strong>{{ store.state.recordingMessage }}</p>
    <p v-else-if="busy"><strong>正在切换记录状态</strong>{{ store.state.recordingMessage }}</p>
    <p v-else><strong>记录已暂停</strong>当前不产生新记录，页面展示的是已有数据。</p>
    <button type="button" :disabled="busy" @click="store.setRecording(true)">{{ busy ? '处理中…' : '继续记录' }}</button>
  </div>
</template>

<style scoped>
.paused-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 13px;
  border: 1px solid color-mix(in srgb, var(--warning) 42%, var(--border-soft));
  border-radius: var(--radius-md);
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--warning-soft) 45%, var(--bg-card));
  font-size: var(--text-xs);
}

.paused-banner[data-state='error'] {
  border-color: color-mix(in srgb, var(--danger) 45%, var(--border-soft));
  background: color-mix(in srgb, var(--danger-soft) 40%, var(--bg-card));
}

.paused-banner > svg {
  flex: 0 0 auto;
  color: var(--warning);
}

.paused-banner[data-state='error'] > svg {
  color: var(--danger);
}

.paused-banner p {
  flex: 1;
  margin: 0;
  line-height: 1.5;
}

.paused-banner p strong {
  margin-right: 8px;
  color: var(--text-primary);
  font-weight: 680;
}

.paused-banner button {
  flex: 0 0 auto;
  min-height: 30px;
  padding: 0 13px;
  border: 0;
  border-radius: var(--radius-sm);
  color: var(--text-inverse);
  background: var(--accent-strong);
  font: inherit;
  font-weight: 560;
  cursor: pointer;
  transition: filter 150ms ease;
}

.paused-banner button:hover {
  filter: brightness(1.08);
}

.paused-banner button:disabled {
  opacity: .55;
  cursor: default;
}

.paused-banner button:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}
</style>
