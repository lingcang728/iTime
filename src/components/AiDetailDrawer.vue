<script setup lang="ts">
import { PhX, PhClock, PhHourglass, PhIntersect, PhPulse, PhWarningCircle } from '@phosphor-icons/vue'
import { useAppStore } from '../stores/appStore'
import { formatClock, formatDuration } from '../utils/format'
const store = useAppStore()
const statusLabels = { running: '运行中', completed: '已完成', waiting: '等待中' }
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
            <div><PhPulse :size="18" /><span>状态</span><strong>{{ statusLabels[store.selectedTool.value.status] }}</strong></div>
            <div><PhClock :size="18" /><span>运行时长</span><strong>{{ formatDuration(store.selectedTool.value.effectiveDuration, true) }}</strong></div>
            <div><PhPulse :size="18" /><span>前台交互</span><strong>{{ formatDuration(store.selectedTool.value.foregroundDuration, true) }}</strong></div>
            <div><PhHourglass :size="18" /><span>静默等待</span><strong>{{ formatDuration(store.selectedTool.value.silentWaitDuration, true) }}</strong></div>
            <div><PhIntersect :size="18" /><span>并行重叠</span><strong>{{ formatDuration(store.selectedTool.value.parallelOverlapDuration, true) }}</strong></div>
          </div>
          <section>
            <h3>运行区间</h3>
            <div v-for="event in store.selectedTool.value.workIntervals" :key="event.id" class="drawer-row">
              <span>{{ formatClock(event.start) }}—{{ formatClock(event.end) }}</span>
              <span>有效执行</span>
            </div>
            <div v-for="event in store.selectedTool.value.interactionIntervals" :key="event.id" class="drawer-row"><span>{{ formatClock(event.start) }}—{{ formatClock(event.end) }}</span><span>前台交互</span></div>
            <div v-for="(event, index) in store.selectedTool.value.waitIntervals" :key="index" class="drawer-row"><span>{{ formatClock(event.start) }}—{{ formatClock(event.end) }}</span><span>静默等待</span></div>
          </section>
          <section>
            <h3>检测依据</h3>
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
