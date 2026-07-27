<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  PhArrowClockwise,
  PhChartBar,
  PhDatabase,
  PhDesktop,
  PhDownloadSimple,
  PhFolderOpen,
  PhHardDrives,
  PhKeyboard,
  PhMoon,
  PhPalette,
  PhPower,
  PhPulse,
  PhRobot,
  PhShieldCheck,
  PhSun,
  PhTerminalWindow,
  PhTrash,
  PhTray,
} from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import { isTauriRuntime } from '../platform/desktop'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()
const desktopControlsAvailable = isTauriRuntime()
const deleteArmed = ref(false)

const inputStatusLabel = computed(() => ({
  loading: '正在连接',
  preview: '预览数据',
  empty: '等待首条记录',
  ready: '本机已连接',
  degraded: '部分可用',
  error: '读取失败',
}[store.state.inputDataStatus]))

const autostartStatusLabel = computed(() => ({
  loading: '正在向 Windows 确认',
  ready: store.state.autostartMessage || (store.state.autostartEnabled ? '系统启动项已启用' : '系统启动项未启用'),
  error: store.state.autostartMessage || '无法读取系统启动项',
}[store.state.autostartStatus]))

const providerStatusLabel = computed(() => ({
  disabled: '未授权',
  loading: '正在读取',
  preview: '预览数据',
  empty: '暂无执行记录',
  ready: '本机已连接',
  degraded: '部分可用',
  error: '读取失败',
}[store.state.providerDataStatus]))

const providerEnabled = computed(() => (
  store.state.providerConsent.codexEnabled || store.state.providerConsent.claudeEnabled
))

const localDataStatusLabel = computed(() => ({
  loading: '正在处理',
  preview: '桌面版功能',
  empty: '暂无本地记录',
  degraded: '部分记录可恢复',
  error: '数据操作失败',
  ready: '本地数据正常',
}[store.state.localDataStatus]))

const localDataBusy = computed(() => store.state.localDataBusy !== null)
const retentionValue = computed(() => store.state.localData.retentionDays?.toString() ?? 'permanent')

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function formatMoment(value: number | null): string {
  if (value === null) return '暂无'
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(value)
}

const localDataRange = computed(() => {
  const { startAt, endAt } = store.state.localData
  if (startAt === null || endAt === null) return '暂无可导出记录'
  return `${formatMoment(startAt)} – ${formatMoment(endAt)}`
})

const inputFacts = computed(() => {
  const capabilities = store.input.value.capabilities
  const historyLabel = {
    minute: '分钟级',
    hour: '小时级',
    day: '日级',
    none: '无历史序列',
  }[capabilities.historyGranularity]
  return [
    { label: '统计方式', value: 'Windows 字符键按下计数', icon: PhKeyboard },
    { label: '聚合粒度', value: historyLabel, icon: PhPulse },
    { label: '输入内容', value: '从不保存', icon: PhShieldCheck },
    { label: '本地存储', value: 'iTime Data · JSONL', icon: PhHardDrives },
  ]
})

function checkedValue(event: Event): boolean {
  return event.currentTarget instanceof HTMLInputElement && event.currentTarget.checked
}

function updateClosePreference(event: Event): void {
  store.state.closePreference = checkedValue(event) ? 'hide' : 'ask'
}

async function updateAutostart(event: Event): Promise<void> {
  await store.setAutostart(checkedValue(event))
}

async function acknowledgeProviderNotice(): Promise<void> {
  await store.updateProviderConsent({ noticeSeen: true })
}

async function updateCodexAccess(event: Event): Promise<void> {
  await store.updateProviderConsent({ noticeSeen: true, codexEnabled: checkedValue(event) })
}

async function updateClaudeAccess(event: Event): Promise<void> {
  await store.updateProviderConsent({ noticeSeen: true, claudeEnabled: checkedValue(event) })
}

async function updateRetention(event: Event): Promise<void> {
  const value = event.currentTarget instanceof HTMLSelectElement ? event.currentTarget.value : 'permanent'
  await store.updateDataRetention(value === 'permanent' ? null : Number(value) as 90 | 365)
}

async function confirmClearLocalData(): Promise<void> {
  if (await store.clearLocalRecords()) deleteArmed.value = false
}

onMounted(() => {
  void store.refreshAutostart()
  void store.refreshLocalData()
})
</script>

