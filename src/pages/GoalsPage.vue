<script setup lang="ts">
import { computed, markRaw, reactive, ref, type Component } from 'vue'
import {
  PhBell,
  PhBookOpen,
  PhCheck,
  PhCode,
  PhFloppyDisk,
  PhMoon,
  PhRobot,
  PhTarget,
  PhTimer,
} from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import { useAppStore } from '../stores/appStore'
import { hasActivityData } from '../stores/dataAvailability'
import { formatDuration } from '../utils/format'
import { goalDefinitions, validateGoalDraft, validateQuietRange, type GoalDraft, type GoalId } from './goalsForm'

const store = useAppStore()
const targetMessage = ref('')
const targetError = ref(false)
const quietMessage = ref('')
const quietError = ref(false)
const goalDraft = reactive<GoalDraft>({
  learning: String(store.state.goals.learning),
  development: String(store.state.goals.development),
  ai: String(store.state.goals.ai),
  continuous: String(store.state.goals.continuous),
})
const quietDraft = reactive({ start: store.state.quietStart, end: store.state.quietEnd })
const activityDataAvailable = computed(() => hasActivityData(store.state.activityDataStatus))
const sourceStateTitle = computed(() => {
  if (store.state.activityDataStatus === 'loading') return '正在读取目标进度'
  if (store.state.activityDataStatus === 'empty') return '暂无可计算的目标进度'
  return '目标进度读取失败'
})
const goalIcons: Record<GoalId, Component> = {
  learning: markRaw(PhBookOpen),
  development: markRaw(PhCode),
  ai: markRaw(PhRobot),
  continuous: markRaw(PhTimer),
}

function currentDuration(id: GoalId): number {
  if (id === 'learning') return store.day.value.apps.filter((app) => app.category === '学习').reduce((sum, app) => sum + app.duration, 0)
  if (id === 'development') return store.day.value.apps.filter((app) => app.category === '开发').reduce((sum, app) => sum + app.duration, 0)
  if (id === 'ai') return store.day.value.aiInteraction.value ?? 0
  return 0
}

const progressGoals = computed(() => goalDefinitions.filter((definition) => definition.id !== 'continuous').map((definition) => {
  const current = currentDuration(definition.id)
  const target = Math.max(1, store.state.goals[definition.id]) * 60_000
  const progress = Math.min(100, current / target * 100)
  return { ...definition, current, target, progress, percentage: Math.round(progress), icon: goalIcons[definition.id] }
}))
const overallProgress = computed(() => Math.round(progressGoals.value.reduce((total, goal) => total + goal.progress, 0) / progressGoals.value.length))
const reachedCount = computed(() => progressGoals.value.filter((goal) => goal.current >= goal.target).length)

function saveTargets(): void {
  const result = validateGoalDraft(goalDraft)
  targetError.value = !result.ok
  targetMessage.value = result.ok ? '目标已保存，首页进度与提醒阈值已同步更新。' : result.message
  if (!result.ok) return
  store.state.goals = { ...store.state.goals, ...result.values }
  store.showToast('提醒目标已保存')
}

function saveQuietHours(): void {
  const result = validateQuietRange(quietDraft.start, quietDraft.end)
  quietError.value = !result.ok
  quietMessage.value = result.ok ? `已保存：${quietDraft.start}—${quietDraft.end} 不主动提醒。` : result.message
  if (!result.ok) return
  store.state.quietStart = quietDraft.start
  store.state.quietEnd = quietDraft.end
  store.showToast('安静时段已保存')
}
</script>

<template>
  <section class="page goals-page">
    <PageHeader title="提醒与目标" subtitle="每日目标与休息提醒" />
    <section v-if="activityDataAvailable" class="goal-overview" aria-labelledby="goal-overview-title">
      <div class="goal-overview__copy">
        <span>今日</span>
        <h2 id="goal-overview-title">{{ reachedCount ? `已达成 ${reachedCount} 项` : '继续推进' }}</h2>
        <p>仅作提醒，不评价效率</p>
      </div>
      <div class="goal-overview__progress" aria-label="综合进度">
        <PhTarget :size="24" weight="regular" aria-hidden="true" />
        <strong>{{ overallProgress }}%</strong>
        <span>综合进度</span>
        <div class="goal-progress" aria-hidden="true"><i :style="{ width: `${overallProgress}%` }"></i></div>
      </div>
    </section>

    <section v-if="activityDataAvailable" class="goal-stats" aria-label="今日目标进度">
      <article v-for="goal in progressGoals" :key="goal.id" class="goal-stat">
        <header>
          <span class="goal-stat__icon"><component :is="goal.icon" :size="19" weight="regular" aria-hidden="true" /></span>
          <span>{{ goal.label }}</span>
          <small>{{ goal.percentage }}%</small>
        </header>
        <strong>{{ formatDuration(goal.current, true) }} <i>/ {{ formatDuration(goal.target, true) }}</i></strong>
        <div class="goal-progress" aria-hidden="true"><i :style="{ width: `${goal.progress}%` }"></i></div>
        <p>{{ goal.current >= goal.target ? '已达成' : `还差 ${formatDuration(goal.target - goal.current, true)}` }}</p>
      </article>
    </section>
    <div v-else class="section-state goals-source-state" :data-state="store.state.activityDataStatus">
      <strong>{{ sourceStateTitle }}</strong><span>{{ store.state.activityDataMessage }}</span>
    </div>

    <div class="goals-layout">
      <section class="goal-editor" aria-labelledby="goal-editor-title">
        <header>
          <span class="section-line-icon"><PhFloppyDisk :size="20" weight="regular" aria-hidden="true" /></span>
          <div><span>编辑</span><h2 id="goal-editor-title">每日目标（分钟）</h2></div>
        </header>
        <div class="target-form">
          <label v-for="definition in goalDefinitions" :key="definition.id">
            <span>{{ definition.label }}</span>
            <div><input v-model="goalDraft[definition.id]" inputmode="numeric" type="number" :min="definition.min" :max="definition.max" step="1"><b>分钟</b></div>
            <small>{{ definition.hint }}</small>
          </label>
        </div>
        <div class="form-footer"><p :class="{ error: targetError }" role="status" aria-live="polite">{{ targetMessage || '请输入整数分钟' }}</p><button class="save-button" type="button" @click="saveTargets"><PhCheck :size="17" />保存</button></div>
      </section>

      <aside class="reminder-panel" aria-label="提醒设置">
        <section class="reminder-row">
          <span class="section-line-icon"><PhBell :size="20" weight="regular" aria-hidden="true" /></span>
          <div><span>连续使用</span><h2>每 {{ store.state.goals.continuous }} 分钟提醒</h2></div>
          <label class="toggle"><input v-model="store.state.reminders" type="checkbox" aria-label="启用连续使用休息提醒"><i></i></label>
        </section>
        <section class="quiet-section">
          <header>
            <span class="section-line-icon"><PhMoon :size="20" weight="regular" aria-hidden="true" /></span>
            <div><span>安静时段</span><h2>记录继续，通知暂停</h2></div>
          </header>
          <div class="time-range"><label><span>开始</span><input v-model="quietDraft.start" type="time"></label><b>至</b><label><span>结束</span><input v-model="quietDraft.end" type="time"></label></div>
          <div class="quiet-footer"><p :class="{ error: quietError }" role="status" aria-live="polite">{{ quietMessage || '可跨午夜，如 22:30–08:00' }}</p><button type="button" @click="saveQuietHours">保存</button></div>
        </section>
      </aside>
    </div>
  </section>
</template>

<style scoped src="./goals-page.css"></style>
