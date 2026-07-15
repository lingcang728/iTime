<script setup lang="ts">
import { computed } from 'vue'
import { PhAppWindow, PhFolderOpen } from '@phosphor-icons/vue'
import { embeddedAppIcons } from '../data/appIconAssets'
import chromeIcon from '../assets/apps/chrome.svg'
import claudeIcon from '../assets/apps/claude.svg'
import vscodeIcon from '../assets/apps/vscode.svg'

const props = withDefaults(defineProps<{ iconKey: string; size?: number }>(), { size: 20 })
const localIcons: Record<string, string> = {
  claude: claudeIcon,
  vscode: vscodeIcon,
  chrome: chromeIcon,
}
const imageSource = computed(() => localIcons[props.iconKey] ?? embeddedAppIcons[props.iconKey])
</script>

<template>
  <span class="application-icon" :style="{ width: `${size}px`, height: `${size}px` }" aria-hidden="true">
    <img v-if="imageSource" :src="imageSource" alt="" />
    <PhFolderOpen v-else-if="iconKey === 'explorer'" :size="size" weight="duotone" />
    <PhAppWindow v-else :size="size" weight="duotone" />
  </span>
</template>
