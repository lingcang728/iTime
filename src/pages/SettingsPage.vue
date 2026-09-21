<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import {
  PhArrowClockwise,
  PhChartBar,
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
  PhTrash,
  PhTray,
  PhUploadSimple,
} from '@phosphor-icons/vue'
import PageHeader from '../components/PageHeader.vue'
import { isTauriRuntime } from '../platform/desktop'
import { useAppStore } from '../stores/appStore'
import {
  checkForDesktopUpdate,
  downloadAndInstallDesktopUpdate,
  updateState,
} from '../services/updateService'

const store = useAppStore()
const desktopControlsAvailable = isTauriRuntime()
const deleteArmed = ref(false)
const updateInstallArmed = ref(false)
const aiConsentArmed = ref(false)
const deleteConfirmation = ref<HTMLElement | null>(null)
const updateConfirmation = ref<HTMLElement | null>(null)
const aiConsentConfirmation = ref<HTMLElement | null>(null)

// 内联确认条出现后把焦点交给「取消」，Esc 就地撤销 —— 危险操作不走捷径。
watch(deleteArmed, async (armed) => {
  if (!armed) return
  await nextTick()
  deleteConfirmation.value?.querySelector('button')?.focus()
})
watch(updateInstallArmed, async (armed) => {
  if (!armed) return
  await nextTick()
  updateConfirmation.value?.querySelector('button')?.focus()
})
watch(aiConsentArmed, async (armed) => {
  if (!armed) return
  await nextTick()
  aiConsentConfirmation.value?.querySelector('button')?.focus()
})

const inputStatusLabel = computed(() => ({
  loading: '连接中',
  preview: '预览',
  empty: '等待首条记录',
  ready: '已连接',
  degraded: '部分可用',
  error: '读取失败',
}[store.state.inputDataStatus]))

const autostartStatusLabel = computed(() => ({
  loading: '确认中',
  ready: store.state.autostartMessage || (store.state.autostartEnabled ? '已启用' : '未启用'),
  error: store.state.autostartMessage || '读取失败',
}[store.state.autostartStatus]))

const providerStatusLabel = computed(() => ({
  disabled: '未授权',
  loading: '读取中',
  preview: '预览',
  empty: '暂无记录',
  ready: '已连接',
  degraded: '部分可用',
  error: '读取失败',
}[store.state.providerDataStatus]))

const providerEnabled = computed(() => (
  store.state.providerConsent.aiAgentToolsEnabled
))

const localDataStatusLabel = computed(() => ({
  loading: '处理中',
  preview: '仅桌面版',
  empty: '暂无数据',
  degraded: '部分可恢复',
  error: '操作失败',
  ready: '正常',
}[store.state.localDataStatus]))

const localDataBusy = computed(() => store.state.localDataBusy !== null)
const updateBusy = computed(() => ['checking', 'downloading', 'installing'].includes(updateState.status))
const updateProgress = computed(() => {
  if (!updateState.totalBytes) return null
  return Math.min(100, Math.round(updateState.downloadedBytes / updateState.totalBytes * 100))
})
const updateStatusLabel = computed(() => ({
  idle: '未检查',
  checking: '检查中',
  available: `有新版本 ${updateState.version}`,
  downloading: '下载中',
  installing: '安装中',
  failed: '更新失败',
  // 24h 节流命中时 upToDate 只是"跳过了自动检查"，如实呈现而非宣称已验证最新。
  upToDate: updateState.autoCheckThrottled ? '今日已自动检查' : '已是最新',
}[updateState.status]))
const retentionValue = computed(() => store.state.localData.retentionDays?.toString() ?? 'permanent')

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function formatUpdateDate(value: string): string {
  if (!value) return '无发布时间'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).format(date)
}

function formatMoment(value: number | null): string {
  if (value === null) return '—'
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(value)
}

const localDataRange = computed(() => {
  const { startAt, endAt } = store.state.localData
  if (startAt === null || endAt === null) return '无可导出记录'
  return `${formatMoment(startAt)} – ${formatMoment(endAt)}`
})

