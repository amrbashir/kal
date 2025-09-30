<script setup lang="ts">
import { useTemplateRef } from "vue";

defineProps<{ placeholder?: string; reloading?: boolean }>();
defineEmits(["keydown"]);
defineExpose({ focus, select });

const inputRef = useTemplateRef("input-ref");

function focus() {
  inputRef?.value?.focus();
}

function select() {
  inputRef?.value?.select();
}

const model = defineModel();
</script>

<template>
  <div class="flex items-center px-4 text-size-base">
    <div class="h-full w-10% flex items-center justify-center">
      <span class="i-builtin-search block h-35% aspect-ratio-square"></span>
    </div>
    <input
      ref="input-ref"
      class="outline-none border-none bg-transparent flex-1 text-size-inherit color-inherit"
      :placeholder
      v-model="model"
      @keydown="$emit('keydown', $event)"
    />

    <Transition name="fade">
      <!-- TODO: use windows 11 spinner -->
      <span v-if="reloading" class="i-builtin-Progress animate-spin mr-4" />
    </Transition>
  </div>
</template>
