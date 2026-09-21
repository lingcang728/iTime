<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { PhAppWindow } from '@phosphor-icons/vue'
import {
  buildAppIdentity,
  canonicalAppKey,
  iconResolverIdentity,
  identityAccent,
  identityGlyph,
} from '../domain/appIdentity'
import { embeddedAppIcons } from '../data/appIconAssets'
import chromeIcon from '../assets/apps/chrome.svg'
import claudeIcon from '../assets/apps/claude.svg'
import explorerIcon from '../assets/apps/explorer.svg'
import grokIcon from '../assets/apps/grok.svg'
import vscodeIcon from '../assets/apps/vscode.svg'
import itimeIcon from '../assets/logo.svg'
import {
  peekAppIcon,
  resolveAppIcon,
  sameIconIdentity,
  subscribeAppIconHints,
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

/** Local brand assets — take priority over flaky native extraction for known apps/CLIs. */
const localIcons: Record<string, string> = {
  claude: claudeIcon,
  chrome: chromeIcon,
  explorer: explorerIcon,
  grok: grokIcon,
  itime: itimeIcon,
  vscode: vscodeIcon,
}

const status = ref<IconStatus>('loading')
const nativeUrl = ref<string | null>(null)
const imageBroken = ref(false)

const displayKey = computed(() => canonicalAppKey(props.appName)
  ?? canonicalAppKey(props.appIdentity)
  ?? canonicalAppKey(props.iconKey)
  ?? (props.appIdentity ?? props.iconKey ?? '').replace(/^app:/, '').toLowerCase())
const resolverIdentity = computed(() => iconResolverIdentity({
  appIdentity: props.appIdentity,
  iconKey: props.iconKey,
  appName: props.appName,
  executablePath: props.executablePath,
  aumid: props.aumid,
  packageFullName: props.packageFullName,
  packageFamilyName: props.packageFamilyName,
  siteHost: props.siteHost,
  processId: props.processId,
}))
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
const requestedNativeSize = computed(() =>
  Math.min(256, Math.max(32, Math.round(props.size * 2))),
)

const embeddedSource = computed(
  () => localIcons[displayKey.value] ?? embeddedAppIcons[displayKey.value] ?? null,
)
/** Catalog brand icons win: CLI agents rarely ship a real .exe icon. */
const preferEmbedded = computed(() => Boolean(embeddedSource.value))
const accent = computed(() => {
  store.themeRevision.value
  return identityAccent(identityInfo.value.identity)
})
const glyph = computed(() => identityGlyph(props.appName, identityInfo.value.identity))

const displayUrl = computed(() => {
  if (imageBroken.value) return null
  if (preferEmbedded.value) return embeddedSource.value
  return nativeUrl.value ?? embeddedSource.value
})

const showImage = computed(() => Boolean(displayUrl.value) && !imageBroken.value)
const showGlyph = computed(() => !showImage.value)
const ariaLabel = computed(() => props.appName || props.appIdentity || props.iconKey || 'app')

let unsubscribe: (() => void) | undefined
let unsubscribeHints: (() => void) | undefined
let mounted = false
/** Backend-canonical identity learned from the last native resolve (may differ from identityInfo). */
const nativeIdentity = ref<string | null>(null)

async function refresh(): Promise<void> {
  imageBroken.value = false
  // A catalog brand icon wins over native extraction — skip the IPC entirely.
  if (preferEmbedded.value) {
    status.value = 'resolved'
    nativeUrl.value = null
    return
  }
  const identity = identityInfo.value.identity
  const requestedSize = requestedNativeSize.value
  // Forget the previously learned canonical identity — it belongs to the
  // props that produced it, not necessarily to this request.
  nativeIdentity.value = null
  const peeked = peekAppIcon(identity, requestedSize)
  if (peeked) {
    nativeIdentity.value = peeked.appIdentity
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
    requestedSize,
    processId: props.processId,
  })

  // Staleness is judged against what this instance asked for, not against
  // result.appIdentity — the backend-normalized identity is authoritative and
  // may legitimately differ (separator collapsing, canonicalized exe paths).
  if (
    !mounted
    || identityInfo.value.identity !== identity
    || requestedNativeSize.value !== requestedSize
  ) return
  nativeIdentity.value = result.appIdentity
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
    if (
      !sameIconIdentity(result.appIdentity, identityInfo.value.identity)
      && !sameIconIdentity(result.appIdentity, nativeIdentity.value)
    ) return
    if (result.width !== requestedNativeSize.value) return
    nativeIdentity.value = result.appIdentity
    status.value = result.status
    nativeUrl.value = result.iconUrl ?? null
    if (result.status === 'resolved') imageBroken.value = false
  })
  // Single shared native listener lives in the service; it already cleared the
  // matching cache entries before this fires.
  unsubscribeHints = subscribeAppIconHints((identity) => {
    if (
      !sameIconIdentity(identity, identityInfo.value.identity)
      && !sameIconIdentity(identity, nativeIdentity.value)
    ) return
    void refresh()
  })
  void refresh()
})

onUnmounted(() => {
  mounted = false
  unsubscribe?.()
  unsubscribeHints?.()
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
    <PhAppWindow v-else class="app-icon__generic" :size="Math.max(12, size - 2)" weight="duotone" aria-hidden="true" />
  </span>
</template>
