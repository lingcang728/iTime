<script setup lang="ts">
withDefaults(defineProps<{ size?: number }>(), { size: 28 })
</script>

<template>
  <svg
    class="provider-activity-icon"
    :width="size"
    :height="size"
    viewBox="0 0 28 28"
    fill="none"
    aria-hidden="true"
  >
    <rect x=".5" y=".5" width="27" height="27" rx="8" class="provider-activity-icon__tile" />
    <path d="M8.1 10.1A7.2 7.2 0 0 1 20 9.3" class="provider-activity-icon__orbit" />
    <path d="m18.1 7.7 2.4 1.4-1.3 2.5" class="provider-activity-icon__arrow" />
    <path d="M19.9 17.9A7.2 7.2 0 0 1 8 18.7" class="provider-activity-icon__orbit" />
    <path d="m9.9 20.3-2.4-1.4 1.3-2.5" class="provider-activity-icon__arrow" />
    <path d="m14 9.8 1.15 2.55 2.55 1.15-2.55 1.15L14 17.2l-1.15-2.55-2.55-1.15 2.55-1.15L14 9.8Z" class="provider-activity-icon__spark" />
  </svg>
</template>

<style scoped>
.provider-activity-icon {
  display: block;
  flex: 0 0 auto;
}

.provider-activity-icon__tile {
  fill: #6658d7;
  stroke: #8d7ae9;
}

.provider-activity-icon__orbit,
.provider-activity-icon__arrow {
  stroke: #f4f1ff;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.35;
}

.provider-activity-icon__spark {
  fill: #a8e17f;
}
</style>
