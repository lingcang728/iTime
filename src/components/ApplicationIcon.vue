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
    processId?: number
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
    :process-id="processId"
    :size="size"
  />
</template>
