<script lang="ts" setup>
import { computed, onMounted, ref, useTemplateRef } from "vue";
import { isEventForHotkey as isEventForAccelerator, isVScrollable } from "../utils";
import { watchDebounced } from "@vueuse/core";
import { ResultItem, Action } from "../result_item";
import ResultItemComponent from "../components/ResultItem.vue";
import SearchBox from "../components/SearchBox.vue";
import Divider from "../components/Divider.vue";
import { IpcCommand, IpcEvent } from "../ipc";
import { useConfig } from "../composables/config";
import { useSystemAccentColors } from "../composables/systemAccentColor";
import { useCustomCSS } from "../composables/customCss";
import { useHead } from "@unhead/vue";

const config = useConfig();

const customCSS = useCustomCSS();
useHead({
  style: () => [
    {
      innerHTML: customCSS.value,
    },
  ],
});

const systemAccentColors = useSystemAccentColors();
const accentColor = computed(() => systemAccentColors.value.accent_light2 ?? "#02a9ea#");

const inputRef = useTemplateRef("input-ref");
onMounted(() =>
  window.KAL.ipc.on(IpcEvent.FocusInput, () => {
    inputRef?.value?.focus();
    inputRef?.value?.select();
  }),
);

const resultItemRefs = useTemplateRef("result-item-refs");
const itemsContainerRef = useTemplateRef<HTMLElement>("items-container-ref");

const results = ref<ResultItem[]>([]);

const reloading = ref(false);

const currentQuery = ref("");
const currentSelection = ref(0);
const currentSelectedItem = computed(() => results.value[currentSelection.value]);
const currentSelectedAction = ref(0);

watchDebounced(
  currentQuery,
  (query) => {
    query ? runQuery(query) : resetQuery();
  },
  { debounce: 50 },
);

async function runQuery(query: string) {
  const response: ResultItem[] = await window.KAL.ipc.invoke(IpcCommand.Query, query);
  resetSelection();
  results.value = response;
}

function resetSelection() {
  currentSelection.value = 0;
  currentSelectedAction.value = 0;
}

async function resetQuery() {
  resetSelection();
  results.value = [];
  await window.KAL.ipc.invoke(IpcCommand.ClearResults);
}

async function hideMainWindow() {
  await window.KAL.ipc.invoke(IpcCommand.HideMainWindow);
}

async function runAction(action: Action) {
  const payload = `${action.id}#${currentSelectedItem.value.id}`;
  await window.KAL.ipc.invoke(IpcCommand.RunAction, payload);
}

function selectNextItem() {
  const current = currentSelection.value;
  currentSelection.value = current === results.value.length - 1 ? 0 : current + 1;
  currentSelectedAction.value = 0;
}

function selectPrevItem() {
  const current = currentSelection.value;
  currentSelection.value = current === 0 ? results.value.length - 1 : current - 1;
  currentSelectedAction.value = 0;
}

function updateSelection(e: KeyboardEvent) {
  if (e.key === "ArrowDown") {
    selectNextItem();
  }

  if (e.key === "ArrowUp") {
    selectPrevItem();
  }

  if (e.key === "Tab" && !e.shiftKey) {
    if (!config.value.general.tabThroughContextButtons) {
      selectNextItem();
    } else if (currentSelectedAction.value == currentSelectedItem.value.actions.length - 1) {
      selectNextItem();
    } else {
      currentSelectedAction.value = currentSelectedAction.value + 1;
    }
  }

  if (e.key === "Tab" && e.shiftKey) {
    if (!config.value.general.tabThroughContextButtons) {
      selectPrevItem();
    } else if (currentSelectedAction.value == 0) {
      selectPrevItem();
      currentSelectedAction.value = currentSelectedItem.value.actions.length - 1;
    } else {
      currentSelectedAction.value = currentSelectedAction.value - 1;
    }
  }
}

function scrollSelected() {
  // avoid scrolling if container is not scrollable atm
  if (!isVScrollable(itemsContainerRef.value)) return;

  const current = currentSelection.value;
  const block = current === 0 ? "end" : current === results.value.length - 1 ? "start" : "nearest";
  resultItemRefs.value?.[current]?.$el.scrollIntoView({
    behavior: "instant",
    block,
  });
}

async function reload() {
  reloading.value = true;
  await window.KAL.ipc.invoke(IpcCommand.Reload);
  reloading.value = false;
  runQuery(currentQuery.value);
}

async function onInputKeyDown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    e.preventDefault();
    await hideMainWindow();
  }

  if (e.ctrlKey && e.key === "r") {
    e.preventDefault();
    await reload();
  }

  if (results.value.length === 0) return;

  if (e.key === "Enter" && currentSelectedAction.value > 0) {
    e.preventDefault();
    const action = currentSelectedItem.value.actions[currentSelectedAction.value];
    await runAction(action);
    return;
  }

  for (const action of currentSelectedItem.value.actions) {
    if (action.accelerator && isEventForAccelerator(e, action.accelerator)) {
      e.preventDefault();
      await runAction(action);
      return;
    }
  }

  if (["ArrowDown", "Tab", "ArrowUp"].includes(e.key)) {
    e.preventDefault();
    updateSelection(e);
    scrollSelected();
  }
}

const isTransparent = computed(() => config.value.appearance.transparent);
const bgPrimaryColor = computed(() =>
  isTransparent.value ? "bg-transparent" : "bg-[rgba(21,_20,_20,_0.75)]",
);
const inputHeight = computed(() => `${config.value.appearance.inputHeight}px`);
const itemHeight = computed(() => `${config.value.appearance.itemHeight}px`);
const itemsContainerHeight = computed(() => `calc(100% - ${inputHeight.value})`);
</script>

<template>
  <main class="w-full h-full" :class="bgPrimaryColor">
    <SearchBox
      ref="input-ref"
      :inputHeight
      :reloading
      placeholder="Start typing..."
      :style="{ height: inputHeight }"
      v-model="currentQuery"
      @keydown="onInputKeyDown"
    >
    </SearchBox>

    <Divider class="bg-[var(--divider)]"></Divider>

    <ul
      ref="items-container-ref"
      :style="{ height: itemsContainerHeight }"
      class="overflow-x-hidden overflow-y-auto p-4 children:mb-1"
      tabindex="-1"
    >
      <ResultItemComponent
        :style="{ height: itemHeight }"
        v-for="(item, index) in results"
        ref="result-item-refs"
        :item
        :selected="index === currentSelection"
        :selectedActionIndex="currentSelectedAction"
      />
    </ul>
  </main>
</template>

<style>
main {
  --accent: v-bind(accentColor);
  --text-primary: #ffffff;
  --text-secondary: #cbcbcb;
  --divider: #3d3d3d;

  color: var(--text-primary);
}
</style>
