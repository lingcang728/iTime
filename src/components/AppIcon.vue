<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { PhAppWindow } from '@phosphor-icons/vue'
import { buildAppIdentity, canonicalAppKey, identityAccent, identityGlyph } from '../domain/appIdentity'
import { embeddedAppIcons } from '../data/appIconAssets'
import chromeIcon from '../assets/apps/chrome.svg'
import claudeIcon from '../assets/apps/claude.svg'
import vscodeIcon from '../assets/apps/vscode.svg'
import {
  peekAppIcon,
  resolveAppIcon,
  subscribeAppIcons,
  type IconStatus,
} from '../services/appIconService'
import { useAppStore } from '../stores/appStore'

const props = withDefaults(
  defineProps<{
    /** Preferred stable identity (or legacy icon key). */
    appIdentity?: string
    /** @deprecated Use appIdentity */
    iconKey?: string
    appName?: string
    executablePath?: string
    aumid?: string
    packageFullName?: string
    packageFamilyName?: string
    siteHost?: string
    processId?: number
    size?: number
  }>(),
  { size: 20 },
)
const store = useAppStore()

const localIcons: Record<string, string> = {
  claude: claudeIcon,
  vscode: vscodeIcon,
  chrome: chromeIcon,
}

const status = ref<IconStatus>('loading')
const nativeUrl = ref<string | null>(null)
const imageBroken = ref(false)

const displayKey = computed(() => canonicalAppKey(props.appName)
  ?? canonicalAppKey(props.appIdentity)
  ?? canonicalAppKey(props.iconKey)
  ?? (props.appIdentity ?? props.iconKey ?? '').replace(/^app:/, '').toLowerCase())
const resolverIdentity = computed(() => {
  const hasConcreteIdentity = Boolean(props.executablePath || props.aumid || props.packageFullName || props.packageFamilyName || props.siteHost)
  if (hasConcreteIdentity) return props.appIdentity ?? props.iconKey ?? props.appName
  const opaqueIdentity = /^(?:process|exe):/i.test(props.appIdentity ?? '')
  return displayKey.value || (opaqueIdentity ? props.appName : null) || props.appIdentity || props.iconKey || props.appName
})
const identityInfo = computed(() =>
  buildAppIdentity({
    appIdentity: resolverIdentity.value,
    iconKey: props.iconKey,
    executablePath: props.executablePath,
    aumid: props.aumid,
    packageFullName: props.packageFullName,
    packageFamilyName: props.packageFamilyName,
    siteHost: props.siteHost,
    appName: props.appName,
  }),
)

const embeddedSource = computed(
  () => localIcons[displayKey.value] ?? embeddedAppIcons[displayKey.value] ?? null,
)
const lockedBrandAsset = computed(() => ['codex', 'typeless'].includes(displayKey.value) ? embeddedSource.value : null)

const accent = computed(() => {
  store.themeRevision.value
  return identityAccent(identityInfo.value.identity)
})
const glyph = computed(() => identityGlyph(props.appName, identityInfo.value.identity))

const displayUrl = computed(() => {
  if (imageBroken.value) return null
  return lockedBrandAsset.value ?? nativeUrl.value ?? embeddedSource.value
})

const showImage = computed(() => Boolean(displayUrl.value) && !imageBroken.value)
const showGlyph = computed(() => !showImage.value)
const ariaLabel = computed(() => props.appName || props.appIdentity || props.iconKey || 'app')

let unsubscribe: (() => void) | undefined
let mounted = false

async function refresh(): Promise<void> {
  imageBroken.value = false
  const identity = identityInfo.value.identity
  const peeked = peekAppIcon(identity)
  if (peeked) {
    status.value = peeked.status
    nativeUrl.value = peeked.iconUrl ?? null
  } else {
    status.value = 'loading'
    nativeUrl.value = null
  }

  const result = await resolveAppIcon({
    appIdentity: resolverIdentity.value,
    iconKey: props.iconKey,
    appName: props.appName,
    executablePath: props.executablePath,
    aumid: props.aumid,
    packageFullName: props.packageFullName,
    packageFamilyName: props.packageFamilyName,
    siteHost: props.siteHost,
    requestedSize: Math.max(32, Math.round(props.size * 2)),
    processId: props.processId,
  })

  if (!mounted || result.appIdentity !== identityInfo.value.identity) return
  status.value = result.status
  nativeUrl.value = result.iconUrl ?? null
  if (result.status === 'failed' || result.status === 'unknown') {
    // Keep embedded icons as soft offline catalog; glyph only when nothing else works.
    if (!embeddedSource.value) imageBroken.value = false
  }
}

function onImageError(): void {
  imageBroken.value = true
  if (status.value === 'resolved') status.value = 'failed'
}

onMounted(() => {
  mounted = true
  unsubscribe = subscribeAppIcons((result) => {
    if (result.appIdentity !== identityInfo.value.identity) return
    status.value = result.status
    nativeUrl.value = result.iconUrl ?? null
    if (result.status === 'resolved') imageBroken.value = false
  })
  void refresh()
})

onUnmounted(() => {
  mounted = false
  unsubscribe?.()
})

watch(
  () => [
    props.appIdentity,
    props.iconKey,
    props.appName,
    props.executablePath,
    props.aumid,
    props.packageFullName,
    props.packageFamilyName,
    props.siteHost,
    props.size,
    props.processId,
  ],
  () => {
    void refresh()
  },
)
</script>

<template>
  <span
    class="app-icon"
    :class="[`is-${status}`, { 'is-fallback': showGlyph }]"
    :style="{ width: `${size}px`, height: `${size}px` }"
    :aria-label="ariaLabel"
    role="img"
  >
    <img
      v-if="showImage"
      class="app-icon__image"
      :src="displayUrl!"
      alt=""
      draggable="false"
      @error="onImageError"
    />
    <span
      v-else-if="appName || appIdentity || iconKey"
      class="app-icon__glyph"
      :style="{ background: accent.background, color: accent.color, fontSize: `${Math.max(10, size * 0.48)}px` }"
    >
      {{ glyph }}
    </span>
    <PhAppWindow v-else class="app-icon__generic" :size="Math.max(12, size - 2)" weight="duotone" />
  </span>
</template>
