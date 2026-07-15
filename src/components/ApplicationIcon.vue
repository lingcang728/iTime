<script setup lang="ts">
import { computed } from 'vue'
import { PhAppWindow, PhFolderOpen } from '@phosphor-icons/vue'
import { embeddedAppIcons } from '../data/appIconAssets'

const props = withDefaults(defineProps<{ iconKey: string; size?: number }>(), { size: 20 })
const localIcons: Record<string, string> = {
  claude: '/src/assets/apps/claude.svg',
  vscode: '/src/assets/apps/vscode.svg',
  chrome: '/src/assets/apps/chrome.svg',
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
