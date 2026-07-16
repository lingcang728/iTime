<script setup lang="ts">
/**
 * Back-compat wrapper. Prefer AppIcon for new call sites.
 */
import AppIcon from './AppIcon.vue'

withDefaults(
  defineProps<{
    iconKey?: string
    appIdentity?: string
    appName?: string
    executablePath?: string
    aumid?: string
    size?: number
  }>(),
  { size: 20 },
)
</script>

<template>
  <AppIcon
    :icon-key="iconKey"
    :app-identity="appIdentity ?? iconKey"
    :app-name="appName"
    :executable-path="executablePath"
    :aumid="aumid"
    :size="size"
  />
</template>
