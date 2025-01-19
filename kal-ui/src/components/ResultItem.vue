<script setup lang="ts">
import { IpcCommand } from "../ipc";
import { Action, ResultItem } from "../result_item";
import { makeIconHTML } from "../utils";
import { neutralForegroundHover } from "@fluentui/web-components";

const props = defineProps<{
  item: ResultItem;
  selected: boolean;
  itemHeight: string;
}>();

async function runAction(action: Action) {
  const payload = `${action.id}#${props.item.id}`;
  await window.KAL.ipc.invoke(IpcCommand.RunAction, payload);
}

const hoverBgColor = neutralForegroundHover.getValueFor(document.documentElement).toColorString();
const hoverBgColor10Percent = `${hoverBgColor}1A`;
</script>

<template>
  <fluent-option
    class="w-full part:content:flex part:content:w-full last:children:hover:flex bg-transparent before:left-0"
    :class="{ 'selected before:bg-[var(--accent-fill-rest)]': selected }"
    :style="{ height: itemHeight }"
    @click="runAction(item.actions[0])"
  >
    <div
      :style="{ width: itemHeight }"
      class="flex justify-center items-center children:w-50% children:h-50%"
      v-html="makeIconHTML(item.icon)"
    ></div>

    <div class="flex-1 flex flex-col justify-center overflow-hidden children:text-ellipsis">
      <span class="text-1rem">
        {{ item.primary_text }}
      </span>
      <span class="text-[var(--neutral-fill-strong-hover)] text-xs">
        {{ item.secondary_text }}
      </span>
    </div>

    <div
      v-if="item.actions"
      class="justify-center items-center mr-2 gap-2 hidden"
      :class="{ flex: selected }"
    >
      <button
        class="py-2 px-2 bg-transparent outline-none b-solid b-1px b-transparent hover:bg-white/10 hover:b-white/20 rounded"
        v-for="action in item.actions.slice(1)"
        :title="`${action.description} (${action.accelerator})`"
        @click.stop="runAction(action)"
      >
        <div v-if="action.icon" class="h-4 w-4" v-html="makeIconHTML(action.icon)"></div>
      </button>
    </div>
  </fluent-option>
</template>

<style scoped>
* {
  --base-height-multiplier: 12;
}

ul fluent-option::part(content) {
  height: v-bind(itemHeight);
}

ul fluent-option:hover,
ul fluent-option.selected {
  background-color: v-bind(hoverBgColor10Percent);
}
</style>
