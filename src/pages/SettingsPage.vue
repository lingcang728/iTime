<script setup lang="ts">
import { computed } from 'vue'
import { PhDatabase, PhDesktop, PhDownloadSimple, PhKeyboard, PhMoon, PhPauseCircle, PhShieldCheck, PhSun, PhTrash, PhWarning } from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import { useAppStore, type MigrationState } from '../stores/appStore'

const store = useAppStore()
const migrationCopy = computed(() => ({
  notFound: { title: '未发现 KeyStats 数据', body: '你可以继续使用 iTime 的模拟输入数据，不会读取或修改现有 KeyStats。' },
  partial: { title: '发现部分 KeyStats 历史', body: '预览显示仅有累计快照，缺少分钟级历史。迁移仍不会执行。' },
  ready: { title: '迁移预览已准备', body: '适配层已识别版本边界；确认按钮仅演示交互。' },
  imported: { title: '迁移演示完成', body: '本原型没有读写任何真实 KeyStats 文件。' },
}[store.state.migrationState]))
const setMigration = (value: MigrationState) => { store.state.migrationState = value }
const deleteToday = () => store.deleteInputDate(store.state.selectedDate)
</script>

<template>
  <section class="page settings-page">
    <PageHeader title="设置" subtitle="应用、记录、隐私与本地数据" />
    <div class="settings-layout">
      <div class="settings-main">
        <article class="card settings-section">
          <div class="settings-heading"><PhSun :size="21" weight="duotone" /><div><h2>外观</h2><p>深色模式同步图表与文字对比度</p></div></div>
          <div class="theme-options" role="radiogroup" aria-label="主题">
            <label :class="{ active: store.state.theme === 'light' }"><input v-model="store.state.theme" type="radio" value="light"><PhSun :size="20" /><span>浅色</span></label>
            <label :class="{ active: store.state.theme === 'dark' }"><input v-model="store.state.theme" type="radio" value="dark"><PhMoon :size="20" /><span>深色</span></label>
            <label :class="{ active: store.state.theme === 'system' }"><input v-model="store.state.theme" type="radio" value="system"><PhDesktop :size="20" /><span>跟随系统</span></label>
          </div>
        </article>
        <article class="card settings-section">
          <div class="settings-heading"><PhPauseCircle :size="21" weight="duotone" /><div><h2>桌面行为</h2><p>侧边栏与托盘共享记录状态</p></div></div>
          <label class="setting-row"><div><strong>记录状态</strong><span>{{ store.state.recording ? '记录中' : '已暂停，模拟实时计数已冻结' }}</span></div><span class="switch"><input :checked="store.state.recording" type="checkbox" @change="store.setRecording(!store.state.recording)"><i></i></span></label>
          <label class="setting-row"><div><strong>关闭窗口时隐藏到托盘</strong><span>首次关闭仍会显示说明，可记住选择</span></div><span class="switch"><input v-model="store.state.hideToTray" type="checkbox"><i></i></span></label>
          <label class="setting-row"><div><strong>提醒开关</strong><span>控制托盘与页面中的主动提醒</span></div><span class="switch"><input v-model="store.state.reminders" type="checkbox"><i></i></span></label>
        </article>
        <article class="card settings-section">
          <div class="settings-heading"><PhKeyboard :size="21" weight="duotone" /><div><h2>输入统计隐私</h2><p>所有数据默认保存在本地</p></div></div>
          <label class="setting-row"><div><strong>键盘热力图</strong><span>仅保存单键累计次数</span></div><span class="switch"><input v-model="store.state.heatmapEnabled" type="checkbox"><i></i></span></label>
          <label class="setting-row"><div><strong>功能组合键统计</strong><span>只保存 Ctrl+C 等明确功能组合累计</span></div><span class="switch"><input v-model="store.state.shortcutsEnabled" type="checkbox"><i></i></span></label>
          <div class="privacy-boundaries"><PhShieldCheck :size="23" weight="duotone" /><p>不保存文字内容、可还原文字的事件序列或原始键盘事件；未来采集器必须排除密码框、Windows 安全桌面与敏感凭据界面。</p></div>
          <button class="danger-button" type="button" @click="deleteToday"><PhTrash :size="16" />删除 {{ store.state.selectedDate }} 的输入统计</button>
        </article>
      </div>
      <aside class="settings-side">
        <article class="card migration-card">
          <div class="settings-heading"><PhDatabase :size="21" weight="duotone" /><div><h2>KeyStats 迁移演示</h2><p>LegacyKeyStatsAdapter → 标准输入模型</p></div></div>
          <div class="migration-state"><PhWarning v-if="store.state.migrationState === 'partial'" :size="22" weight="duotone" /><PhDownloadSimple v-else :size="22" weight="duotone" /><div><strong>{{ migrationCopy.title }}</strong><p>{{ migrationCopy.body }}</p></div></div>
          <div class="migration-facts"><span><i>来源</i><strong>演示数据</strong></span><span><i>版本</i><strong>legacy-preview-v1</strong></span><span><i>能力</i><strong>累计快照 / 部分历史</strong></span></div>
          <div class="migration-actions">
            <button class="button secondary" type="button" @click="setMigration(store.state.migrationState === 'notFound' ? 'partial' : 'notFound')">切换发现状态</button>
            <button class="button primary" type="button" @click="setMigration('imported')">演示迁移</button>
          </div>
        </article>
        <article class="local-data-card"><PhShieldCheck :size="25" weight="duotone" /><div><span>本地优先</span><h2>原型不会访问真实 KeyStats</h2><p>当前仓库、安装、历史数据与启动状态均保持不变。</p></div></article>
      </aside>
    </div>
  </section>
</template>