const inputFacts = computed(() => {
  const capabilities = store.input.value.capabilities
  const historyLabel = {
    minute: '分钟',
    hour: '小时',
    day: '天',
    none: '无',
  }[capabilities.historyGranularity]
  return [
    { label: '方式', value: '字符键计数', icon: PhKeyboard },
    { label: '粒度', value: historyLabel, icon: PhPulse },
    { label: '内容', value: '不保存', icon: PhShieldCheck },
    { label: '存储', value: '仅本机文件', icon: PhHardDrives },
  ]
})

function checkedValue(event: Event): boolean {
  return event.currentTarget instanceof HTMLInputElement && event.currentTarget.checked
}

function updateClosePreference(event: Event): void {
  // 三态：每次询问 / 隐藏到托盘 / 直接退出 —— 与关闭对话框的选择一一对应。
  const value = event.currentTarget instanceof HTMLSelectElement ? event.currentTarget.value : 'ask'
  if (value === 'ask' || value === 'hide' || value === 'quit') {
    store.state.closePreference = value
  }
}

async function updateRecording(event: Event): Promise<void> {
  const input = event.currentTarget instanceof HTMLInputElement ? event.currentTarget : null
  await store.setRecording(checkedValue(event))
  // F-16: 与 autostart/consent 同一约定——失败立即回弹勾选态。
  if (input && store.state.recordingStatus === 'error') {
    input.checked = store.state.recording
  }
}

async function updateAutostart(event: Event): Promise<void> {
  const input = event.currentTarget instanceof HTMLInputElement ? event.currentTarget : null
  await store.setAutostart(checkedValue(event))
  // F-16: 失败时 state 未被修改，主动回弹 DOM 勾选态，不等 error 状态触发的 re-render。
  if (input && store.state.autostartStatus === 'error') {
    input.checked = store.state.autostartEnabled
  }
}

async function updateAiAgentToolsAccess(event: Event): Promise<void> {
  const input = event.currentTarget instanceof HTMLInputElement ? event.currentTarget : null
  const enabled = checkedValue(event)
  // First-ever enable must surface the read-scope notice *before* consent is
  // persisted — the backend rejects `enabled` without `noticeSeen`, and the
  // notice is what makes the consent real rather than a one-click side effect.
  if (enabled && !store.state.providerConsent.noticeSeen) {
    if (input) input.checked = false
    aiConsentArmed.value = true
    return
  }
  await store.updateProviderConsent(enabled
    ? { noticeSeen: true, aiAgentToolsEnabled: true }
    : { aiAgentToolsEnabled: false })
  // F-16: 保存失败立即回弹勾选态，与后端真实状态保持一致。
  if (input && store.state.providerConsentStatus === 'error') {
    input.checked = store.state.providerConsent.aiAgentToolsEnabled
  }
}

async function confirmAiAgentToolsAccess(): Promise<void> {
  aiConsentArmed.value = false
  await store.updateProviderConsent({ noticeSeen: true, aiAgentToolsEnabled: true })
}

async function updateRetention(event: Event): Promise<void> {
  const value = event.currentTarget instanceof HTMLSelectElement ? event.currentTarget.value : 'permanent'
  await store.updateDataRetention(value === 'permanent' ? null : Number(value) as 90 | 365)
}

async function confirmClearLocalData(): Promise<void> {
  if (await store.clearLocalRecords()) deleteArmed.value = false
}

async function installUpdate(): Promise<void> {
  updateInstallArmed.value = false
  await downloadAndInstallDesktopUpdate()
}

onMounted(() => {
  void store.refreshAutostart()
  void store.refreshLocalData()
})
</script>

