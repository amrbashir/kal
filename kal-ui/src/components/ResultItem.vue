<script setup lang="ts">
import { runAction } from "../ipc";
import { ResultItem } from "../result_item";
import { makeIconHTML } from "../utils";
import ResultItemAction from "./ResultItemAction.vue";

defineProps<{
  item: ResultItem;
  selected: boolean;
  selectedActionIndex: number;
}>();
</script>

<template>
  <div
    class="bg-transparent w-full flex last:children:hover:flex hover:bg-white/5 rd-1 relative"
    :class="{
      'bg-white/5 before:content-[\'\'] before:w-3px before:bg-[var(--accent)] before:rd-1':
        selected,
      'before:absolute before:h-40% before:translate-y--50% before:top-50%': selected,
    }"
    :aria-selected="selected"
    @click="runAction(item.actions[0], item.id)"
    :title="item.tooltip ? item.tooltip : `${item.primary_text}\n${item.secondary_text}`"
  >
    <div
      class="flex w-10% justify-center items-center children:h-50% children:aspect-ratio-square"
      v-html="makeIconHTML(item.icon)"
    />

    <div class="flex-1 grid grid-rows-2 children:text-ellipsis">
      <span class="text-size-base">
        {{ item.primary_text }}
      </span>
      <span class="text-[var(--text-secondary)] text-xs">
        {{ item.secondary_text }}
      </span>
    </div>

    <ul
      v-if="item.actions"
      class="justify-center items-center mr-2 gap-2 hidden"
      :class="{ flex: selected }"
    >
      <ResultItemAction
        v-for="(action, index) in item.actions.slice(1)"
        :action
        :itemId="item.id"
        :selected="selected && selectedActionIndex == index + 1"
      />
    </ul>
  </div>
</template>
