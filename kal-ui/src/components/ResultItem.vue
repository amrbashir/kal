<script setup lang="ts">
import { ResultItem } from "../result_item";
import { makeIconHTML } from "../utils";
import { neutralForegroundHover } from "@fluentui/web-components";

defineProps<{
  item: ResultItem;
  selected: boolean;
  showConfirm: boolean;
  itemHeight: string;
}>();

const hoverBgColor = neutralForegroundHover.getValueFor(document.documentElement).toColorString();
const hoverBgColor10Percent = `${hoverBgColor}1A`;
</script>

<template>
  <fluent-option
    class="w-full part:content:flex part:content:w-full"
    :class="{ selected }"
    :style="{ height: itemHeight }"
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

    <div class="flex justify-center items-center mr-2">
      <Transition name="slide-fade">
        <span class="text-[var(--accent-fill-rest)]" v-if="showConfirm"> Proceed? </span>
      </Transition>
    </div>
  </fluent-option>
</template>

<style scoped>
* {
  --base-height-multiplier: 12;
}
fluent-option::before {
  width: 3px;
  left: 0;
}

ul fluent-option::part(content) {
  height: v-bind(itemHeight);
}

ul fluent-option {
  background-color: transparent;
  outline: none;
}

ul fluent-option:hover,
ul fluent-option.selected {
  background: v-bind(hoverBgColor10Percent);
}

ul fluent-option.selected::before {
  background: var(--accent-fill-rest);
}
</style>
