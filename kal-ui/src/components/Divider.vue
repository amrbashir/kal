<script setup lang="ts">
const { horizontal = true, vertical } = defineProps<{
  horizontal?: boolean;
  vertical?: boolean;
}>();
</script>

<template>
  <div
    :class="{
      'h-1px w-full': horizontal,
      'w-1px h-full': vertical,
    }"
    class="bg-[var(--divider)]"
  ></div>
</template>
