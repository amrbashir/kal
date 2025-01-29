<script setup lang="ts">
import { runAction } from "../ipc";
import { Action } from "../result_item";
import { makeIconHTML } from "../utils";

const props = defineProps<{ itemId: string; action: Action; selected: boolean }>();
</script>

<template>
  <button
    class="py-2 px-2 bg-transparent outline-none rounded b-solid b-0.1rem b-transparent hover:bg-white/10 hover:b-white/20"
    :title="`${action.description} (${action.accelerator})`"
    @click.stop="runAction(action, itemId)"
    :class="{ 'bg-white/10 b-white/20': selected }"
  >
    <div v-if="action.icon" class="h-4 w-4" v-html="makeIconHTML(action.icon)" />
  </button>
</template>
