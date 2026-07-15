<script setup lang="ts">
import { computed } from 'vue'
import { PhBell, PhBrain, PhClock, PhCode, PhMoonStars, PhRobot, PhTarget } from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import { useAppStore } from '../stores/appStore'
import { formatDuration } from '../utils/format'

const store = useAppStore()
const goals = computed(() => [
  { id: 'learning', label: '学习时间', icon: PhBrain, color: '#55ae7d', current: store.day.value.apps.filter((app) => app.category === '学习').reduce((sum, app) => sum + app.duration, 0), target: store.state.goals.learning * 60_000 },
  { id: 'development', label: '开发时间', icon: PhCode, color: '#5a82db', current: store.day.value.apps.filter((app) => app.category === '开发').reduce((sum, app) => sum + app.duration, 0), target: store.state.goals.development * 60_000 },
  { id: 'ai', label: 'AI 有效工时', icon: PhRobot, color: '#806be1', current: store.day.value.aiEffective.value ?? 0, target: store.state.goals.ai * 60_000 },
])
</script>

<template>
  <section class="page goals-page">
    <PageHeader title="提醒与目标" subtitle="用温和的边界帮助自己安排时间，不比较输入量高低" />
    <div class="goal-hero">
      <div><span>今日进度</span><h1>3 个目标中，2 个正在稳步推进</h1><p>提醒只基于你设置的时间边界，不对工作效率作价值判断。</p></div>
      <div class="hero-target"><PhTarget :size="38" weight="duotone" /><strong>68%</strong><span>综合进度</span></div>
    </div>
    <div class="goal-grid">
      <article v-for="goal in goals" :key="goal.id" class="card goal-card">
        <div class="goal-card__icon" :style="{ color: goal.color, background: `${goal.color}18` }"><component :is="goal.icon" :size="22" weight="duotone" /></div>
        <span>{{ goal.label }}</span><strong>{{ formatDuration(goal.current, true) }} / {{ formatDuration(goal.target, true) }}</strong>
        <div class="goal-progress"><i :style="{ width: `${Math.min(100, goal.current / goal.target * 100)}%`, background: goal.color }"></i></div>
        <small>{{ goal.current >= goal.target ? '今日目标已达到' : `还差 ${formatDuration(goal.target - goal.current, true)}` }}</small>
      </article>
    </div>
    <div class="reminder-grid">
      <article class="card reminder-card"><div class="reminder-icon green"><PhClock :size="23" weight="duotone" /></div><div><span>连续使用提醒</span><h2>每 50 分钟轻声提醒</h2><p>当前连续活动 42 分钟，下一次提醒约在 8 分钟后。</p></div><label class="switch"><input v-model="store.state.reminders" type="checkbox"><span></span></label></article>
      <article class="card reminder-card"><div class="reminder-icon violet"><PhMoonStars :size="23" weight="duotone" /></div><div><span>安静时段</span><h2>22:30—08:00 不打扰</h2><p>安静时段内保留记录，不显示主动提醒。</p></div><button class="button secondary" type="button">编辑</button></article>
      <article class="card reminder-card"><div class="reminder-icon orange"><PhBell :size="23" weight="duotone" /></div><div><span>AI 任务完成</span><h2>仅在后台任务完成时通知</h2><p>合并 2 分钟内的连续完成消息，减少打断。</p></div><label class="switch"><input type="checkbox" checked><span></span></label></article>
    </div>
  </section>
</template>