<template>
  <section class="page settings-page">
    <PageHeader title="设置" subtitle="自定义 iTime 的行为与外观，管理输入统计与隐私设置。" />
    <div class="settings-layout">
      <div class="settings-main">
        <section class="settings-group" aria-labelledby="startup-title">
          <header class="settings-group__header">
            <div><h2 id="startup-title">启动与窗口</h2><p>这些选项会影响 iTime 在 Windows 中的实际运行方式。</p></div>
          </header>
          <div class="settings-list">
            <label class="control-row">
              <span class="control-icon"><PhPower :size="20" /></span>
              <div><strong>开机自启动</strong><span>登录 Windows 后自动启动 iTime，不必手动打开。</span><small :class="['system-status', store.state.autostartStatus]">{{ autostartStatusLabel }}</small></div>
              <span class="toggle"><input :checked="store.state.autostartEnabled" :disabled="store.state.autostartStatus === 'loading'" type="checkbox" @change="updateAutostart"><i></i></span>
            </label>
            <label class="control-row">
              <span class="control-icon"><PhTray :size="20" /></span>
              <div><strong>关闭窗口时隐藏到托盘</strong><span>保留本机记录进程，可从托盘再次打开。</span></div>
              <span class="toggle"><input :checked="store.state.closePreference === 'hide'" type="checkbox" @change="updateClosePreference"><i></i></span>
            </label>
            <label class="control-row">
              <span class="control-icon"><PhChartBar :size="20" /></span>
              <div><strong>活动记录</strong><span>{{ store.state.recordingMessage }}</span></div>
              <span class="toggle"><input :checked="store.state.recording" :disabled="store.state.recordingStatus === 'loading'" type="checkbox" @change="store.setRecording(!store.state.recording)"><i></i></span>
            </label>
          </div>
        </section>

        <section class="settings-group" aria-labelledby="privacy-title">
          <header class="settings-group__header">
            <div><h2 id="privacy-title">输入统计与隐私</h2><p>只保存聚合计数，不保存输入内容或可还原文字的事件序列。</p></div>
          </header>
          <div class="privacy-note"><PhShieldCheck :size="20" weight="regular" aria-hidden="true" /><p>iTime 只累计字符键按下次数并按分钟保存数量；不保存具体键值、键盘文字、密码内容、语音输入或剪贴板正文。</p></div>
        </section>

        <section class="settings-group provider-section" aria-labelledby="provider-title">
          <header class="settings-group__header">
            <div><h2 id="provider-title">Provider 本机会话授权</h2><p>Codex 与 Claude Code 分别授权；未启用的数据源不会枚举目录或读取文件。</p></div>
          </header>
          <div v-if="!store.state.providerConsent.noticeSeen" class="provider-consent" role="note" aria-labelledby="provider-consent-title">
            <PhShieldCheck :size="24" weight="regular" aria-hidden="true" />
            <div>
              <strong id="provider-consent-title">先了解读取边界，再选择是否启用</strong>
              <p>启用后，iTime 仅在对应本机会话目录中读取时间、事件类型、时长和文件更新时间，用于计算 Provider 执行区间。不会访问、保存或显示消息正文、提示词、回复内容和代码内容。关闭后立即停止扫描，并清除内存缓存。</p>
              <button type="button" :disabled="store.state.providerConsentStatus === 'loading'" @click="acknowledgeProviderNotice">我已了解，选择数据源</button>
            </div>
          </div>
          <template v-else>
            <div class="settings-list provider-list">
              <label class="control-row">
                <span class="control-icon"><PhTerminalWindow :size="20" /></span>
                <div><strong>Codex 本机会话</strong><span>只读 %USERPROFILE%\.codex\sessions 中的任务开始、完成与中止时间事件。</span></div>
                <span class="toggle"><input :checked="store.state.providerConsent.codexEnabled" :disabled="store.state.providerConsentStatus === 'loading'" type="checkbox" @change="updateCodexAccess"><i></i></span>
              </label>
              <label class="control-row">
                <span class="control-icon"><PhRobot :size="20" /></span>
                <div><strong>Claude Code 本机会话</strong><span>只读 %USERPROFILE%\.claude\projects 中的用户回合、结束与时长元数据。</span></div>
                <span class="toggle"><input :checked="store.state.providerConsent.claudeEnabled" :disabled="store.state.providerConsentStatus === 'loading'" type="checkbox" @change="updateClaudeAccess"><i></i></span>
              </label>
            </div>
            <div :class="['source-status', 'provider-source-status', store.state.providerDataStatus]">
              <span class="status-dot"></span><div><strong>{{ providerStatusLabel }}</strong><p>{{ store.state.providerDataMessage }}</p></div>
              <button v-if="providerEnabled" type="button" :disabled="store.state.providerDataStatus === 'loading'" @click="store.refreshProviderData"><PhArrowClockwise :size="16" />刷新</button>
            </div>
          </template>
        </section>

        <section class="settings-group local-data-section" aria-labelledby="local-data-title">
          <header class="settings-group__header">
            <div><h2 id="local-data-title">本地数据管理</h2><p>检查存储范围，导出可重新读取的记录，或明确删除全部本地统计。</p></div>
          </header>
          <div :class="['source-status', 'local-data-status', store.state.localDataStatus]" role="status" aria-live="polite">
            <span class="status-dot"></span>
            <div><strong>{{ localDataStatusLabel }}</strong><p>{{ store.state.localDataMessage }}</p></div>
          </div>
          <dl class="local-data-facts">
            <div><dt>数据位置</dt><dd :title="store.state.localData.directory">{{ store.state.localData.directory }}</dd></div>
            <div><dt>记录范围</dt><dd>{{ localDataRange }}</dd></div>
            <div><dt>最后写入</dt><dd>{{ formatMoment(store.state.localData.lastWriteAt) }}</dd></div>
            <div><dt>占用空间</dt><dd>{{ formatBytes(store.state.localData.sizeBytes) }} · {{ store.state.localData.fileCount }} 个分片</dd></div>
            <div><dt>导出内容</dt><dd>{{ store.state.localData.activityRecords }} 条活动 · {{ store.state.localData.keyboardRecords }} 条字符键计数</dd></div>
          </dl>
          <label class="retention-control">
            <span><strong>自动保留期</strong><small>升级后默认永久；只清理过期且已关闭的日期分片。</small></span>
            <select :value="retentionValue" :disabled="!desktopControlsAvailable || localDataBusy" @change="updateRetention">
              <option value="permanent">永久</option>
              <option value="365">365 天</option>
              <option value="90">90 天</option>
            </select>
          </label>
          <div class="local-data-actions">
            <button type="button" :disabled="!desktopControlsAvailable || localDataBusy" @click="store.openLocalData"><PhFolderOpen :size="17" />打开目录</button>
            <button type="button" :disabled="!desktopControlsAvailable || localDataBusy" @click="store.exportLocalRecords('json')"><PhDownloadSimple :size="17" />导出 JSON</button>
            <button type="button" :disabled="!desktopControlsAvailable || localDataBusy" @click="store.exportLocalRecords('csv')"><PhDownloadSimple :size="17" />导出 CSV</button>
            <button class="danger-action" type="button" :disabled="!desktopControlsAvailable || localDataBusy" @click="deleteArmed = true"><PhTrash :size="17" />删除全部</button>
          </div>
          <p v-if="store.state.localDataExportMessage" class="export-result">{{ store.state.localDataExportMessage }}</p>
          <div v-if="deleteArmed" class="delete-confirmation" role="alert">
            <PhTrash :size="21" aria-hidden="true" />
            <div><strong>确认删除全部本地活动与字符键计数？</strong><p>iTime 会先暂停采集并刷新文件，导出文件不会删除。此操作无法撤销。</p></div>
            <button type="button" :disabled="localDataBusy" @click="deleteArmed = false">取消</button>
            <button class="confirm-delete" type="button" :disabled="localDataBusy" @click="confirmClearLocalData">确认删除</button>
          </div>
        </section>

        <section class="settings-group appearance-section" aria-labelledby="appearance-title">
          <header class="settings-group__header">
            <div><h2 id="appearance-title">外观</h2><p>选择适合当前 Windows 桌面的显示方式。</p></div>
          </header>
          <div class="theme-options" role="radiogroup" aria-label="主题">
            <span class="theme-icon"><PhPalette :size="20" /><i><strong>主题</strong><small>选择 iTime 的外观主题</small></i></span>
            <label :class="{ active: store.state.theme === 'light' }"><input v-model="store.state.theme" type="radio" value="light"><PhSun :size="18" weight="regular" /><span>浅色</span></label>
            <label :class="{ active: store.state.theme === 'dark' }"><input v-model="store.state.theme" type="radio" value="dark"><PhMoon :size="18" weight="regular" /><span>深色</span></label>
            <label :class="{ active: store.state.theme === 'system' }"><input v-model="store.state.theme" type="radio" value="system"><PhDesktop :size="18" weight="regular" /><span>跟随系统</span></label>
          </div>
        </section>
      </div>

      <aside class="settings-side">
        <section class="source-panel" aria-labelledby="source-title">
          <header class="settings-group__header">
            <div><h2 id="source-title">本机键盘计数</h2><p>查看 Windows 字符键计数器的当前状态。</p></div>
          </header>
          <div :class="['source-status', store.state.inputDataStatus]">
            <span class="status-dot"></span><div><strong>{{ inputStatusLabel }}</strong><p>{{ store.state.inputDataMessage }}</p></div>
          </div>
          <dl class="source-facts">
            <div v-for="fact in inputFacts" :key="fact.label"><component :is="fact.icon" :size="19" /><dt>{{ fact.label }}</dt><dd>{{ fact.value }}</dd></div>
          </dl>
          <button class="refresh-button" type="button" :disabled="store.state.inputDataStatus === 'loading'" @click="store.refreshInputData">
            <PhArrowClockwise :size="17" weight="regular" />刷新键盘计数
          </button>
        </section>

        <section class="data-boundary">
          <PhDatabase :size="22" weight="regular" aria-hidden="true" />
          <div><span>数据边界</span><h2>接入前历史不会被补造</h2><p>键盘计数从本次版本启动后开始；应用活动来自 iTime 采集器。Provider 活动仅在用户逐项授权后读取对应本机会话时间元数据。</p></div>
        </section>
      </aside>
    </div>
  </section>
</template>

<style scoped src="./settings-page.css"></style>
