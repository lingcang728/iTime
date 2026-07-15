<script setup lang="ts">
import { PhX, PhClock, PhPulse, PhWarningCircle } from '@phosphor-icons/vue'
import { useAppStore } from '../stores/appStore'
import { formatClock, formatDuration } from '../utils/format'
const store = useAppStore()
</script>

<template>
  <Transition name="drawer">
    <div v-if="store.state.detailDrawerOpen" class="drawer-backdrop" @click.self="store.closeTool()">
      <aside class="detail-drawer" aria-label="AI 工具详情">
        <button class="icon-button drawer-close" type="button" aria-label="关闭详情" @click="store.closeTool()"><PhX :size="18" /></button>
        <template v-if="store.selectedTool.value">
          <div class="drawer-kicker">AI 工具详情</div>
          <h2>{{ store.selectedTool.value.toolName }}</h2>
          <div class="drawer-metrics">
            <div><PhClock :size="18" /><span>前台活动</span><strong>{{ formatDuration(store.selectedTool.value.foregroundDuration, true) }}</strong></div>
            <div><PhPulse :size="18" /><span>有效代理工时</span><strong>{{ formatDuration(store.selectedTool.value.effectiveDuration, true) }}</strong></div>
          </div>
          <section>
            <h3>运行区间</h3>
            <div v-for="event in store.selectedTool.value.workIntervals" :key="event.id" class="drawer-row">
              <span>{{ formatClock(event.start) }}—{{ formatClock(event.end) }}</span>
              <span>{{ event.taskId.split('-').slice(-2).join(' ') }}</span>
            </div>
          </section>
          <section>
            <h3>统计方式</h3>
            <p>有效代理区间按代理累加；前台活动区间取时间并集。</p>
            <h3 class="drawer-subheading">检测依据</h3>
            <p v-for="basis in store.selectedTool.value.detectionBasis" :key="basis">{{ basis }}</p>
            <div class="confidence-line"><span>置信度</span><strong>{{ Math.round(store.selectedTool.value.confidence * 100) }}%</strong></div>
          </section>
          <section v-if="store.selectedTool.value.pendingRecords.length" class="drawer-warning">
            <PhWarningCircle :size="20" />
            <div><h3>待确认记录</h3><p>{{ store.selectedTool.value.pendingRecords.length }} 条估算区间需要确认。</p></div>
          </section>
        </template>
      </aside>
    </div>
  </Transition>
</template>