<template>
  <section class="page settings-page">
    <PageHeader title="设置" subtitle="启动、隐私、数据与外观" />
    <div class="settings-layout">
      <div class="settings-main">
        <section class="settings-group" aria-labelledby="startup-title">
          <header class="settings-group__header">
            <div><h2 id="startup-title">启动与窗口</h2></div>
          </header>
          <div class="settings-list">
            <label class="control-row">
              <span class="control-icon"><PhPower :size="20" aria-hidden="true" /></span>
              <div><strong>开机自启动</strong><span>登录后自动运行</span><small :class="['system-status', store.state.autostartStatus]">{{ autostartStatusLabel }}</small></div>
              <span class="toggle"><input :checked="store.state.autostartEnabled" :disabled="store.state.autostartStatus === 'loading'" type="checkbox" @change="updateAutostart"><i></i></span>
            </label>
            <label class="control-row">
              <span class="control-icon"><PhTray :size="20" aria-hidden="true" /></span>
              <div><strong>关闭主窗口时</strong><span>每次询问、隐藏到托盘或直接退出</span></div>
              <select class="control-select" aria-label="关闭主窗口时的行为" :value="store.state.closePreference" @change="updateClosePreference">
                <option value="ask">每次询问</option>
                <option value="hide">隐藏到托盘</option>
                <option value="quit">直接退出</option>
              </select>
            </label>
            <label class="control-row">
              <span class="control-icon"><PhChartBar :size="20" aria-hidden="true" /></span>
              <div><strong>活动记录</strong><span>{{ store.state.recordingMessage }}</span></div>
              <span class="toggle"><input :checked="store.state.recording" :disabled="store.state.recordingStatus === 'loading'" type="checkbox" @change="updateRecording"><i></i></span>
            </label>
          </div>
        </section>

        <section class="settings-group provider-section" aria-labelledby="provider-title">
          <header class="settings-group__header">
            <div><h2 id="provider-title">AI 工具</h2><p>统一授权；关闭后停止检测。</p></div>
          </header>
          <div class="settings-list provider-list">
            <label class="control-row">
              <span class="control-icon"><PhRobot :size="20" aria-hidden="true" /></span>
              <div><strong>AI 工具</strong><span>读取本机会话元数据</span></div>
              <span class="toggle"><input :checked="store.state.providerConsent.aiAgentToolsEnabled" :disabled="store.state.providerConsentStatus === 'loading'" type="checkbox" @change="updateAiAgentToolsAccess"><i></i></span>
            </label>
          </div>
          <div v-if="aiConsentArmed" ref="aiConsentConfirmation" class="update-confirmation" role="alert" @keydown.esc="aiConsentArmed = false">
            <PhShieldCheck :size="21" aria-hidden="true" />
            <div>
              <strong>授权前请确认读取范围</strong>
              <p>开启后 iTime 会扫描本机 AI Agent 工具目录（如 ~/.codex/sessions、~/.claude/projects、~/.grok/sessions、~/.copilot/session-state、opencode.db 等）中的会话 JSONL/数据库文件，并在 PATH 中检测工具是否安装。这些文件可能包含提示词与代码内容，但 iTime 仅提取会话 ID、时间戳与事件类型等元数据，字段级丢弃其余内容；所有数据仅保存在本机、不外发。关闭开关即停止扫描。</p>
            </div>
            <button type="button" @click="aiConsentArmed = false">取消</button>
            <button class="confirm-update" type="button" @click="confirmAiAgentToolsAccess">我已了解，开启</button>
          </div>
          <div :class="['source-status', 'provider-source-status', store.state.providerDataStatus]">
            <span class="status-dot"></span><div><strong>{{ providerStatusLabel }}</strong><p>{{ store.state.providerDataMessage }}</p></div>
            <button v-if="providerEnabled" type="button" :disabled="store.state.providerDataStatus === 'loading'" @click="store.refreshProviderData"><PhArrowClockwise :size="16" aria-hidden="true" />刷新</button>
          </div>
        </section>

        <section class="settings-group update-section" aria-labelledby="update-title">
          <header class="settings-group__header">
            <div><h2 id="update-title">软件更新</h2><p>每日最多静默检查一次。</p></div>
          </header>
          <div :class="['source-status', 'update-status', updateState.status]" role="status" aria-live="polite">
            <span class="status-dot"></span>
            <div>
              <strong>{{ updateStatusLabel }}</strong>
              <p v-if="updateState.status === 'available'">当前 {{ updateState.currentVersion }} · {{ formatUpdateDate(updateState.date) }}<template v-if="updateState.sizeBytes"> · {{ formatBytes(updateState.sizeBytes) }}</template></p>
              <p v-else-if="updateState.status === 'downloading'">{{ updateProgress === null ? '下载中' : `已下载 ${updateProgress}%` }}</p>
              <!-- F-E1: 安装窗口内记录由后端暂停以静默写盘，明确告知而非看似无响应。 -->
              <p v-else-if="updateState.status === 'installing'">正在安装，期间记录暂停，完成后自动重启…</p>
              <p v-else-if="updateState.status === 'failed'">{{ updateState.error }}</p>
              <p v-else>版本 {{ updateState.currentVersion || '读取中' }}</p>
            </div>
            <button type="button" :disabled="!desktopControlsAvailable || updateBusy" @click="checkForDesktopUpdate(true)"><PhArrowClockwise :size="16" aria-hidden="true" />检查更新</button>
          </div>
          <progress v-if="updateState.status === 'downloading' && updateProgress !== null" class="update-progress" :value="updateProgress" max="100">{{ updateProgress }}%</progress>
          <div v-if="updateState.status === 'available'" class="update-release">
            <div><strong>iTime {{ updateState.version }}</strong><p>{{ updateState.notes || '无更新说明' }}</p></div>
            <button type="button" @click="updateInstallArmed = true"><PhUploadSimple :size="17" aria-hidden="true" />下载安装</button>
          </div>
          <div v-if="updateInstallArmed" ref="updateConfirmation" class="update-confirmation" role="alert" @keydown.esc="updateInstallArmed = false">
            <PhShieldCheck :size="21" aria-hidden="true" />
            <div><strong>安装 iTime {{ updateState.version }}？</strong><p>会先保存本地数据；失败则保留当前版本。</p></div>
            <button type="button" @click="updateInstallArmed = false">取消</button>
            <button class="confirm-update" type="button" @click="installUpdate">确认</button>
          </div>
        </section>

        <section class="settings-group local-data-section" aria-labelledby="local-data-title">
          <header class="settings-group__header">
            <div><h2 id="local-data-title">本地数据</h2><p>导出、保留期与清空</p></div>
          </header>
          <div :class="['source-status', 'local-data-status', store.state.localDataStatus]" role="status" aria-live="polite">
            <span class="status-dot"></span>
            <div><strong>{{ localDataStatusLabel }}</strong><p>{{ store.state.localDataMessage }}</p></div>
          </div>
          <dl class="local-data-facts">
            <div><dt>位置</dt><dd :title="store.state.localData.directory">{{ store.state.localData.directory }}</dd></div>
            <div><dt>范围</dt><dd>{{ localDataRange }}</dd></div>
            <div><dt>写入</dt><dd>{{ formatMoment(store.state.localData.lastWriteAt) }}</dd></div>
            <div><dt>空间</dt><dd>{{ formatBytes(store.state.localData.sizeBytes) }} · {{ store.state.localData.fileCount }} 个文件</dd></div>
            <div><dt>记录</dt><dd>{{ store.state.localData.activityRecords }} 活动 · {{ store.state.localData.keyboardRecords }} 按键</dd></div>
          </dl>
          <label class="retention-control">
            <span><strong>保留期</strong><small>默认永久；仅清理已关闭的过期分片</small></span>
            <select :value="retentionValue" :disabled="!desktopControlsAvailable || localDataBusy" @change="updateRetention">
              <option value="permanent">永久</option>
              <option value="365">365 天</option>
              <option value="90">90 天</option>
            </select>
          </label>
          <div class="local-data-actions">
            <button type="button" :disabled="!desktopControlsAvailable || localDataBusy" @click="store.openLocalData"><PhFolderOpen :size="17" aria-hidden="true" />打开目录</button>
            <button type="button" :disabled="!desktopControlsAvailable || localDataBusy" @click="store.exportLocalRecords('json')"><PhDownloadSimple :size="17" aria-hidden="true" />导出 JSON</button>
            <button type="button" :disabled="!desktopControlsAvailable || localDataBusy" @click="store.exportLocalRecords('csv')"><PhDownloadSimple :size="17" aria-hidden="true" />导出 CSV</button>
            <button class="danger-action" type="button" :disabled="!desktopControlsAvailable || localDataBusy" @click="deleteArmed = true"><PhTrash :size="17" aria-hidden="true" />删除全部</button>
          </div>
          <p v-if="store.state.localDataExportMessage" class="export-result">{{ store.state.localDataExportMessage }}</p>
          <div v-if="deleteArmed" ref="deleteConfirmation" class="delete-confirmation" role="alert" @keydown.esc="deleteArmed = false">
            <PhTrash :size="21" aria-hidden="true" />
            <div><strong>删除全部本地记录？</strong><p>不可撤销。将删除全部活动/键盘记录、图标缓存、恢复数据并撤销 AI 工具授权；已导出的文件与界面偏好设置保留。</p></div>
            <button type="button" :disabled="localDataBusy" @click="deleteArmed = false">取消</button>
            <button class="confirm-delete" type="button" :disabled="localDataBusy" @click="confirmClearLocalData">确认删除</button>
          </div>
        </section>

        <section class="settings-group appearance-section" aria-labelledby="appearance-title">
          <header class="settings-group__header">
            <div><h2 id="appearance-title">外观</h2></div>
          </header>
          <div class="theme-options" role="radiogroup" aria-label="主题">
            <span class="theme-icon"><PhPalette :size="20" aria-hidden="true" /><i><strong>主题</strong></i></span>
            <label :class="{ active: store.state.theme === 'light' }"><input v-model="store.state.theme" type="radio" value="light"><PhSun :size="18" weight="regular" aria-hidden="true" /><span>浅色</span></label>
            <label :class="{ active: store.state.theme === 'dark' }"><input v-model="store.state.theme" type="radio" value="dark"><PhMoon :size="18" weight="regular" aria-hidden="true" /><span>深色</span></label>
            <label :class="{ active: store.state.theme === 'system' }"><input v-model="store.state.theme" type="radio" value="system"><PhDesktop :size="18" weight="regular" aria-hidden="true" /><span>跟随系统</span></label>
          </div>
        </section>
      </div>

      <aside class="settings-side">
        <section class="source-panel" aria-labelledby="source-title">
          <header class="settings-group__header">
            <div><h2 id="source-title">键盘统计</h2></div>
          </header>
          <div :class="['source-status', store.state.inputDataStatus]">
            <span class="status-dot"></span><div><strong>{{ inputStatusLabel }}</strong><p>{{ store.state.inputDataMessage }}</p></div>
          </div>
          <h3 class="source-facts__title">概览</h3>
          <dl class="source-facts">
            <div v-for="fact in inputFacts" :key="fact.label"><component :is="fact.icon" :size="19" aria-hidden="true" /><dt>{{ fact.label }}</dt><dd>{{ fact.value }}</dd></div>
          </dl>
          <button class="refresh-button" type="button" :disabled="store.state.inputDataStatus === 'loading'" @click="store.refreshInputData">
            <PhArrowClockwise :size="17" weight="regular" aria-hidden="true" />刷新
          </button>
        </section>
        <section class="source-panel shortcuts-panel" aria-labelledby="shortcuts-title">
          <header class="settings-group__header">
            <div><h2 id="shortcuts-title">快捷键</h2><p>在页面空白处生效</p></div>
          </header>
          <ul class="shortcut-list">
            <li><span class="shortcut-list__keys"><kbd>Ctrl</kbd><kbd>PgUp</kbd>/<kbd>PgDn</kbd></span><span>上一个 / 下一个页面</span></li>
            <li><span class="shortcut-list__keys"><kbd>Alt</kbd><kbd>←</kbd>/<kbd>→</kbd></span><span>后退 / 前进</span></li>
            <li><span class="shortcut-list__keys"><kbd>←</kbd>/<kbd>→</kbd></span><span>空白处直接翻页</span></li>
            <li><span class="shortcut-list__keys"><kbd>Esc</kbd></span><span>关闭弹层与抽屉</span></li>
          </ul>
        </section>
      </aside>
    </div>
  </section>
</template>

<style scoped src="./settings-page.css"></style>
