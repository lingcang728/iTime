<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { PhCheck } from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import { uiIcons } from '../data/uiIcons'
import { useAppStore } from '../stores/appStore'
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
const icons = {
  learning: uiIcons.goalLearning,
  development: uiIcons.goalDevelopment,
  ai: uiIcons.goalAi,
  continuous: uiIcons.goalContinuous,
}
const colors = { learning: '#50a874', development: '#4c79e8', ai: '#7664d8', continuous: '#d99538' }

function currentDuration(id: GoalId): number {
  if (id === 'learning') return store.day.value.apps.filter((app) => app.category === '学习').reduce((sum, app) => sum + app.duration, 0)
  if (id === 'development') return store.day.value.apps.filter((app) => app.category === '开发').reduce((sum, app) => sum + app.duration, 0)
  if (id === 'ai') return store.day.value.aiInteraction.value ?? 0
  return 0
}

const progressGoals = computed(() => goalDefinitions.filter((definition) => definition.id !== 'continuous').map((definition) => {
  const current = currentDuration(definition.id)
  const target = store.state.goals[definition.id] * 60_000
  return { ...definition, current, target, icon: icons[definition.id], color: colors[definition.id] }
}))
const overallProgress = computed(() => Math.round(progressGoals.value.reduce((total, goal) => total + Math.min(100, goal.current / goal.target * 100), 0) / progressGoals.value.length))
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
    <PageHeader title="提醒与目标" subtitle="按你的节奏设定边界；所有目标都可随时调整" />
    <div class="goal-hero">
      <div><span>今日目标</span><h1>{{ reachedCount ? `${reachedCount} 个目标已经达成` : '按自己的节奏继续' }}</h1><p>进度来自本机活动记录，只用于提醒，不评价工作效率。</p></div>
      <div class="hero-target"><img class="hero-target__icon" :src="uiIcons.goalSave" alt="" draggable="false" /><strong>{{ overallProgress }}%</strong><span>综合进度</span></div>
    </div>

    <div class="goal-grid">
      <article v-for="goal in progressGoals" :key="goal.id" class="card goal-card">
        <div class="goal-card__top">
          <span class="goal-icon goal-icon--art" :style="{ background: `${goal.color}16` }">
            <img :src="goal.icon" alt="" draggable="false" />
          </span>
          <small>{{ Math.min(100, Math.round(goal.current / goal.target * 100)) }}%</small>
        </div>
        <span>{{ goal.label }}</span><strong>{{ formatDuration(goal.current, true) }} <i>/ {{ formatDuration(goal.target, true) }}</i></strong>
        <div class="goal-progress"><i :style="{ width: `${Math.min(100, goal.current / goal.target * 100)}%`, background: goal.color }"></i></div>
        <p>{{ goal.current >= goal.target ? '今天已达到设定目标' : `还差 ${formatDuration(goal.target - goal.current, true)}` }}</p>
      </article>
    </div>

    <div class="goals-layout">
      <article class="card editor-card">
        <header>
          <div><span>目标编辑</span><h2>设置每天想投入的分钟数</h2><p>保存后会同步到首页目标环和提醒。</p></div>
          <img class="editor-card__icon" :src="uiIcons.goalSave" alt="" draggable="false" />
        </header>
        <div class="target-form">
          <label v-for="definition in goalDefinitions" :key="definition.id">
            <span>{{ definition.label }}</span>
            <div><input v-model="goalDraft[definition.id]" inputmode="numeric" type="number" :min="definition.min" :max="definition.max" step="1"><b>分钟</b></div>
            <small>{{ definition.hint }}</small>
          </label>
        </div>
        <div class="form-footer"><p :class="{ error: targetError }" role="status" aria-live="polite">{{ targetMessage || '请输入整数分钟；超出合理范围时不会保存。' }}</p><button class="save-button" type="button" @click="saveTargets"><PhCheck :size="17" />保存目标</button></div>
      </article>

      <div class="reminder-stack">
        <article class="card reminder-card">
          <span class="reminder-icon green reminder-icon--art"><img :src="uiIcons.goalContinuous" alt="" draggable="false" /></span>
          <div><span>连续使用提醒</span><h2>每 {{ store.state.goals.continuous }} 分钟提醒休息</h2><p>达到阈值后显示一次温和提醒。</p></div>
          <label class="toggle"><input v-model="store.state.reminders" type="checkbox"><i></i></label>
        </article>
        <article class="card quiet-card">
          <header>
            <span class="reminder-icon violet reminder-icon--art"><img :src="uiIcons.goalQuiet" alt="" draggable="false" /></span>
            <div><span>安静时段</span><h2>记录继续，通知暂停</h2></div>
          </header>
          <div class="time-range"><label><span>开始</span><input v-model="quietDraft.start" type="time"></label><b>至</b><label><span>结束</span><input v-model="quietDraft.end" type="time"></label></div>
          <div class="quiet-footer"><p :class="{ error: quietError }" role="status" aria-live="polite">{{ quietMessage || '支持跨午夜时段，例如 22:30 至 08:00。' }}</p><button type="button" @click="saveQuietHours">保存时段</button></div>
        </article>
      </div>
    </div>
  </section>
</template>

<style scoped src="./goals-page.css"></style>
