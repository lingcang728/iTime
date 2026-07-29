<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
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
  store.state.providerConsent.aiAgentToolsEnabled
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
const updateBusy = computed(() => ['checking', 'downloading', 'installing'].includes(updateState.status))
const updateProgress = computed(() => {
  if (!updateState.totalBytes) return null
  return Math.min(100, Math.round(updateState.downloadedBytes / updateState.totalBytes * 100))
})
const updateStatusLabel = computed(() => ({
  idle: '尚未检查',
  checking: '正在检查',
  available: `发现 ${updateState.version}`,
  downloading: '正在下载',
  installing: '正在安装',
  failed: '更新失败',
  upToDate: '已是最新版本',
}[updateState.status]))
const retentionValue = computed(() => store.state.localData.retentionDays?.toString() ?? 'permanent')

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function formatUpdateDate(value: string): string {
  if (!value) return '未提供发布时间'
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).format(date)
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

async function updateAiAgentToolsAccess(event: Event): Promise<void> {
  await store.updateProviderConsent({
    noticeSeen: true,
    aiAgentToolsEnabled: checkedValue(event),
  })
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

        <section class="settings-group provider-section" aria-labelledby="provider-title">
          <header class="settings-group__header">
            <div><h2 id="provider-title">AI Agent 编程工具</h2><p>一个开关统一授权所有受支持工具；关闭后停止目录检测、匿名设备上报并清空待上传队列。</p></div>
          </header>
          <div class="settings-list provider-list">
            <label class="control-row">
              <span class="control-icon"><PhRobot :size="20" /></span>
              <div><strong>AI Agent 编程工具</strong><span>授权读取八类工具的本机会话结构元数据，并发送匿名硬件、性能与每日工具汇总。</span></div>
              <span class="toggle"><input :checked="store.state.providerConsent.aiAgentToolsEnabled" :disabled="store.state.providerConsentStatus === 'loading'" type="checkbox" @change="updateAiAgentToolsAccess"><i></i></span>
            </label>
          </div>
          <div :class="['source-status', 'provider-source-status', store.state.providerDataStatus]">
            <span class="status-dot"></span><div><strong>{{ providerStatusLabel }}</strong><p>{{ store.state.providerDataMessage }}</p></div>
            <button v-if="providerEnabled" type="button" :disabled="store.state.providerDataStatus === 'loading'" @click="store.refreshProviderData"><PhArrowClockwise :size="16" />刷新</button>
          </div>
        </section>

        <section class="settings-group update-section" aria-labelledby="update-title">
          <header class="settings-group__header">
            <div><h2 id="update-title">软件更新</h2><p>从 iTime 的 GitHub Release 检查签名安装包；启动时每天最多静默检查一次。</p></div>
          </header>
          <div :class="['source-status', 'update-status', updateState.status]" role="status" aria-live="polite">
            <span class="status-dot"></span>
            <div>
              <strong>{{ updateStatusLabel }}</strong>
              <p v-if="updateState.status === 'available'">当前 {{ updateState.currentVersion }} · {{ formatUpdateDate(updateState.date) }}<template v-if="updateState.sizeBytes"> · {{ formatBytes(updateState.sizeBytes) }}</template></p>
              <p v-else-if="updateState.status === 'downloading'">{{ updateProgress === null ? '正在接收签名安装包' : `已下载 ${updateProgress}%` }}</p>
              <p v-else-if="updateState.status === 'installing'">本地数据已安全落盘，正在交给 Windows 安装器。</p>
              <p v-else-if="updateState.status === 'failed'">{{ updateState.error }}</p>
              <p v-else>当前版本 {{ updateState.currentVersion || '读取中' }}</p>
            </div>
            <button type="button" :disabled="!desktopControlsAvailable || updateBusy" @click="checkForDesktopUpdate(true)"><PhArrowClockwise :size="16" />检查更新</button>
          </div>
          <progress v-if="updateState.status === 'downloading' && updateProgress !== null" class="update-progress" :value="updateProgress" max="100">{{ updateProgress }}%</progress>
          <div v-if="updateState.status === 'available'" class="update-release">
            <div><strong>iTime {{ updateState.version }}</strong><p>{{ updateState.notes || '此版本未提供 Release Notes。' }}</p></div>
            <button type="button" @click="updateInstallArmed = true"><PhUploadSimple :size="17" />下载并安装</button>
          </div>
          <div v-if="updateInstallArmed" class="update-confirmation" role="alert">
            <PhShieldCheck :size="21" aria-hidden="true" />
            <div><strong>确认下载并安装 iTime {{ updateState.version }}？</strong><p>iTime 会先暂停采集、刷新本地数据并保存待上传队列；失败时会恢复当前版本。</p></div>
            <button type="button" @click="updateInstallArmed = false">取消</button>
            <button class="confirm-update" type="button" @click="installUpdate">确认更新</button>
          </div>
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
      </aside>
    </div>
  </section>
</template>

<style scoped src="./settings-page.css"></style>
